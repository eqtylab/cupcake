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
echo "Initializing Cupcake project for Cursor..."
cupcake init --harness cursor
echo "✅ Project initialized with Cursor harness"

# Copy Cursor-specific example policies from fixtures
echo "Copying Cursor-specific example policies..."
mkdir -p .cupcake/policies/cursor
cp ../../fixtures/cursor/security_policy.rego .cupcake/policies/cursor/
cp ../../fixtures/cursor/file_protection.rego .cupcake/policies/cursor/
cp ../../fixtures/cursor/mcp_protection.rego .cupcake/policies/cursor/
cp ../../fixtures/cursor/prompt_filter.rego .cupcake/policies/cursor/
echo "✅ Cursor-specific policies copied"

# Copy system aggregation policy
echo "Copying system aggregation policy..."
mkdir -p .cupcake/policies/system
cp ../../fixtures/system/evaluate.rego .cupcake/policies/system/
echo "✅ System policy copied"

# Compile policies to WASM
echo "Compiling policies to WASM..."
opa build -t wasm -e cupcake/system/evaluate .cupcake/policies/
echo "✅ Policies compiled to bundle.tar.gz"

# Create Cursor hooks configuration
echo "Setting up Cursor hooks integration..."
CUPCAKE_PATH="$(realpath ../../../target/release/cupcake)"
HOOKS_FILE="$HOME/.cursor/hooks.json"

# Create .cursor directory if it doesn't exist
mkdir -p "$HOME/.cursor"

# Check if hooks.json already exists
if [ -f "$HOOKS_FILE" ]; then
    echo "⚠️  Existing hooks.json found. Creating backup..."
    cp "$HOOKS_FILE" "$HOOKS_FILE.backup.$(date +%Y%m%d_%H%M%S)"
fi

# Create Cursor hooks configuration
cat > "$HOOKS_FILE" << EOF
{
  "version": 1,
  "hooks": {
    "beforeShellExecution": [
      {
        "command": "$CUPCAKE_PATH eval --harness cursor --log-level info"
      }
    ],
    "beforeMCPExecution": [
      {
        "command": "$CUPCAKE_PATH eval --harness cursor --log-level info"
      }
    ],
    "afterFileEdit": [
      {
        "command": "$CUPCAKE_PATH eval --harness cursor --log-level info"
      }
    ],
    "beforeReadFile": [
      {
        "command": "$CUPCAKE_PATH eval --harness cursor --log-level info"
      }
    ],
    "beforeSubmitPrompt": [
      {
        "command": "$CUPCAKE_PATH eval --harness cursor --log-level info"
      }
    ],
    "stop": [
      {
        "command": "$CUPCAKE_PATH eval --harness cursor --log-level info"
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
echo "cupcake eval --harness cursor < test-events/shell-rm.json"
echo ""
echo "View active policies:"
echo "cupcake inspect --harness cursor"