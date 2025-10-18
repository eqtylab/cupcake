#!/bin/bash
set -e

echo "MCP Database Demo Cleanup"
echo "========================="

# Stop and remove PostgreSQL container
if docker ps -a | grep -q cupcake-postgres; then
    echo "Stopping PostgreSQL container..."
    docker stop cupcake-postgres
    echo "Removing PostgreSQL container..."
    docker rm cupcake-postgres
    echo "✅ PostgreSQL container removed"
else
    echo "ℹ️  No PostgreSQL container found"
fi

# Remove MCP configuration
if [ -d ".mcp" ]; then
    echo "Removing MCP configuration..."
    rm -rf .mcp
    echo "✅ MCP configuration removed"
fi

# Remove appointment policy
if [ -f ".cupcake/policies/cursor/appointment_policy.rego" ]; then
    echo "Removing appointment policy..."
    rm -f .cupcake/policies/cursor/appointment_policy.rego
    echo "✅ Appointment policy removed"
fi

# Remove signal script
if [ -f ".cupcake/signals/check_appointment_time.py" ]; then
    echo "Removing appointment time check signal..."
    rm -f .cupcake/signals/check_appointment_time.py
    echo "✅ Signal script removed"
fi

echo ""
echo "🧹 MCP demo cleanup complete!"
echo ""
echo "Run ./mcp_setup.sh to set up the demo again."