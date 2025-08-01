#!/bin/bash
# Smart database risk assessment for Cupcake policies

# Core tables that should never be dropped when they have data
CRITICAL_TABLES="film rental customer inventory payment staff store address language category city country"

# Check if specific SQL command would cause data loss
check_sql_risk() {
    local sql_command="$1"
    
    # ALWAYS flag CASCADE operations as high risk
    if echo "$sql_command" | grep -qiE "(DROP|TRUNCATE).*(CASCADE)"; then
        echo "HIGH_RISK:cascade_operation_detected"
        return 0
    fi
    
    # Check for dropping entire schemas
    if echo "$sql_command" | grep -qiE "DROP.*SCHEMA"; then
        echo "HIGH_RISK:dropping_entire_schema"
        return 0
    fi
    
    # Extract table name from DROP TABLE command
    local table_name=$(echo "$sql_command" | grep -oiE "DROP\s+TABLE\s+(\w+)" | awk '{print $3}')
    
    if [[ -n "$table_name" ]]; then
        # Check if table has foreign key relationships
        local has_fkeys=$(docker exec drizzle-test-db psql -U testuser -d drizzle_test -t -c "
            SELECT COUNT(*)
            FROM information_schema.table_constraints tc
            JOIN information_schema.referential_constraints rc 
                ON tc.constraint_name = rc.constraint_name
            WHERE tc.table_name = '$table_name' 
               OR rc.referenced_table_name = '$table_name';" 2>/dev/null | tr -d ' ')
        
        if [[ -n "$has_fkeys" ]] && [[ "$has_fkeys" -gt 0 ]]; then
            echo "HIGH_RISK:table_has_relationships:$table_name"
            return 0
        fi
        
        # Check if table has significant data
        local row_count=$(docker exec drizzle-test-db psql -U testuser -d drizzle_test -t -c "
            SELECT COUNT(*) FROM $table_name;" 2>/dev/null | tr -d ' ')
        
        if [[ -n "$row_count" ]] && [[ "$row_count" -gt 100 ]]; then
            echo "HIGH_RISK:dropping_table_with_data:$table_name:$row_count"
            return 0
        fi
    fi
    
    # Dropping an isolated table with little/no data is fine
    echo "LOW_RISK:safe_operation"
    return 1
}

# Check if database has meaningful data
check_data_exists() {
    # Count rows in critical tables only
    local total_rows=0
    for table in $CRITICAL_TABLES; do
        local count=$(docker exec drizzle-test-db psql -U testuser -d drizzle_test -t -c "
            SELECT COUNT(*) FROM $table;" 2>/dev/null | tr -d ' ')
        if [[ -n "$count" ]] && [[ "$count" =~ ^[0-9]+$ ]]; then
            total_rows=$((total_rows + count))
        fi
    done
    
    if [[ "$total_rows" -eq 0 ]]; then
        echo "EMPTY"
        return 1
    else
        echo "HAS_DATA:$total_rows"
        return 0
    fi
}

# Check if drizzle-kit would drop critical tables
check_schema_changes() {
    # Get list of tables that would be affected
    local changes=$(drizzle-kit push --dry-run 2>&1 || true)
    
    # Check if any critical tables would be dropped
    for table in $CRITICAL_TABLES; do
        if echo "$changes" | grep -qiE "DROP.*TABLE.*$table"; then
            # Check if this table has data
            local row_count=$(docker exec drizzle-test-db psql -U testuser -d drizzle_test -t -c "
                SELECT COUNT(*) FROM $table;" 2>/dev/null | tr -d ' ')
            
            if [[ -n "$row_count" ]] && [[ "$row_count" -gt 0 ]]; then
                echo "DESTRUCTIVE:$table"
                return 0
            fi
        fi
    done
    
    echo "SAFE"
    return 1
}

# Main logic
case "$1" in
    "data")
        check_data_exists
        ;;
    "schema")
        check_schema_changes
        ;;
    "sql")
        # Check specific SQL command
        check_sql_risk "$2"
        ;;
    "risk")
        # Combined risk assessment for db:push
        data_status=$(check_data_exists)
        if [[ "$data_status" == "EMPTY" ]]; then
            echo "LOW_RISK:empty_database"
            exit 0
        fi
        
        # If we have data, check if operation would be destructive
        schema_status=$(check_schema_changes)
        if [[ "$schema_status" =~ ^DESTRUCTIVE ]]; then
            echo "HIGH_RISK:would_drop_${schema_status#DESTRUCTIVE:}"
            exit 0
        else
            echo "MEDIUM_RISK:has_data_but_safe_changes"
            exit 0
        fi
        ;;
    *)
        echo "Usage: $0 {data|schema|sql <command>|risk}"
        exit 1
        ;;
esac