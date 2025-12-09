#!/bin/bash
set -e

echo "Cupcake MCP Database Policy Demo Setup"
echo "========================================"

# Check if Docker is running
if ! docker info > /dev/null 2>&1; then
    echo "Docker is not running. Please start Docker and try again."
    exit 1
else
    echo "Docker is running"
fi

# Pull postgres-mcp Docker image if needed
echo "Pulling postgres-mcp Docker image..."
docker pull crystaldba/postgres-mcp || echo "Warning: Could not pull postgres-mcp image"

# Stop and remove existing container if it exists
echo "Cleaning up any existing demo database..."
docker stop cupcake-demo-db 2>/dev/null || true
docker rm cupcake-demo-db 2>/dev/null || true

# Start PostgreSQL container
echo "Starting PostgreSQL container..."
docker run -d \
    --name cupcake-demo-db \
    -e POSTGRES_USER=demo \
    -e POSTGRES_PASSWORD=demopass \
    -e POSTGRES_DB=appointments \
    -p 15432:5432 \
    postgres:latest

echo "PostgreSQL container started"

# Wait for database to be ready
echo "Waiting for database to be ready..."
for i in {1..30}; do
    if docker exec cupcake-demo-db pg_isready -U demo > /dev/null 2>&1; then
        echo "Database is ready"
        break
    fi
    echo -n "."
    sleep 1
done

# Create appointments table and seed data
echo "Creating appointments table and seeding data..."
docker exec -i cupcake-demo-db psql -U demo -d appointments << 'EOF'
CREATE TABLE IF NOT EXISTS appointments (
    id SERIAL PRIMARY KEY,
    patient_name VARCHAR(100) NOT NULL,
    appointment_time TIMESTAMP NOT NULL,
    status VARCHAR(20) DEFAULT 'scheduled',
    notes TEXT
);

-- Clear any existing data
TRUNCATE TABLE appointments RESTART IDENTITY;

-- Insert appointments
-- First appointment is 10 hours from now
INSERT INTO appointments (patient_name, appointment_time, status, notes) VALUES
    ('Sarah Johnson', NOW() + INTERVAL '10 hours', 'scheduled', 'Annual checkup'),
    ('Michael Chen', NOW() + INTERVAL '2 days', 'scheduled', 'Follow-up visit'),
    ('Emily Davis', NOW() + INTERVAL '3 days', 'scheduled', 'Consultation'),
    ('James Wilson', NOW() + INTERVAL '4 days', 'scheduled', 'Routine exam'),
    ('Maria Garcia', NOW() + INTERVAL '5 days', 'scheduled', 'Lab results review'),
    ('Robert Taylor', NOW() + INTERVAL '6 days', 'scheduled', 'Physical therapy'),
    ('Lisa Anderson', NOW() + INTERVAL '1 week', 'scheduled', 'Vaccination'),
    ('David Martinez', NOW() + INTERVAL '8 days', 'scheduled', 'Post-op checkup'),
    ('Jennifer Brown', NOW() + INTERVAL '9 days', 'scheduled', 'Allergy testing'),
    ('William Jones', NOW() + INTERVAL '10 days', 'scheduled', 'Blood work'),
    ('Patricia Miller', NOW() + INTERVAL '2 weeks', 'scheduled', 'Dental cleaning'),
    ('Richard Wilson', NOW() + INTERVAL '15 days', 'scheduled', 'Eye exam'),
    ('Susan Moore', NOW() + INTERVAL '16 days', 'scheduled', 'Hearing test'),
    ('Joseph Taylor', NOW() + INTERVAL '17 days', 'scheduled', 'Cardiology consult'),
    ('Margaret White', NOW() + INTERVAL '3 weeks', 'scheduled', 'Dermatology check'),
    ('Charles Harris', NOW() + INTERVAL '22 days', 'scheduled', 'Nutrition counseling'),
    ('Barbara Clark', NOW() + INTERVAL '23 days', 'scheduled', 'Mental health check-in'),
    ('Thomas Lewis', NOW() + INTERVAL '24 days', 'scheduled', 'Sports medicine'),
    ('Nancy Walker', NOW() + INTERVAL '25 days', 'scheduled', 'Prenatal visit'),
    ('Daniel Hall', NOW() + INTERVAL '4 weeks', 'scheduled', 'General consultation');

-- Show the first few appointments
SELECT id, patient_name, appointment_time, status
FROM appointments
ORDER BY appointment_time
LIMIT 5;
EOF

echo "Database setup complete"

