#!/bin/bash
# Test script for routing debug system

echo "Testing Cupcake Routing Debug System"
echo "===================================="
echo

# Navigate to example directory with policies
cd "$(dirname "$0")/0_start_here_demo" || exit 1

echo "1. Running without debug (normal operation):"
echo "   cupcake eval < test_event.json"
echo

echo "2. Running with routing debug enabled:"
export CUPCAKE_DEBUG_ROUTING=1
echo "   CUPCAKE_DEBUG_ROUTING=1 cupcake eval < test_event.json"
echo

# Create a test event
cat > test_event.json << 'EOF'
{
  "hook_event_name": "PreToolUse",
  "tool_name": "Bash",
  "tool_input": {
    "command": "echo 'test'"
  },
  "session_id": "test-session",
  "cwd": "/tmp"
}
EOF

# Run cupcake with debug enabled (using cargo run for development)
echo "Running cupcake with routing debug..."
cargo run --package cupcake-cli -- eval 2>/dev/null <<< '{"hook_event_name":"PreToolUse","tool_name":"Bash","tool_input":{"command":"echo test"},"session_id":"test","cwd":"/tmp"}'

echo
echo "Checking debug output:"
if [ -d ".cupcake/debug/routing" ]; then
    echo "Debug directory created: .cupcake/debug/routing/"
    echo "Files generated:"
    ls -la .cupcake/debug/routing/ | grep routing_map
    echo
    echo "Sample of text output:"
    head -30 .cupcake/debug/routing/routing_map_*.txt 2>/dev/null | tail -20
else
    echo "No debug output found. Make sure CUPCAKE_DEBUG_ROUTING=1 is set."
fi

echo
echo "To view the full routing map:"
echo "  cat .cupcake/debug/routing/routing_map_*.txt"
echo
echo "To generate a visual graph (requires graphviz):"
echo "  dot -Tpng .cupcake/debug/routing/routing_map_*.dot -o routing_graph.png"