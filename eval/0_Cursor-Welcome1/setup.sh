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
cd ../..
cargo build --release
echo "✅ Build complete"

# Add to PATH for this session
export PATH="$(pwd)/target/release:$PATH"
echo "✅ Added cupcake to PATH for this session"

# Return to eval directory
cd eval/0_Cursor-Welcome1

# Initialize Cupcake project with Cursor harness
echo "Initializing Cupcake project for Cursor..."
cupcake init --harness cursor
echo "✅ Project initialized with Cursor harness"

# Copy Cursor-specific example policies
echo "Creating Cursor-specific policies..."

# Create security policy for Cursor
cat > .cupcake/policies/cursor/security_policy.rego << 'EOF'
# METADATA
# scope: package
# title: Cursor Security Policy
# custom:
#   routing:
#     required_events: ["beforeShellExecution"]
package cupcake.policies.cursor.security

import rego.v1

# Block dangerous shell commands with differentiated feedback
deny contains decision if {
    input.hook_event_name == "beforeShellExecution"
    dangerous_commands := ["rm -rf", "sudo rm", "format", "fdisk", "> /dev/"]
    some cmd in dangerous_commands
    contains(input.command, cmd)

    decision := {
        "rule_id": "CURSOR-SECURITY-001",
        "reason": concat(" ", ["Dangerous command blocked:", cmd]),
        "agent_context": concat("", [
            cmd, " detected in command. This is a destructive operation. ",
            "Alternatives: 1) Use 'trash' command for safe deletion, ",
            "2) Be more specific with paths, ",
            "3) Use --dry-run flag first to preview changes."
        ]),
        "severity": "CRITICAL"
    }
}

# Block sudo with helpful agent guidance
deny contains decision if {
    input.hook_event_name == "beforeShellExecution"
    contains(input.command, "sudo")

    decision := {
        "rule_id": "CURSOR-SUDO-001",
        "reason": "Elevated privileges required",
        "agent_context": "sudo detected. Elevated privileges are dangerous. Consider: 1) Use specific commands without sudo, 2) Modify file permissions instead, 3) Use Docker containers for isolation. If you must use sudo, ask the user to run it manually.",
        "severity": "HIGH"
    }
}
EOF

# Create file protection policy
cat > .cupcake/policies/cursor/file_protection.rego << 'EOF'
# METADATA
# scope: package
# title: Cursor File Protection
# custom:
#   routing:
#     required_events: ["beforeReadFile", "afterFileEdit"]
package cupcake.policies.cursor.file_protection

import rego.v1

# Protect sensitive files from reading
deny contains decision if {
    input.hook_event_name == "beforeReadFile"
    sensitive_patterns := [".ssh/id_", ".aws/credentials", ".env", "secrets"]
    some pattern in sensitive_patterns
    contains(input.file_path, pattern)

    decision := {
        "rule_id": "CURSOR-FILE-READ-001",
        "reason": "Access to sensitive file blocked",
        "agent_context": concat("", [
            "Attempted to read sensitive file containing '", pattern, "'. ",
            "These files contain secrets that should not be exposed. ",
            "Instead: 1) Ask user to provide redacted version, ",
            "2) Use environment variables, ",
            "3) Create example/template files without real secrets."
        ]),
        "severity": "CRITICAL"
    }
}

# Validate file edits
deny contains decision if {
    input.hook_event_name == "afterFileEdit"
    contains(input.file_path, "/etc/")

    decision := {
        "rule_id": "CURSOR-FILE-EDIT-001",
        "reason": "System file modification blocked",
        "agent_context": "Attempted to modify system file in /etc/. System files require manual intervention. Create configuration in user space instead.",
        "severity": "HIGH"
    }
}
EOF

# Create MCP protection policy
cat > .cupcake/policies/cursor/mcp_protection.rego << 'EOF'
# METADATA
# scope: package
# title: Cursor MCP Tool Protection
# custom:
#   routing:
#     required_events: ["beforeMCPExecution"]
package cupcake.policies.cursor.mcp_protection

import rego.v1

# Block dangerous database operations
deny contains decision if {
    input.hook_event_name == "beforeMCPExecution"
    startswith(input.tool_name, "postgres")
    dangerous_ops := ["DELETE", "DROP", "TRUNCATE"]
    some op in dangerous_ops
    contains(upper(input.tool_input), op)

    decision := {
        "rule_id": "CURSOR-MCP-001",
        "reason": concat(" ", ["Dangerous database operation blocked:", op]),
        "agent_context": concat("", [
            op, " operation detected in SQL. ",
            "Destructive database operations are not allowed. ",
            "Alternatives: 1) Use SELECT to query data, ",
            "2) Use UPDATE to modify specific records, ",
            "3) Create backups before destructive operations."
        ]),
        "severity": "CRITICAL"
    }
}

# Ask for confirmation on data modifications
ask contains decision if {
    input.hook_event_name == "beforeMCPExecution"
    contains(upper(input.tool_input), "UPDATE")

    decision := {
        "rule_id": "CURSOR-MCP-002",
        "reason": "Database update requires confirmation",
        "question": "Allow database update operation?",
        "severity": "MEDIUM"
    }
}
EOF

# Create prompt filter policy
cat > .cupcake/policies/cursor/prompt_filter.rego << 'EOF'
# METADATA
# scope: package
# title: Cursor Prompt Filtering
# custom:
#   routing:
#     required_events: ["beforeSubmitPrompt"]
package cupcake.policies.cursor.prompt_filter

import rego.v1

# Block prompts containing secrets
deny contains decision if {
    input.hook_event_name == "beforeSubmitPrompt"
    secret_patterns := ["password", "api_key", "secret", "token"]
    some pattern in secret_patterns
    contains(lower(input.prompt), pattern)

    # Check if it looks like an actual secret (long random string)
    regex.match(`[A-Za-z0-9]{20,}`, input.prompt)

    decision := {
        "rule_id": "CURSOR-PROMPT-001",
        "reason": "Potential secret detected in prompt",
        "severity": "HIGH"
    }
}
EOF

echo "✅ Cursor-specific policies created"

# Compile policies to WASM
echo "Compiling policies to WASM..."
opa build -t wasm -e cupcake/system/evaluate .cupcake/policies/
echo "✅ Policies compiled to bundle.tar.gz"

# Create Cursor hooks configuration
echo "Setting up Cursor hooks integration..."
CUPCAKE_PATH="$(realpath ../../target/release/cupcake)"
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