# Copy the Cursor-specific appointment time check signal
echo "Installing appointment time check signal..."
cp ../../fixtures/cursor/check_appointment_time.py .cupcake/
chmod +x .cupcake/check_appointment_time.py

# Add signal configuration to rulebook.yml
echo "Configuring appointment time signal..."
uv run --with pyyaml python3 << 'EOF'
import yaml

rulebook_path = '.cupcake/rulebook.yml'
with open(rulebook_path, 'r') as f:
    rulebook = yaml.safe_load(f.read())

if rulebook is None:
    rulebook = {}

if 'signals' not in rulebook or rulebook['signals'] is None:
    rulebook['signals'] = {}

if 'actions' in rulebook and rulebook['actions'] is None:
    rulebook['actions'] = {}

rulebook['signals']['appointment_time_check'] = {
    'command': 'uv run --with psycopg2-binary python3 .cupcake/check_appointment_time.py'
}

with open(rulebook_path, 'w') as f:
    yaml.dump(rulebook, f, default_flow_style=False, sort_keys=False)

print("Signal configuration added to rulebook.yml")
EOF

# Copy the appointment policy from fixtures to Cursor policies directory
cp ../../fixtures/cursor/appointment_policy.rego .cupcake/policies/cursor/
echo "Appointment policy installed"

# Note: No manual opa build needed - cupcake eval handles compilation automatically
# including the helpers at .cupcake/helpers/

# Create .cursor/mcp.json for project-level MCP configuration
echo "Configuring Cursor MCP settings..."
mkdir -p .cursor/rules

cat > .cursor/mcp.json << 'EOF'
{
  "mcpServers": {
    "postgres": {
      "command": "docker",
      "args": [
        "run",
        "-i",
        "--rm",
        "--network", "host",
        "-e", "DATABASE_URI",
        "crystaldba/postgres-mcp",
        "--access-mode=unrestricted"
      ],
      "env": {
        "DATABASE_URI": "postgresql://demo:demopass@localhost:15432/appointments"
      }
    }
  }
}
EOF

echo "MCP postgres server configured in .cursor/mcp.json"

# Create .cursor/rules/db.mdc with database instructions
echo "Creating .cursor/rules/db.mdc with database schema..."
cat > .cursor/rules/db.mdc << 'EOF'
---
alwaysApply: true
---

# Database Access

PostgreSQL database available via MCP tool: `postgres_execute_sql`

## Connection
- Database: `appointments`
- Table: `appointments`

## Schema
```sql
CREATE TABLE appointments (
    id SERIAL PRIMARY KEY,
    patient_name VARCHAR(100) NOT NULL,
    appointment_time TIMESTAMP NOT NULL,
    status VARCHAR(20) DEFAULT 'scheduled',
    notes TEXT
);
```
## Sample Queries

using the tool: `postgres_execute_sql`

- List appointments: `SELECT * FROM appointments ORDER BY appointment_time`
- Find specific patient: `SELECT * FROM appointments WHERE patient_name = 'Name'`
- Update status: `UPDATE appointments SET status = 'completed' WHERE id = X`
EOF

echo ".cursor/rules/db.mdc created"

echo ""
echo "=========================================="
echo "MCP Database Demo Setup Complete!"
echo "=========================================="
echo ""
echo "Database Details:"
echo "  Host: localhost:15432"
echo "  Database: appointments"
echo "  User: demo"
echo "  Password: demopass"
echo ""
echo "IMPORTANT:"
echo "1. Restart Cursor to load the MCP configuration"
echo "2. Cursor will ask to approve the project MCP server - click 'Allow'"
echo "3. The MCP tool will appear as postgres_execute_sql"
echo ""
echo "Test Scenarios:"
echo "1. Ask to list all appointments - Should work"
echo "2. Ask to cancel appointment ID 1 - Should be blocked"
echo "   (It's scheduled within 24 hours)"
echo "3. Ask to delete old appointments - Should be blocked"
echo "   (No deletions allowed)"
echo ""
echo "Example prompts to try:"
echo '  "Show me all appointments in the database"'
echo '  "Cancel the appointment for Sarah Johnson"'
echo '  "Delete appointments older than 30 days"'
echo ""
echo "Troubleshooting:"
echo "- If MCP shows no servers, restart Cursor"
echo "- If Docker connection fails, check if postgres container is running: docker ps"
echo "- Check container logs: docker logs cupcake-demo-db"
echo ""
echo "To clean up when done: ./mcp_cleanup.sh"
