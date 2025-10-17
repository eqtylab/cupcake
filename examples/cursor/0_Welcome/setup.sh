#!/bin/bash
set -e

echo "Cupcake Cursor Evaluation Setup"
echo "================================"

# Check if Rust/Cargo is installed
if ! command -v cargo &> /dev/null; then
    echo "❌ Cargo not found in PATH. Please install Rust:"
    echo "   https://rustup.rs/"
    exit 1
else
    echo "✅ Cargo found: $(cargo --version | head -n1)"
fi

# Check if OPA is installed
if ! command -v opa &> /dev/null; then
    echo "❌ OPA not found in PATH. Please install OPA:"
    echo "   https://www.openpolicyagent.org/docs/latest/#running-opa"
    exit 1
else
    echo "✅ OPA found: $(opa version | head -n1)"
fi

# Build Cupcake binary
echo "Building Cupcake binary..."
cd ../../..
cargo build --release
echo "✅ Build complete"

# Add to PATH for this session
export PATH="$(pwd)/target/release:$PATH"
echo "✅ Added cupcake to PATH for this session"

# Return to examples directory
cd examples/cursor/0_Welcome

# Initialize Cupcake project with Cursor harness
# This creates both claude/ and cursor/ subdirectories (allowing dual-harness usage)
# and automatically configures hooks at ~/.cursor/hooks.json
echo "Initializing Cupcake project..."
cupcake init --harness cursor
echo "✅ Cupcake project initialized"

# Copy Cursor-specific example policies from fixtures
# These are demo policies specific to this walkthrough
echo "Copying Cursor-specific example policies..."
cp ../../fixtures/cursor/security_policy.rego .cupcake/policies/cursor/
cp ../../fixtures/cursor/file_protection.rego .cupcake/policies/cursor/
cp ../../fixtures/cursor/mcp_protection.rego .cupcake/policies/cursor/
cp ../../fixtures/cursor/prompt_filter.rego .cupcake/policies/cursor/
echo "✅ Cursor-specific policies copied"

# No need to manually compile policies - the engine compiles them automatically
# The engine only compiles .cupcake/policies/cursor/ when --harness cursor is used
echo "✅ Policies ready (engine will compile on first run)"

# Override hooks configuration to use absolute paths and enable logging
# (cupcake init already configured hooks, but we want absolute paths to both
# the binary, policy directory, OPA, and debug dir since Cursor doesn't inherit shell environment)
echo "Configuring Cursor hooks with absolute paths..."
CUPCAKE_PATH="$(realpath ../../../target/release/cupcake)"
POLICY_DIR="$(pwd)/.cupcake"
OPA_PATH="$(which opa)"
DEBUG_DIR="$(pwd)/.cupcake/debug"
HOOKS_FILE="$HOME/.cursor/hooks.json"

echo "   Cupcake binary: $CUPCAKE_PATH"
echo "   Policy directory: $POLICY_DIR"
echo "   OPA binary: $OPA_PATH"
echo "   Debug directory: $DEBUG_DIR"

# Create .cursor directory if it doesn't exist
mkdir -p "$HOME/.cursor"

# Check if hooks.json already exists
if [ -f "$HOOKS_FILE" ]; then
    echo "⚠️  Existing hooks.json found. Creating backup..."
    cp "$HOOKS_FILE" "$HOOKS_FILE.backup.$(date +%Y%m%d_%H%M%S)"
fi

# Create Cursor hooks configuration with absolute paths and debug enabled
cat > "$HOOKS_FILE" << EOF
{
  "version": 1,
  "hooks": {
    "beforeShellExecution": [
      {
        "command": "$CUPCAKE_PATH eval --harness cursor --policy-dir $POLICY_DIR --opa-path $OPA_PATH --debug-dir $DEBUG_DIR --log-level info --debug-files"
      }
    ],
    "beforeMCPExecution": [
      {
        "command": "$CUPCAKE_PATH eval --harness cursor --policy-dir $POLICY_DIR --opa-path $OPA_PATH --debug-dir $DEBUG_DIR --log-level info --debug-files"
      }
    ],
    "afterFileEdit": [
      {
        "command": "$CUPCAKE_PATH eval --harness cursor --policy-dir $POLICY_DIR --opa-path $OPA_PATH --debug-dir $DEBUG_DIR --log-level info --debug-files"
      }
    ],
    "beforeReadFile": [
      {
        "command": "$CUPCAKE_PATH eval --harness cursor --policy-dir $POLICY_DIR --opa-path $OPA_PATH --debug-dir $DEBUG_DIR --log-level info --debug-files"
      }
    ],
    "beforeSubmitPrompt": [
      {
        "command": "$CUPCAKE_PATH eval --harness cursor --policy-dir $POLICY_DIR --opa-path $OPA_PATH --debug-dir $DEBUG_DIR --log-level info --debug-files"
      }
    ],
    "stop": [
      {
        "command": "$CUPCAKE_PATH eval --harness cursor --policy-dir $POLICY_DIR --opa-path $OPA_PATH --debug-dir $DEBUG_DIR --log-level info --debug-files"
      }
    ]
  }
}
EOF

echo "✅ Cursor hooks configured at $HOOKS_FILE"

# Create test events for debugging
mkdir -p test-events

cat > test-events/shell-rm.json << 'EOF'
{
  "hook_event_name": "beforeShellExecution",
  "conversation_id": "test-001",
  "generation_id": "gen-001",
  "workspace_roots": ["/tmp"],
  "command": "rm -rf /tmp/test",
  "cwd": "/tmp"
}
EOF

cat > test-events/file-read-ssh.json << 'EOF'
{
  "hook_event_name": "beforeReadFile",
  "conversation_id": "test-002",
  "generation_id": "gen-002",
  "workspace_roots": ["/home/user"],
  "file_path": "/home/user/.ssh/id_rsa",
  "file_content": "-----BEGIN OPENSSH PRIVATE KEY-----",
  "attachments": []
}
EOF

echo "✅ Test events created in test-events/"

# Create screenshots directory
mkdir -p screenshots
echo "📸 Screenshots directory created (placeholder for demo screenshots)"

echo ""
echo "🎉 Setup complete!"
echo ""
echo "Next steps:"
echo "1. Restart Cursor to load the new hooks configuration"
echo "2. Open this directory in Cursor: cursor ."
echo "3. Try commands that trigger policies:"
echo "   - 'delete /tmp/test directory' (blocks rm -rf)"
echo "   - 'read my SSH key' (blocks sensitive file access)"
echo "   - 'run sudo apt update' (blocks sudo with guidance)"
echo ""
echo "Test policies manually:"
echo "cupcake eval --harness cursor --policy-dir .cupcake < test-events/shell-rm.json"
echo ""
echo "View active policies:"
echo "cupcake inspect --policy-dir .cupcake --table"