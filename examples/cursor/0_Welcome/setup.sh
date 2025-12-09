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

# Check if uv is installed
if ! command -v uv &> /dev/null; then
    echo "❌ uv not found in PATH. Please install uv:"
    echo "   https://docs.astral.sh/uv/"
    exit 1
else
    echo "✅ uv found: $(uv --version)"
fi

# Build Cupcake binary
echo "Building Cupcake binary..."
cd ../../..
cargo build --release
echo "✅ Build complete"

# Store the cupcake binary path
CUPCAKE_BIN="$(pwd)/target/release/cupcake"
echo "✅ Using cupcake binary at: $CUPCAKE_BIN"

# Return to examples directory
cd examples/cursor/0_Welcome

# Initialize Cupcake project using the explicit path
echo "Initializing Cupcake project with Cursor harness..."
"$CUPCAKE_BIN" init --harness cursor
echo "✅ Project initialized"

# Copy example policies to Cursor policies directory
echo "Copying example policies..."
cp ../../fixtures/cursor/security_policy.rego .cupcake/policies/cursor/
cp ../../fixtures/git_workflow.rego .cupcake/policies/cursor/
echo "✅ Example policies copied (context_injection skipped - not supported by Cursor)"

# Builtins are now pre-configured in the base template
echo "✅ Builtins configured (protected_paths, git_pre_check, rulebook_security_guardrails)"

# Compile policies to WASM (Cursor policies + shared helpers)
echo "Compiling Cursor policies to WASM..."
opa build -t wasm -e cupcake/system/evaluate .cupcake/policies/cursor/ .cupcake/policies/helpers/
echo "✅ Policies compiled to bundle.tar.gz"

# Create project-level Cursor hooks configuration
echo "Setting up Cursor hooks integration..."
mkdir -p .cursor

cat > .cursor/hooks.json << EOF
{
  "version": 1,
  "hooks": {
    "beforeShellExecution": [
      {
        "command": "$CUPCAKE_BIN eval --harness cursor --log-level info"
      }
    ],
    "beforeMCPExecution": [
      {
        "command": "$CUPCAKE_BIN eval --harness cursor --log-level info"
      }
    ],
    "afterFileEdit": [
      {
        "command": "$CUPCAKE_BIN eval --harness cursor --log-level info"
      }
    ],
    "beforeReadFile": [
      {
        "command": "$CUPCAKE_BIN eval --harness cursor --log-level info"
      }
    ],
    "beforeSubmitPrompt": [
      {
        "command": "$CUPCAKE_BIN eval --harness cursor --log-level info"
      }
    ],
    "stop": [
      {
        "command": "$CUPCAKE_BIN eval --harness cursor --log-level info"
      }
    ]
  }
}
EOF

echo "✅ Cursor hooks configured at .cursor/hooks.json"

# Create test events directory
mkdir -p test-events

# Create test event files
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
  "content": "-----BEGIN OPENSSH PRIVATE KEY-----",
  "attachments": []
}
EOF

echo "✅ Test events created in test-events/"

echo ""
echo "🎉 Setup complete!"
echo ""
echo "Next steps:"
echo "1. Open this directory in Cursor (hooks load automatically)"
echo "2. Try commands that trigger policies:"
echo "   - 'delete /tmp/test directory' (blocks rm -rf)"
echo "   - 'read my SSH key' (blocks sensitive file access)"
echo "   - 'run sudo command' (blocks admin operations)"
echo ""
echo "Test policies manually:"
echo "cat test-events/shell-rm.json | $CUPCAKE_BIN eval --harness cursor"
echo ""
echo "View active policies:"
echo "$CUPCAKE_BIN inspect --harness cursor"
