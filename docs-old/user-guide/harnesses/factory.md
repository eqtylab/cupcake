# Factory AI Integration Guide

Cupcake provides comprehensive policy enforcement for [Factory AI](https://factory.ai) (Droid), the autonomous coding agent. This guide shows you how to set up and use Cupcake with Factory AI.

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
cupcake init --harness factory
```

This creates a `.cupcake/` directory with the following structure:

```
.cupcake/
├── policies/
│   ├── factory/          # Factory AI-specific policies
│   │   ├── system/
│   │   │   └── evaluate.rego
│   │   └── builtins/
│   └── claude/           # Claude Code policies (for comparison)
├── signals/
├── actions/
└── rulebook.yml
```

### 3. Configure Factory AI

The `init` command automatically configures Factory AI by adding hooks to `.factory/settings.json`:

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "*",
        "hooks": [
          {
            "type": "command",
            "command": "cupcake eval --harness factory --policy-dir \"$FACTORY_PROJECT_DIR\"/.cupcake"
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
            "command": "cupcake eval --harness factory --policy-dir \"$FACTORY_PROJECT_DIR\"/.cupcake"
          }
        ]
      }
    ],
    "UserPromptSubmit": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "cupcake eval --harness factory --policy-dir \"$FACTORY_PROJECT_DIR\"/.cupcake"
          }
        ]
      }
    ],
    "SessionStart": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "cupcake eval --harness factory --policy-dir \"$FACTORY_PROJECT_DIR\"/.cupcake"
          }
        ]
      }
    ],
    "Stop": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "cupcake eval --harness factory --policy-dir \"$FACTORY_PROJECT_DIR\"/.cupcake"
          }
        ]
      }
    ],
    "SubagentStop": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "cupcake eval --harness factory --policy-dir \"$FACTORY_PROJECT_DIR\"/.cupcake"
          }
        ]
      }
    ]
  }
}
```

### 4. Start Using Factory AI

Once configured, Cupcake will automatically evaluate all Factory AI actions against your policies. The agent will be blocked if policies deny the action.

---

## Understanding Factory AI Events

Factory AI provides hook events similar to Claude Code that Cupcake can intercept:

| Hook Event         | When It Fires                | Use Case                       |
| ------------------ | ---------------------------- | ------------------------------ |
| `PreToolUse`       | Before executing any tool    | Block dangerous operations     |
| `PostToolUse`      | After tool execution         | Validate results, run checks   |
| `UserPromptSubmit` | Before sending prompt to LLM | Filter prompts, inject context |
| `SessionStart`     | When session starts/resumes  | Load context, set environment  |
| `Stop`             | When agent stops             | Cleanup, logging               |
| `SubagentStop`     | When subagent completes      | Subagent coordination          |

### Event Data Structures

Each event has a specific JSON structure. Here are the key events:

#### PreToolUse (Shell Command)

```json
{
  "hook_event_name": "PreToolUse",
  "session_id": "session_abc123",
  "cwd": "/working/directory",
  "tool_name": "Bash",
  "tool_input": {
    "command": "git commit -m 'fix bug'"
  }
}
```

#### PreToolUse (File Edit)

```json
{
  "hook_event_name": "PreToolUse",
  "session_id": "session_abc123",
  "cwd": "/working/directory",
  "tool_name": "Edit",
  "tool_input": {
    "file_path": "/path/to/file.txt",
    "old_string": "original text",
    "new_string": "replacement text"
  }
}
```

#### PostToolUse (File Write)

```json
{
  "hook_event_name": "PostToolUse",
  "session_id": "session_abc123",
  "cwd": "/working/directory",
  "tool_name": "Write",
  "tool_input": {
    "file_path": "/path/to/file.txt",
    "content": "file contents..."
  },
  "tool_response": {
    "success": true
  }
}
```

#### UserPromptSubmit

```json
{
  "hook_event_name": "UserPromptSubmit",
  "session_id": "session_abc123",
  "cwd": "/working/directory",
  "prompt": "Help me implement authentication"
}
```

#### SessionStart

```json
{
  "hook_event_name": "SessionStart",
  "session_id": "session_abc123",
  "cwd": "/working/directory"
}
```

#### Stop / SubagentStop

```json
{
  "hook_event_name": "Stop",
  "session_id": "session_abc123",
  "cwd": "/working/directory"
}
```

---

## Writing Policies for Factory AI

Policies for Factory AI are written in Rego and placed in `.cupcake/policies/factory/`.

### Basic Policy Structure

```rego
# METADATA
# scope: package
# custom:
#   routing:
#     required_events: ["PreToolUse"]
#     required_tools: ["Bash"]
package cupcake.policies.builtins.block_dangerous_commands

import rego.v1

deny contains decision if {
    input.tool_name == "Bash"
    contains(input.tool_input.command, "rm -rf /")
    decision := {
        "rule_id": "FACTORY-SHELL-001",
        "reason": "Dangerous recursive delete command blocked",
        "severity": "CRITICAL"
    }
}
```

### Key Similarities with Claude Code

Factory AI uses the same event structure as Claude Code, making policy migration straightforward:

| Field         | Factory AI                   | Claude Code                  |
| ------------- | ---------------------------- | ---------------------------- |
| Event type    | `input.hook_event_name`      | `input.hook_event_name`      |
| Tool name     | `input.tool_name`            | `input.tool_name`            |
| Shell command | `input.tool_input.command`   | `input.tool_input.command`   |
| File path     | `input.tool_input.file_path` | `input.tool_input.file_path` |
| Prompt        | `input.prompt`               | `input.prompt`               |
| Session ID    | `input.session_id`           | `input.session_id`           |

**Shared policies**: Many policies can be used across both Factory AI and Claude Code without modification.

---

## Policy Examples

### 1. Block Shell Commands with `--no-verify`

```rego
# METADATA
# scope: package
# custom:
#   routing:
#     required_events: ["PreToolUse"]
#     required_tools: ["Bash"]
package cupcake.policies.builtins.git_block_no_verify

import rego.v1

deny contains decision if {
    input.tool_name == "Bash"
    lower_cmd := lower(input.tool_input.command)
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
#     required_events: ["PreToolUse"]
#     required_tools: ["Read"]
package cupcake.policies.protect_secrets

import rego.v1

deny contains decision if {
    input.tool_name == "Read"

    # List of sensitive file patterns
    sensitive_patterns := [
        ".env",
        ".aws/credentials",
        ".ssh/id_rsa",
        "secrets.yml"
    ]

    # Check if file path matches any pattern
    some pattern in sensitive_patterns
    contains(input.tool_input.file_path, pattern)

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
#     required_events: ["PostToolUse"]
#     required_tools: ["Write", "Edit"]
package cupcake.policies.validate_edits

import rego.v1

deny contains decision if {
    input.tool_name in ["Write", "Edit"]
    endswith(input.tool_input.file_path, ".py")

    # Check if write was successful
    input.tool_response.success

    # Use signal to run linting
    lint_result := input.signals.python_lint
    lint_result.exit_code != 0

    decision := {
        "rule_id": "EDIT-VALIDATE-001",
        "reason": concat("", ["Python lint failed: ", lint_result.error]),
        "severity": "HIGH"
    }
}
```

### 4. Filter Prompts for Compliance

```rego
# METADATA
# scope: package
# custom:
#   routing:
#     required_events: ["UserPromptSubmit"]
package cupcake.policies.prompt_compliance

import rego.v1

deny contains decision if {
    input.hook_event_name == "UserPromptSubmit"

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

### 5. Inject Context at Session Start

```rego
# METADATA
# scope: package
# custom:
#   routing:
#     required_events: ["SessionStart"]
package cupcake.policies.session_context

import rego.v1

add_context contains context if {
    input.hook_event_name == "SessionStart"

    # Load project-specific guidelines
    context := "Remember: Always run tests before committing. Use conventional commit messages."
}

add_context contains branch_context if {
    input.hook_event_name == "SessionStart"

    # Use signal to get current git branch
    branch := input.signals.git_branch.output
    contains(branch, "main")

    branch_context := "You're on the main branch. Be extra careful with changes."
}
```

---

## Response Formats

Cupcake translates policy decisions into Factory AI's expected response format:

### Allow (Continue)

```json
{
  "continue": true
}
```

### Deny (Block)

```json
{
  "continue": false,
  "stopReason": "Dangerous recursive delete command blocked"
}
```

### Allow with Context Injection

```json
{
  "continue": true,
  "hookSpecificOutput": {
    "additionalContext": "Remember to run tests before committing"
  }
}
```

**Context injection is supported on:**

- `UserPromptSubmit` - via `hookSpecificOutput.additionalContext`
- `SessionStart` - via `hookSpecificOutput.additionalContext`

### Ask (Prompt User)

```json
{
  "continue": false,
  "stopReason": "Policy requires confirmation",
  "hookSpecificOutput": {
    "permissionDecision": "ask",
    "permissionDecisionReason": "Do you want to allow this git push to main?"
  }
}
```

---

## Built-in Policies

Cupcake includes several built-in policies for Factory AI. Enable them in `.cupcake/rulebook.yml`:

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

  system_protection:
    enabled: true
    protected_dirs:
      - "/etc"
      - "/bin"
      - "/usr/bin"

  sensitive_data_protection:
    enabled: true

  cupcake_exec_protection:
    enabled: true

  factory_enforce_full_file_read:
    enabled: true
    max_lines: 2000
```

See [Built-in Policies Reference](../policies/builtin-policies-reference.md) for complete list.

---

## Testing Your Policies

### 1. Test Locally with JSON

Create a test event file `test-event.json`:

```json
{
  "hook_event_name": "PreToolUse",
  "session_id": "test_session",
  "cwd": "/tmp",
  "tool_name": "Bash",
  "tool_input": {
    "command": "rm -rf /"
  }
}
```

Run evaluation:

```bash
cupcake eval --harness factory < test-event.json
```

Expected output:

```json
{
  "continue": false,
  "stopReason": "Dangerous recursive delete command blocked"
}
```

### 2. Enable Debug Mode

```bash
cupcake eval --harness factory --debug-files < test-event.json
```

This creates `.cupcake/debug/` with detailed evaluation logs.

---

## Troubleshooting

### Factory AI Isn't Calling Cupcake

**Check Factory AI settings:**

```bash
cat .factory/settings.json | grep cupcake
```

**Verify hook configuration:**

- Ensure `hooks` object exists in settings
- Verify command path is correct
- Check that `--harness factory` flag is present
- Ensure hooks have proper structure with `matcher` and `hooks` array

### Policies Not Loading

**Verify policy directory:**

```bash
ls -la .cupcake/policies/factory/
```

Policies must be in the `factory/` subdirectory, not the root `policies/` directory.

**Check policy syntax:**

```bash
opa fmt --check .cupcake/policies/factory/*.rego
```

### Permission Errors

Ensure Cupcake binary is executable:

```bash
chmod +x $(which cupcake)
```

---

## Security Disclaimer

**USE AT YOUR OWN RISK**: Factory AI hooks execute arbitrary shell commands on your system automatically. By using hooks, you acknowledge that:

- You are solely responsible for the commands you configure
- Hooks can modify, delete, or access any files your user account can access
- Malicious or poorly written hooks can cause data loss or system damage
- You should thoroughly test hooks in a safe environment before production use
- Cupcake policies add a layer of protection but are not foolproof

**IMPORTANT**: Always review and understand the policies you enable. Test thoroughly in isolated environments before deploying to production systems.

---

## Advanced Configuration

### Global Policies

Place policies in `~/.config/cupcake/policies/factory/` (Linux/macOS) or `%APPDATA%\cupcake\policies\factory\` (Windows) to apply them across all projects.

Global policies are evaluated **first** and can enforce organization-wide rules.

### Signals for Dynamic Data

Use signals to gather runtime information:

```yaml
# .cupcake/rulebook.yml
signals:
  git_branch:
    command: "git rev-parse --abbrev-ref HEAD"
    timeout_seconds: 2

  python_lint:
    command: "pylint {{ file_path }}"
    timeout_seconds: 10
```

Reference in policy:

```rego
deny contains decision if {
    input.tool_name == "Bash"
    contains(input.tool_input.command, "git push")

    # Check if on main branch
    branch := input.signals.git_branch.output
    contains(branch, "main")

    decision := {
        "rule_id": "GIT-MAIN-PUSH",
        "reason": "Cannot push directly to main branch",
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
    FACTORY-SHELL-001:
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

## Comparison with Other Harnesses

See [Harness Comparison Matrix](harness-comparison.md) for detailed differences between Factory AI, Claude Code, Cursor, and OpenCode integration.
