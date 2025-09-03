#!/bin/bash
set -e

echo "🧁 Cupcake Evaluation Setup"
echo "=========================="

# Check if OPA is installed
if ! command -v opa &> /dev/null; then
    echo "❌ OPA not found in PATH. Please install OPA:"
    echo "   https://www.openpolicyagent.org/docs/latest/#running-opa"
    exit 1
else
    echo "✅ OPA found: $(opa version)"
fi

# Build Cupcake binary
echo "Building Cupcake binary..."
cd ../..
cargo build --release
echo "✅ Build complete"

# Add to PATH for this session
export PATH="$(pwd)/target/release:$PATH"
echo "✅ Added cupcake to PATH for this session"

# Return to eval directory
cd eval/0_Claude-Code-Welcome1

# Initialize Cupcake project
echo "Initializing Cupcake project..."
cupcake init
echo "✅ Project initialized"

# Copy example policies
echo "Copying example policies..."
cp ../fixtures/security_policy.rego .cupcake/policies/
cp ../fixtures/git_workflow.rego .cupcake/policies/
cp ../fixtures/context_injection.rego .cupcake/policies/
echo "✅ Example policies copied"

# Enable some builtins for demonstration
echo "Configuring builtins..."
# Replace the empty builtins section with our configuration
sed -i '' '/^builtins:$/,/^$/c\
builtins:\
  protected_paths:\
    enabled: true\
    paths:\
      - "/etc/"\
      - "/System/"\
      - "~/.ssh/"\
    message: "System path modification blocked by policy"\
\
  git_pre_check:\
    enabled: true\
    checks:\
      - command: "echo Validation passed"\
        message: "Basic validation check"\
' .cupcake/guidebook.yml

echo "✅ Builtins configured"

# Compile policies to WASM
echo "Compiling policies to WASM..."
opa build -t wasm -e cupcake/system/evaluate .cupcake/policies/
echo "✅ Policies compiled to bundle.tar.gz"

# Create Claude Code settings directory and hooks integration
echo "Setting up Claude Code hooks integration..."
mkdir -p .claude

# Create Claude Code settings with direct cargo command (like working demo)
MANIFEST_PATH="$(realpath ../../Cargo.toml)"
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
            "command": "cargo run --manifest-path $MANIFEST_PATH -- eval",
            "timeout": 30,
            "env": {
              "RUST_LOG": "info",
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
            "command": "cargo run --manifest-path $MANIFEST_PATH -- eval",
            "timeout": 30,
            "env": {
              "RUST_LOG": "info",
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
            "command": "cargo run --manifest-path $MANIFEST_PATH -- eval",
            "timeout": 30,
            "env": {
              "RUST_LOG": "info",
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
echo "1. Run 'export PATH=\"$(realpath ../../target/release):\$PATH\"' to add cupcake to your shell PATH"
echo "2. Start Claude Code in this directory"
echo "3. Try running commands that trigger policies"
echo ""
echo "Example commands to test:"
echo "- ls -la (safe, should work)"  
echo "- rm -rf /tmp/test (dangerous, should block)"
echo "- Edit /etc/hosts (system file, should block)"
echo "- git push --force (risky, should ask)"