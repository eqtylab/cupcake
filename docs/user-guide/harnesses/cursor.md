# Cursor Integration Guide

Cupcake provides comprehensive policy enforcement for [Cursor](https://cursor.com), the AI-powered code editor. This guide shows you how to set up and use Cupcake with Cursor.

## Quick Start

### 1. Install Cupcake

```bash
# Install from source (recommended for development)
cargo install --path cupcake-cli

# Or download pre-built binary from releases
```

### 2. Initialize Your Project

Navigate to your project directory and run:

```bash
cupcake init --harness cursor
```

This creates a `.cupcake/` directory with the following structure:

```
.cupcake/
├── policies/
│   ├── claude/          # Claude Code policies (for comparison)
│   │   ├── system/
│   │   │   └── evaluate.rego
│   │   └── builtins/
│   └── cursor/          # Cursor-specific policies
│       ├── system/
│       │   └── evaluate.rego
│       └── builtins/
├── signals/
├── actions/
└── rulebook.yml
```

### 3. Configure Cursor

The `init` command automatically configures Cursor by adding hooks to `~/.cursor/hooks.json`:

```json
{
  "version": 1,
  "hooks": {
    "beforeShellExecution": [
      { "command": "cupcake eval --harness cursor" }
    ],
    "beforeMCPExecution": [
      { "command": "cupcake eval --harness cursor" }
    ],
    "afterFileEdit": [
      { "command": "cupcake eval --harness cursor" }
    ],
    "beforeReadFile": [
      { "command": "cupcake eval --harness cursor" }
    ],
    "beforeSubmitPrompt": [
      { "command": "cupcake eval --harness cursor" }
    ],
    "stop": [
      { "command": "cupcake eval --harness cursor" }
    ]
  }
}
```

### 4. Start Using Cursor

Once configured, Cupcake will automatically evaluate all Cursor actions against your policies. The agent will be blocked if policies deny the action.

---

## Understanding Cursor Events

Cursor provides 6 hook events that Cupcake can intercept:

| Hook Event | When It Fires | Use Case |
|------------|---------------|----------|
| `beforeShellExecution` | Before running shell commands | Block dangerous commands like `rm -rf /` |
| `beforeMCPExecution` | Before calling MCP tools | Control external tool access |
| `afterFileEdit` | After file modifications | Run validation, linting, or auditing |
| `beforeReadFile` | Before reading file contents | Protect sensitive files |
| `beforeSubmitPrompt` | Before sending prompt to LLM | Filter prompts, enforce guidelines |
| `stop` | When agent stops | Cleanup, logging, notifications |

### Event Data Structures

Each event has a specific JSON structure. Here are the key events:

#### beforeShellExecution
```json
{
  "hook_event_name": "beforeShellExecution",
  "conversation_id": "conv_123",
  "generation_id": "gen_456",
  "workspace_roots": ["/path/to/project"],
  "command": "git commit -m 'fix bug'",
  "cwd": "/path/to/project"
}
```

#### beforeMCPExecution
```json
{
  "hook_event_name": "beforeMCPExecution",
  "conversation_id": "conv_123",
  "generation_id": "gen_456",
  "workspace_roots": ["/path/to/project"],
  "tool_name": "memory_store",
  "tool_input": {"key": "value"}
}
// Plus either:
{ "url": "http://localhost:3000/mcp" }
// Or:
{ "command": "node mcp-server.js" }
```

#### beforeReadFile
```json
{
  "hook_event_name": "beforeReadFile",
  "conversation_id": "conv_123",
  "generation_id": "gen_456",
  "workspace_roots": ["/path/to/project"],
  "file_path": "/path/to/file.txt",
  "file_content": "file contents here...",
  "attachments": [
    {
      "type": "rule",
      "file_path": "/path/to/.cursorrules"
    }
  ]
}
```

**Attachments**: Array of objects with:
- `type`: Either `"file"` or `"rule"`
- `file_path`: Absolute path to the attached file
  - `"rule"` type indicates cursor rules/configuration files
  - `"file"` type indicates regular attached files

#### beforeSubmitPrompt
```json
{
  "hook_event_name": "beforeSubmitPrompt",
  "conversation_id": "conv_123",
  "generation_id": "gen_456",
  "workspace_roots": ["/path/to/project"],
  "prompt": "Help me implement authentication",
  "attachments": [
    {
      "type": "file",
      "file_path": "/path/to/auth.js"
    }
  ]
}
```

**Attachments**: Same structure as `beforeReadFile` - array of objects with `type` and `file_path`.

#### stop
```json
{
  "hook_event_name": "stop",
  "conversation_id": "conv_123",
  "generation_id": "gen_456",
  "workspace_roots": ["/path/to/project"],
  "status": "completed"
}
```

**Status Values**:
- `"completed"` - Agent finished its task successfully
- `"aborted"` - User stopped the agent
- `"error"` - Agent encountered an error

---

## Writing Policies for Cursor

Policies for Cursor are written in Rego and placed in `.cupcake/policies/cursor/`.

### Basic Policy Structure

```rego
# METADATA
# scope: package
# custom:
#   routing:
#     required_events: ["beforeShellExecution"]
package cursor.policies.block_dangerous_commands

import rego.v1

deny contains decision if {
    input.hook_event_name == "beforeShellExecution"
    contains(input.command, "rm -rf /")
    decision := {
        "rule_id": "CURSOR-SHELL-001",
        "reason": "Dangerous recursive delete command blocked",
        "severity": "CRITICAL"
    }
}
```

### Providing Agent Feedback

When blocking actions, Cursor allows you to provide **separate messages** for users and agents:

- **`reason`**: User-friendly message shown to the person using Cursor
- **`agent_context`**: Technical details sent to the AI agent to help it understand and self-correct

**Example with separate messages:**

```rego
deny contains decision if {
    input.hook_event_name == "beforeShellExecution"
    contains(input.command, "rm -rf /")
    decision := {
        "rule_id": "SHELL-DANGER-001",
        "reason": "Dangerous command blocked for safety",  // User sees this
        "agent_context": "rm -rf / detected. This recursively deletes from root. Use 'trash' command or specify a subdirectory instead. Pattern matched: recursive force delete on root directory.",  // Agent sees this
        "severity": "CRITICAL"
    }
}
```

**If you omit `agent_context`**, the `reason` is used for both user and agent (default behavior).

**Why this matters:** Good agent feedback helps the LLM understand what went wrong and how to fix it, while keeping user messages concise and non-technical.

**Note**: Context injection when *allowing* actions is not supported by Cursor (only Claude Code supports this via `additionalContext`).

### Key Differences from Claude Code

Cursor policies access event data differently than Claude Code:

| Field | Claude Code | Cursor |
|-------|-------------|--------|
| Event type | `input.hook_event_name` (camelCase) | `input.hook_event_name` (camelCase) |
| Shell command | `input.tool_input.command` | `input.command` |
| File path | `input.tool_input.file_path` | `input.file_path` |
| File content | N/A | `input.file_content` |
| Prompt | `input.prompt` | `input.prompt` |

**Example: Same policy for both harnesses**

Claude Code version (`policies/claude/block_rm.rego`):
```rego
deny contains decision if {
    input.tool_name == "Bash"
    contains(input.tool_input.command, "rm -rf")
    decision := {...}
}
```

Cursor version (`policies/cursor/block_rm.rego`):
```rego
deny contains decision if {
    input.hook_event_name == "beforeShellExecution"
    contains(input.command, "rm -rf")
    decision := {...}
}
```

---

## Policy Examples

### 1. Block Shell Commands with `--no-verify`

```rego
# METADATA
# scope: package
# custom:
#   routing:
#     required_events: ["beforeShellExecution"]
package cursor.policies.builtins.git_block_no_verify

import rego.v1

deny contains decision if {
    input.hook_event_name == "beforeShellExecution"
    lower_cmd := lower(input.command)
    contains(lower_cmd, "git")
    contains(lower_cmd, "--no-verify")

    decision := {
        "rule_id": "GIT-NO-VERIFY",
        "reason": "Git operations with --no-verify bypass important checks",
        "severity": "HIGH"
    }
}
```

### 2. Protect Sensitive Files

```rego
# METADATA
# scope: package
# custom:
#   routing:
#     required_events: ["beforeReadFile"]
package cursor.policies.protect_secrets

import rego.v1

deny contains decision if {
    input.hook_event_name == "beforeReadFile"

    # List of sensitive file patterns
    sensitive_patterns := [
        ".env",
        ".aws/credentials",
        ".ssh/id_rsa",
        "secrets.yml"
    ]

    # Check if file path matches any pattern
    some pattern in sensitive_patterns
    contains(input.file_path, pattern)

    decision := {
        "rule_id": "FILE-PROTECT-001",
        "reason": concat("", ["Blocked access to sensitive file: ", pattern]),
        "severity": "CRITICAL"
    }
}
```

### 3. Validate File Edits (Post-Hook)

```rego
# METADATA
# scope: package
# custom:
#   routing:
#     required_events: ["afterFileEdit"]
package cursor.policies.validate_edits

import rego.v1

deny contains decision if {
    input.hook_event_name == "afterFileEdit"
    endswith(input.file_path, ".py")

    # Check if file content has syntax errors
    # (In practice, you'd use a signal to run pylint)
    not valid_python_syntax(input.file_content)

    decision := {
        "rule_id": "EDIT-VALIDATE-001",
        "reason": "Python file has syntax errors after edit",
        "severity": "HIGH"
    }
}

# Helper function (simplified example)
valid_python_syntax(content) if {
    # In real usage, trigger a signal that runs: python -m py_compile file.py
    true  # Placeholder
}
```

### 4. Filter Prompts for Compliance

```rego
# METADATA
# scope: package
# custom:
#   routing:
#     required_events: ["beforeSubmitPrompt"]
package cursor.policies.prompt_compliance

import rego.v1

deny contains decision if {
    input.hook_event_name == "beforeSubmitPrompt"

    # Block prompts that might leak proprietary information
    proprietary_terms := ["ACME Corp", "trade secret", "confidential"]

    some term in proprietary_terms
    contains(lower(input.prompt), lower(term))

    decision := {
        "rule_id": "PROMPT-COMPLIANCE-001",
        "reason": concat("", ["Prompt contains proprietary term: ", term]),
        "severity": "HIGH"
    }
}
```

---

## Response Formats

Cupcake translates policy decisions into Cursor's expected response format:

### Allow (Continue)
```json
{
  "permission": "allow"
}
```

### Deny (Block)
```json
{
  "permission": "deny",
  "userMessage": "Action blocked: Dangerous recursive delete command",
  "agentMessage": "Policy CURSOR-SHELL-001 blocked this command"
}
```

### Ask (Prompt User)
```json
{
  "permission": "ask",
  "userMessage": "Do you want to allow this git operation?",
  "agentMessage": "Policy requires confirmation for git commands",
  "question": "Allow git push to main branch?"
}
```

**Note**: `beforeSubmitPrompt` only supports `{"continue": true|false}` and cannot inject context.

---

## Built-in Policies

Cupcake includes several built-in policies for Cursor. Enable them in `.cupcake/rulebook.yml`:

```yaml
builtins:
  git_block_no_verify:
    enabled: true

  protected_paths:
    enabled: true
    paths:
      - ".env"
      - ".aws/credentials"
      - "secrets/"

  global_file_lock:
    enabled: true
    message: "All file modifications require approval"
```

See [Built-in Policies Reference](../policies/builtin-policies-reference.md) for complete list.

---

## Testing Your Policies

### 1. Test Locally with JSON

Create a test event file `test-event.json`:

```json
{
  "hook_event_name": "beforeShellExecution",
  "conversation_id": "test",
  "generation_id": "test",
  "workspace_roots": ["/tmp"],
  "command": "rm -rf /"
}
```

Run evaluation:

```bash
cupcake eval --harness cursor < test-event.json
```

Expected output:
```json
{
  "permission": "deny",
  "userMessage": "Dangerous recursive delete command blocked",
  "agentMessage": "Policy CURSOR-SHELL-001 blocked this command"
}
```

### 2. Enable Debug Mode

```bash
cupcake eval --harness cursor --debug-files < test-event.json
```

This creates `.cupcake/debug/` with detailed evaluation logs.

---

## Troubleshooting

### Cursor Isn't Calling Cupcake

**Check Cursor hooks:**
```bash
cat ~/.cursor/hooks.json
```

**Verify hook configuration:**
- Ensure `version` field is set to `1`
- Verify `hooks` object exists with event arrays
- Check that command paths are correct
- Ensure `--harness cursor` flag is present

### Policies Not Loading

**Verify policy directory:**
```bash
ls -la .cupcake/policies/cursor/
```

Policies must be in the `cursor/` subdirectory, not the root `policies/` directory.

**Check policy syntax:**
```bash
opa fmt --check .cupcake/policies/cursor/*.rego
```

### Permission Errors

Ensure Cupcake binary is executable:
```bash
chmod +x $(which cupcake)
```

---

## Security Disclaimer

**⚠️ USE AT YOUR OWN RISK**: Cursor hooks execute arbitrary shell commands on your system automatically. By using hooks, you acknowledge that:

- You are solely responsible for the commands you configure
- Hooks can modify, delete, or access any files your user account can access
- Malicious or poorly written hooks can cause data loss or system damage
- Cursor and its developers provide no warranty and assume no liability for any damages
- You should thoroughly test hooks in a safe environment before production use
- Cupcake policies add a layer of protection but are not foolproof

**IMPORTANT**: Always review and understand the policies you enable. Test thoroughly in isolated environments before deploying to production systems.

---

## Advanced Configuration

### Global Policies

Place policies in `~/.config/cupcake/policies/cursor/` (Linux/macOS) or `%APPDATA%\cupcake\policies\cursor\` (Windows) to apply them across all projects.

### Signals for Dynamic Data

Use signals to gather runtime information:

```yaml
# .cupcake/rulebook.yml
signals:
  python_syntax_check:
    command: "python -m py_compile {{ file_path }}"
    timeout_seconds: 5
```

Reference in policy:
```rego
deny contains decision if {
    input.hook_event_name == "afterFileEdit"
    endswith(input.file_path, ".py")

    # Signal returns { "exit_code": 0|1, "output": "...", "success": true|false }
    signal_result := input.signals.python_syntax_check
    signal_result.exit_code != 0

    decision := {
        "rule_id": "PY-SYNTAX",
        "reason": concat("", ["Syntax error: ", signal_result.error]),
        "severity": "HIGH"
    }
}
```

### Actions on Decisions

Execute actions when policies trigger:

```yaml
# .cupcake/rulebook.yml
actions:
  by_rule_id:
    CURSOR-SHELL-001:
      - command: "echo 'Dangerous command blocked' >> /var/log/cupcake.log"

  on_any_denial:
    - command: "notify-send 'Cupcake blocked an action'"
```

---

## Next Steps

- [Architecture: Harness Model](../architecture/harness-model.md) - Understand how harnesses work
- [Writing Policies](../policies/writing-policies.md) - Complete policy authoring guide
- [Built-in Policies Reference](../policies/builtin-policies-reference.md) - Available builtins
- [Signals](../configuration/signals.md) - Gather dynamic data for policies
- [Actions](../configuration/actions.md) - Execute commands on policy decisions

---

## Comparison with Claude Code

See [Harness Comparison Matrix](harness-comparison.md) for detailed differences between Cursor and Claude Code integration.
