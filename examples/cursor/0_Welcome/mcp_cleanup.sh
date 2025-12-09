#!/bin/bash
set -e

echo "MCP Database Demo Cleanup"
echo "========================="

# Stop and remove PostgreSQL container
if docker ps -a | grep -q cupcake-demo-db; then
    echo "Stopping PostgreSQL container..."
    docker stop cupcake-demo-db
    echo "Removing PostgreSQL container..."
    docker rm cupcake-demo-db
    echo "✅ PostgreSQL container removed"
else
    echo "ℹ️  No PostgreSQL container found"
fi

# Remove MCP configuration from .cursor/mcp.json
if [ -f ".cursor/mcp.json" ]; then
    echo "Removing MCP configuration..."
    rm -f .cursor/mcp.json
    echo "✅ MCP configuration removed"
fi

# Remove appointment policy
if [ -f ".cupcake/policies/cursor/appointment_policy.rego" ]; then
    echo "Removing appointment policy..."
    rm -f .cupcake/policies/cursor/appointment_policy.rego
    echo "✅ Appointment policy removed"
fi

# Remove signal script (mcp_setup.sh copies it to .cupcake/)
if [ -f ".cupcake/check_appointment_time.py" ]; then
    echo "Removing appointment time check signal..."
    rm -f .cupcake/check_appointment_time.py
    echo "✅ Signal script removed"
fi

echo ""
echo "🧹 MCP demo cleanup complete!"
echo ""
echo "Run ./mcp_setup.sh to set up the demo again."