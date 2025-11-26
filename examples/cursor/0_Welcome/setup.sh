#!/bin/bash
set -e

echo "Cupcake Evaluation Setup"
echo "=========================="

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

# Create Claude Code settings directory and hooks integration
echo "Setting up Claude Code hooks integration..."
mkdir -p .claude

# Create Claude Code settings with direct cargo command (like working demo)
MANIFEST_PATH="$(realpath ../../../Cargo.toml)"
OPA_DIR="$(dirname "$(which opa)")"

cat > .claude/settings.json << EOF
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "*",
        "hooks": [
          {
            "type": "command",
            "command": "cargo run --manifest-path $MANIFEST_PATH -- eval --log-level info",
            "timeout": 120,
            "env": {
              "PATH": "$OPA_DIR:\$PATH"
            }
          }
        ]
      }
    ],
    "PostToolUse": [
      {
        "matcher": "*", 
        "hooks": [
          {
            "type": "command",
            "command": "cargo run --manifest-path $MANIFEST_PATH -- eval --log-level info",
            "timeout": 120,
            "env": {
              "PATH": "$OPA_DIR:\$PATH"
            }
          }
        ]
      }
    ],
    "UserPromptSubmit": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "cargo run --manifest-path $MANIFEST_PATH -- eval --log-level info",
            "timeout": 120,
            "env": {
              "PATH": "$OPA_DIR:\$PATH"
            }
          }
        ]
      }
    ]
  }
}
EOF

echo "✅ Claude Code hooks configured"

echo ""
echo "🎉 Setup complete!"
echo ""
echo "Next steps:"
echo "1. To add cupcake to your PATH, run:"
echo "   export PATH=\"$(realpath ../../../target/release):\$PATH\""
echo "2. Start Claude Code in this directory"
echo "3. Try running commands that trigger policies"
echo ""
echo "Example commands to test:"
echo "- ls -la (safe, should work)"  
echo "- rm -rf /tmp/test (dangerous, should block)"
echo "- Edit /etc/hosts (system file, should block)"
echo "- git push --force (risky, should ask)"