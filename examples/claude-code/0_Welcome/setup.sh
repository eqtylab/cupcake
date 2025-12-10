#!/bin/bash
set -e

echo "Cupcake Claude Code Evaluation Setup"
echo "====================================="

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

# Save current directory to return to later
ORIGINAL_DIR="$(pwd)"

# Build Cupcake binary
echo "Building Cupcake binary..."
cd ../../..
cargo build --release
echo "✅ Build complete"

# Store the cupcake binary path
CUPCAKE_BIN="$(pwd)/target/release/cupcake"
echo "✅ Using cupcake binary at: $CUPCAKE_BIN"

# Return to original directory
cd "$ORIGINAL_DIR"

# Initialize Cupcake project using the explicit path
echo "Initializing Cupcake project with Claude Code harness..."
"$CUPCAKE_BIN" init --harness claude
echo "✅ Project initialized"

# Update settings.json to use the full path to the cupcake binary
# This ensures Claude Code can find cupcake even if it's not in PATH
echo "Updating settings.json with full binary path..."
if [[ "$OSTYPE" == "darwin"* ]]; then
    # macOS uses BSD sed
    sed -i '' "s|cupcake eval|$CUPCAKE_BIN eval|g" .claude/settings.json
else
    # Linux uses GNU sed
    sed -i "s|cupcake eval|$CUPCAKE_BIN eval|g" .claude/settings.json
fi
echo "✅ Hooks configured with: $CUPCAKE_BIN"

# Copy example policies to Claude Code policies directory
echo "Copying example policies..."
cp ../../fixtures/security_policy.rego .cupcake/policies/claude/
cp ../../fixtures/git_workflow.rego .cupcake/policies/claude/
cp ../../fixtures/context_injection.rego .cupcake/policies/claude/
echo "✅ Example policies copied"

# Builtins are now pre-configured in the base template
echo "✅ Builtins configured (protected_paths, git_pre_check, rulebook_security_guardrails)"

# Note: WASM compilation is handled automatically by 'cupcake eval' at runtime
# No manual 'opa build' step needed - cupcake compiles policies including helpers

echo ""
echo "🎉 Setup complete!"
echo ""
echo "Next steps:"
echo "1. To add cupcake to your PATH, run:"
echo "   export PATH=\"$(realpath ../../../target/release):\$PATH\""
echo "2. Open this directory in Claude Code"
echo "3. Try running commands that trigger policies"
echo ""
echo "Manual testing with test events:"
echo "   cupcake eval --harness claude < test-events/shell-rm.json"
echo ""
echo "Example commands to test in Claude Code:"
echo "- ls -la (safe, should work)"
echo "- rm -rf /tmp/test (dangerous, should block)"
echo "- Edit /etc/hosts (system file, should block)"
echo "- git push --force (risky, should ask)"
