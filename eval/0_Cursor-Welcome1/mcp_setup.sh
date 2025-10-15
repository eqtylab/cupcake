#!/bin/bash
set -e

echo "MCP Database Demo Setup for Cursor"
echo "==================================="

# Check if Docker is running
if ! docker info > /dev/null 2>&1; then
    echo "❌ Docker is not running. Please start Docker Desktop and try again."
    exit 1
fi

echo "✅ Docker is running"

# Stop and remove existing container if it exists
if docker ps -a | grep -q cupcake-postgres; then
    echo "Removing existing PostgreSQL container..."
    docker stop cupcake-postgres 2>/dev/null || true
    docker rm cupcake-postgres 2>/dev/null || true
fi

# Start PostgreSQL container
echo "Starting PostgreSQL container..."
docker run -d \
  --name cupcake-postgres \
  -e POSTGRES_PASSWORD=cupcake123 \
  -e POSTGRES_USER=cupcake \
  -e POSTGRES_DB=appointments \
  -p 5432:5432 \
  postgres:15-alpine

# Wait for database to be ready
echo "Waiting for database to be ready..."
for i in {1..30}; do
    if docker exec cupcake-postgres pg_isready -U cupcake > /dev/null 2>&1; then
        echo "✅ Database is ready"
        break
    fi
    if [ $i -eq 30 ]; then
        echo "❌ Database failed to start"
        exit 1
    fi
    sleep 1
done

# Create appointments table and sample data
echo "Creating appointments table..."
docker exec -i cupcake-postgres psql -U cupcake -d appointments << 'EOF'
CREATE TABLE IF NOT EXISTS appointments (
    id SERIAL PRIMARY KEY,
    patient_name VARCHAR(100) NOT NULL,
    appointment_time TIMESTAMP NOT NULL,
    status VARCHAR(20) DEFAULT 'scheduled',
    notes TEXT
);

-- Clear existing data
TRUNCATE TABLE appointments;

-- Insert sample appointments
INSERT INTO appointments (patient_name, appointment_time, status, notes) VALUES
    ('Sarah Johnson', NOW() + INTERVAL '2 hours', 'scheduled', 'Regular checkup'),
    ('Michael Chen', NOW() + INTERVAL '1 day', 'scheduled', 'Follow-up visit'),
    ('Emily Davis', NOW() + INTERVAL '3 days', 'scheduled', 'Initial consultation'),
    ('Robert Wilson', NOW() - INTERVAL '1 day', 'completed', 'Annual physical'),
    ('Lisa Anderson', NOW() - INTERVAL '7 days', 'completed', 'Vaccination'),
    ('James Taylor', NOW() + INTERVAL '12 hours', 'scheduled', 'Urgent care');

-- Show the data
SELECT * FROM appointments ORDER BY appointment_time;
EOF

echo "✅ Sample appointments created"

# Create MCP configuration for Cursor
echo "Creating MCP configuration..."
mkdir -p .mcp

cat > .mcp/config.json << 'EOF'
{
  "postgres": {
    "command": "npx",
    "args": ["-y", "@modelcontextprotocol/server-postgres", "postgresql://cupcake:cupcake123@localhost/appointments"]
  }
}
EOF

# Create MCP-specific policy for appointments
cat > .cupcake/policies/cursor/appointment_policy.rego << 'EOF'
# METADATA
# scope: package
# title: Appointment Protection Policy
# custom:
#   routing:
#     required_events: ["beforeMCPExecution"]
#     required_signals: ["check_appointment_time"]
package cupcake.policies.cursor.appointments

import rego.v1

# Block appointment cancellations within 24 hours
deny contains decision if {
    input.hook_event_name == "beforeMCPExecution"
    contains(upper(input.tool_input), "UPDATE")
    contains(lower(input.tool_input), "status")
    contains(lower(input.tool_input), "cancel")

    # Check if appointment is within 24 hours (would use signal in production)
    # For demo, block all cancellations
    decision := {
        "rule_id": "APPT-001",
        "reason": "Cannot cancel appointments within 24 hours",
        "agent_context": "Appointment cancellation blocked. This appointment is within the 24-hour window. Policy requires manual cancellation for appointments this close. Contact the patient directly or have them reschedule through the patient portal.",
        "severity": "HIGH"
    }
}

# Block all deletions on appointment data
deny contains decision if {
    input.hook_event_name == "beforeMCPExecution"
    contains(upper(input.tool_input), "DELETE")

    decision := {
        "rule_id": "APPT-002",
        "reason": "Appointment deletions not allowed",
        "agent_context": "DELETE operations are not allowed on appointment data. Use UPDATE to change status to 'cancelled' or 'completed' instead. All appointment records must be preserved for audit purposes.",
        "severity": "CRITICAL"
    }
}

# Block TRUNCATE operations
deny contains decision if {
    input.hook_event_name == "beforeMCPExecution"
    contains(upper(input.tool_input), "TRUNCATE")

    decision := {
        "rule_id": "APPT-003",
        "reason": "Mass deletion operations blocked",
        "agent_context": "TRUNCATE operation detected. Mass deletion of appointment data is strictly prohibited. Individual record updates are allowed through UPDATE statements only.",
        "severity": "CRITICAL"
    }
}

# Allow SELECT queries
allow_override contains decision if {
    input.hook_event_name == "beforeMCPExecution"
    startswith(trim_space(upper(input.tool_input)), "SELECT")

    decision := {
        "rule_id": "APPT-ALLOW-001",
        "reason": "Read operation permitted",
        "severity": "INFO"
    }
}
EOF

echo "✅ MCP appointment policy created"

# Create a Python signal script for checking appointment times (simplified for demo)
cat > .cupcake/signals/check_appointment_time.py << 'EOF'
#!/usr/bin/env python3
import json
import sys
from datetime import datetime, timedelta

# For demo purposes, this is simplified
# In production, would actually query the database

def main():
    try:
        # Read input from stdin
        event = json.load(sys.stdin)

        # Simple check: if the command mentions Sarah Johnson,
        # return that it's within 24 hours (for demo)
        if 'Sarah Johnson' in event.get('tool_input', ''):
            result = {"within_24_hours": True}
        else:
            result = {"within_24_hours": False}

        print(json.dumps(result))
    except Exception as e:
        print(json.dumps({"error": str(e)}))
        sys.exit(1)

if __name__ == "__main__":
    main()
EOF

chmod +x .cupcake/signals/check_appointment_time.py

# Add signal to rulebook
if ! grep -q "check_appointment_time" .cupcake/rulebook.yml 2>/dev/null; then
    cat >> .cupcake/rulebook.yml << 'EOF'

signals:
  check_appointment_time:
    script: .cupcake/signals/check_appointment_time.py
    timeout_seconds: 5
EOF
fi

echo ""
echo "🎉 MCP Database Demo Setup Complete!"
echo ""
echo "Database is running with sample appointments:"
echo "- Sarah Johnson: in 2 hours (cannot cancel - within 24 hours)"
echo "- Michael Chen: tomorrow (can modify)"
echo "- James Taylor: in 12 hours (cannot cancel - within 24 hours)"
echo ""
echo "Restart Cursor and try these commands:"
echo ""
echo "✅ ALLOWED: 'Show all appointments'"
echo "✅ ALLOWED: 'Find appointments for Michael Chen'"
echo "❌ BLOCKED: 'Cancel Sarah Johnson appointment'"
echo "❌ BLOCKED: 'Delete old appointments'"
echo "❌ BLOCKED: 'Clear all appointment data'"
echo ""
echo "To stop the database: ./mcp_cleanup.sh"