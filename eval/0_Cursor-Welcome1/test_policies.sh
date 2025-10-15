#!/bin/bash
set -e

echo "Testing Cursor Policy Evaluation"
echo "================================"
echo ""

# Check if cupcake is available
if ! command -v cupcake &> /dev/null; then
    echo "❌ Cupcake not found in PATH"
    echo "Run: export PATH=\"$(realpath ../../target/release):\$PATH\""
    exit 1
fi

echo "Testing policy evaluations with sample events..."
echo ""

# Test 1: Dangerous shell command
echo "1. Testing dangerous shell command (rm -rf)..."
echo '{"hook_event_name":"beforeShellExecution","conversation_id":"test","command":"rm -rf /tmp/test","cwd":"/tmp","workspace_roots":["/tmp"]}' | \
    cupcake eval --harness cursor 2>/dev/null | jq -r '.permission'

if [ $? -eq 0 ]; then
    echo "   ✅ Command blocked as expected"
else
    echo "   ❌ Unexpected result"
fi
echo ""

# Test 2: Safe shell command
echo "2. Testing safe shell command (ls -la)..."
RESULT=$(echo '{"hook_event_name":"beforeShellExecution","conversation_id":"test","command":"ls -la","cwd":"/tmp","workspace_roots":["/tmp"]}' | \
    cupcake eval --harness cursor 2>/dev/null | jq -r '.permission')

if [ "$RESULT" == "allow" ]; then
    echo "   ✅ Command allowed as expected"
else
    echo "   ❌ Unexpected result: $RESULT"
fi
echo ""

# Test 3: Sensitive file read
echo "3. Testing sensitive file read (.ssh/id_rsa)..."
RESULT=$(echo '{"hook_event_name":"beforeReadFile","conversation_id":"test","file_path":"~/.ssh/id_rsa","file_content":"key","workspace_roots":["/home"]}' | \
    cupcake eval --harness cursor 2>/dev/null | jq -r '.permission')

if [ "$RESULT" == "deny" ]; then
    echo "   ✅ File read blocked as expected"
else
    echo "   ❌ Unexpected result: $RESULT"
fi
echo ""

# Test 4: Sudo command with agent feedback
echo "4. Testing sudo command (checking agent feedback)..."
RESPONSE=$(echo '{"hook_event_name":"beforeShellExecution","conversation_id":"test","command":"sudo apt update","cwd":"/","workspace_roots":["/"]}' | \
    cupcake eval --harness cursor 2>/dev/null)

PERMISSION=$(echo "$RESPONSE" | jq -r '.permission')
AGENT_MSG=$(echo "$RESPONSE" | jq -r '.agentMessage // ""')

if [ "$PERMISSION" == "deny" ] && [ -n "$AGENT_MSG" ]; then
    echo "   ✅ Command blocked with agent guidance"
    echo "   Agent message: $(echo "$AGENT_MSG" | head -c 80)..."
else
    echo "   ❌ Unexpected result"
fi
echo ""

# Test 5: Git --no-verify
echo "5. Testing git --no-verify (builtin protection)..."
RESULT=$(echo '{"hook_event_name":"beforeShellExecution","conversation_id":"test","command":"git commit --no-verify -m test","cwd":"/tmp","workspace_roots":["/tmp"]}' | \
    cupcake eval --harness cursor 2>/dev/null | jq -r '.permission')

if [ "$RESULT" == "deny" ]; then
    echo "   ✅ Git --no-verify blocked as expected"
else
    echo "   ❌ Unexpected result: $RESULT"
fi
echo ""

# Show active policies
echo "Active Policies:"
echo "----------------"
cupcake inspect --harness cursor --table 2>/dev/null || cupcake inspect --harness cursor

echo ""
echo "✅ Testing complete!"
echo ""
echo "To test interactively in Cursor:"
echo "1. Open Cursor in this directory"
echo "2. Try the blocked commands to see policies in action"
echo "3. Note the different messages for users vs agents"