#!/bin/bash

echo "Cupcake MCP Database Demo Cleanup"
echo "===================================="

# Stop and remove PostgreSQL container
echo "Stopping PostgreSQL container..."
docker stop cupcake-demo-db 2>/dev/null || echo "Container not running"
docker rm cupcake-demo-db 2>/dev/null || echo "Container already removed"
echo "✅ PostgreSQL container removed"

# Remove appointment policy
if [ -f .cupcake/policies/appointment_policy.rego ]; then
    echo "Removing appointment policy..."
    rm .cupcake/policies/appointment_policy.rego
    echo "✅ Appointment policy removed"
fi

# Remove CLAUDE.md if it exists
if [ -f CLAUDE.md ]; then
    echo "Removing CLAUDE.md..."
    rm CLAUDE.md
    echo "✅ CLAUDE.md removed"
fi

# Recompile policies without appointment policy
if [ -d .cupcake/policies ]; then
    echo "Recompiling policies..."
    opa build -t wasm -e cupcake/system/evaluate .cupcake/policies/
    echo "✅ Policies recompiled"
fi

# Remove .mcp.json if it exists
if [ -f .mcp.json ]; then
    echo "Removing .mcp.json MCP configuration..."
    rm .mcp.json
    echo "✅ .mcp.json removed"
fi

echo ""
echo "✅ MCP Database Demo cleanup complete!"
echo ""
echo "Note: You may need to restart Claude Code if it was running"