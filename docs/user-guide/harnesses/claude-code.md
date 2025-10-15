# Claude Code Integration Guide

Cupcake provides comprehensive policy enforcement for [Claude Code](https://claude.ai/code), Anthropic's official CLI for Claude. This guide shows you how to set up and use Cupcake with Claude Code.

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
cupcake init --harness claude
```

This creates a `.cupcake/` directory with the following structure:

```
.cupcake/
├── policies/
│   ├── claude/          # Claude Code-specific policies
│   │   ├── system/
│   │   │   └── evaluate.rego
│   │   └── builtins/
│   └── cursor/          # Cursor policies (for comparison)
├── signals/
├── actions/
└── rulebook.yml
```

### 3. Configure Claude Code

The `init` command automatically configures Claude Code by adding hooks to `.claude/settings.json`:

```json
{
  "hooks": {
    "UserPromptSubmit": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "cupcake eval --harness claude"
          }
        ]
      }
    ],
    "PreToolUse": [
      {
        "matcher": "*",
        "hooks": [
          {
            "type": "command",
            "command": "cupcake eval --harness claude"
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
            "command": "cupcake eval --harness claude"
          }
        ]
      }
    ],
    "SessionStart": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "cupcake eval --harness claude"
          }
        ]
      }
    ]
  }
}
```

### 4. Start Using Claude Code

Once configured, Cupcake will automatically evaluate all Claude Code actions against your policies. The agent will be blocked if policies deny the action.

---

## Understanding Claude Code Events

Claude Code provides 5 primary hook events that Cupcake can intercept:

| Hook Event | When It Fires | Use Case |
|------------|---------------|----------|
| `UserPromptSubmit` | Before sending prompt to LLM | Filter prompts, inject context |
| `PreToolUse` | Before executing any tool | Block dangerous operations |
| `PostToolUse` | After tool execution | Validate results, run checks |
| `SessionStart` | When session starts/resumes | Load context, set environment |
| `PreCompact` | Before compacting conversation | Inject context before compression |

**Additional Events** (less common):
- `SessionEnd` - When session ends
- `Stop` - When agent stops
- `SubagentStop` - When subagent completes
- `Notification` - On system notifications

### Event Data Structures

Each event has a specific JSON structure. Here are the key events:

#### UserPromptSubmit

```json
{
  "hook_event_name": "UserPromptSubmit",
  "session_id": "session_abc123",
  "transcript_path": "/path/to/transcript.jsonl",
  "cwd": "/working/directory",
  "prompt": "Help me implement authentication"
}
```

#### PreToolUse (Shell Command)

```json
{
  "hook_event_name": "PreToolUse",
  "session_id": "session_abc123",
  "transcript_path": "/path/to/transcript.jsonl",
  "cwd": "/working/directory",
  "tool_name": "Bash",
  "tool_input": {
    "command": "git commit -m 'fix bug'"
  }
}
```

**Deprecation Note**: The following fields are deprecated and will be removed in a future version:
- `blocking_rate_limit_sleep` - No longer used for rate limiting
- `hook_user_id` - Replaced by session-based identification
- `subagent_session_id` - Subagent context is now handled differently

These fields may still appear in events for backward compatibility but should not be relied upon in new policies.

#### PreToolUse (File Read)

```json
{
  "hook_event_name": "PreToolUse",
  "session_id": "session_abc123",
  "transcript_path": "/path/to/transcript.jsonl",
  "cwd": "/working/directory",
  "tool_name": "Read",
  "tool_input": {
    "file_path": "/path/to/file.txt"
  }
}
```

#### PostToolUse (File Write)

```json
{
  "hook_event_name": "PostToolUse",
  "session_id": "session_abc123",
  "transcript_path": "/path/to/transcript.jsonl",
  "cwd": "/working/directory",
  "tool_name": "Write",
  "tool_input": {
    "file_path": "/path/to/file.txt",
    "content": "file contents..."
  },
  "tool_response": {
    "filePath": "/path/to/file.txt",
    "success": true
  }
}
```

#### SessionStart

```json
{
  "hook_event_name": "SessionStart",
  "session_id": "session_abc123",
  "transcript_path": "/path/to/transcript.jsonl",
  "cwd": "/working/directory",
  "source": "startup"
}
```

**Source Values**: `startup`, `resume`, `clear`, `compact`

#### PreCompact

```json
{
  "hook_event_name": "PreCompact",
  "session_id": "session_abc123",
  "transcript_path": "/path/to/transcript.jsonl",
  "cwd": "/working/directory",
  "trigger": "automatic",
  "custom_instructions": "Remember to follow the style guide"
}
```

**Trigger Values**: `automatic`, `manual`

#### SessionEnd

```json
{
  "hook_event_name": "SessionEnd",
  "session_id": "session_abc123",
  "transcript_path": "/path/to/transcript.jsonl",
  "cwd": "/working/directory",
  "reason": "clear"
}
```

**Reason Values**: `clear`, `logout`, `prompt_input_exit`, `other`

#### Stop / SubagentStop

```json
{
  "hook_event_name": "Stop",
  "session_id": "session_abc123",
  "transcript_path": "/path/to/transcript.jsonl",
  "cwd": "/working/directory",
  "stop_hook_active": true
}
```

**Note**: `stop_hook_active` indicates whether a stop hook is currently processing (used to prevent infinite loops).

---

## Writing Policies for Claude Code

Policies for Claude Code are written in Rego and placed in `.cupcake/policies/claude/`.

### Basic Policy Structure

```rego
# METADATA
# scope: package
# custom:
#   routing:
#     required_events: ["PreToolUse"]
#     required_tools: ["Bash"]
package cupcake.policies.block_dangerous_commands

import rego.v1

deny contains decision if {
    input.tool_name == "Bash"
    contains(input.tool_input.command, "rm -rf /")
    decision := {
        "rule_id": "CLAUDE-SHELL-001",
        "reason": "Dangerous recursive delete command blocked",
        "severity": "CRITICAL"
    }
}
```

### Key Differences from Cursor

Claude Code policies access event data differently than Cursor:

| Field | Claude Code | Cursor |
|-------|-------------|--------|
| Event type | `input.hook_event_name` | `input.hook_event_name` |
| Shell command | `input.tool_input.command` | `input.command` |
| File path | `input.tool_input.file_path` | `input.file_path` |
| File content | Via tool_input (Write) or tool_response (Read) | `input.file_content` |
| Prompt | `input.prompt` | `input.prompt` |
| Session ID | `input.session_id` | `input.conversation_id` |

**Example: Same policy for both harnesses**

Cursor version (`policies/cursor/block_rm.rego`):
```rego
deny contains decision if {
    input.hook_event_name == "beforeShellExecution"
    contains(input.command, "rm -rf")
    decision := {...}
}
```

Claude Code version (`policies/claude/block_rm.rego`):
```rego
deny contains decision if {
    input.tool_name == "Bash"
    contains(input.tool_input.command, "rm -rf")
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

    # In practice, use a signal to run linting
    # For this example, checking basic conditions
    decision := {
        "rule_id": "EDIT-VALIDATE-001",
        "reason": "Python file validation required after edit",
        "severity": "MEDIUM"
    }
}

# Helper: Trigger lint signal for validation
# In rulebook.yml:
# signals:
#   python_lint:
#     command: "pylint {{ file_path }}"
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
    input.source == "startup"

    # Load project-specific guidelines
    context := "Remember: Always run tests before committing. Use conventional commit messages."
}

add_context contains branch_context if {
    input.hook_event_name == "SessionStart"

    # Use signal to get current git branch
    branch := input.signals.git_branch.output
    contains(branch, "main")

    branch_context := "⚠️ You're on the main branch. Be extra careful with changes."
}
```

### 6. Prevent Agent Stoppage on Important Tasks

```rego
# METADATA
# scope: package
# custom:
#   routing:
#     required_events: ["Stop"]
package cupcake.policies.prevent_premature_stop

import rego.v1

ask contains decision if {
    input.hook_event_name == "Stop"

    # Check if tests are running (via signal)
    tests_running := input.signals.check_tests.exit_code != 0

    tests_running

    decision := {
        "rule_id": "STOP-PREVENT-001",
        "reason": "Tests are still running",
        "question": "Tests haven't completed. Stop anyway?",
        "severity": "MEDIUM"
    }
}
```

---

## Response Formats

Cupcake translates policy decisions into Claude Code's expected response format:

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

**Note**: Context injection is supported on:
- `UserPromptSubmit` - via `hookSpecificOutput.additionalContext`
- `SessionStart` - via `hookSpecificOutput.additionalContext`
- `PreCompact` - joins with `\n\n` (double newline)

**Not supported on**: `PreToolUse` (cannot inject context during tool execution)

**Cursor comparison**: Cursor does not support context injection at all. The `add_context` verb only affects Claude Code. However, Cursor does support separate `userMessage` and `agentMessage` fields when *blocking* actions (see Cursor guide for details).

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

### Hook Blocking Behavior

When Cupcake policies return a deny/block decision, the behavior varies by Claude Code event type:

| Hook Event         | Can Block? | What Happens When Blocked |
| ------------------ | ---------- | -------------------------- |
| `PreToolUse`       | ✅ Yes     | Tool doesn't execute, agent sees feedback |
| `PostToolUse`      | ⚠️ Partial | Tool already ran, but agent sees feedback |
| `UserPromptSubmit` | ✅ Yes     | Prompt blocked, user sees reason |
| `Stop`             | ✅ Yes     | Agent continues working |
| `SubagentStop`     | ✅ Yes     | Subagent continues working |
| `PreCompact`       | ❌ No      | Informational only |
| `SessionStart`     | ❌ No      | Informational only |
| `SessionEnd`       | ❌ No      | Informational only |
| `Notification`     | ❌ No      | Informational only |

This affects how you design policies - blocking policies only make sense for events that can actually prevent actions.

### Exit Code 2 Behavior

When a hook script exits with code 2 (error), Claude Code handles it differently based on the event type:

| Hook Event         | Behavior                                                           |
| ------------------ | ------------------------------------------------------------------ |
| `PreToolUse`       | Blocks the tool call, shows stderr to Claude                      |
| `PostToolUse`      | Shows stderr to Claude (tool already ran)                         |
| `UserPromptSubmit` | Blocks the prompt, shows error to user                            |
| `SessionStart`     | Shows stderr to Claude (session continues)                        |
| `PreCompact`       | Shows stderr to Claude (compaction continues)                     |
| `SessionEnd`       | Logs error (session already ended)                                |
| `Stop`             | Shows stderr to Claude (stop can be prevented if hook blocks)     |
| `SubagentStop`     | Shows stderr to Claude (subagent stop can be prevented if hook blocks) |
| `Notification`     | Logs error (informational only)                                   |

**Important**: Exit code 2 indicates a hook execution error, not a policy decision. Use proper JSON responses for policy decisions.

---

## Built-in Policies

Cupcake includes several built-in policies for Claude Code. Enable them in `.cupcake/rulebook.yml`:

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

  enforce_full_file_read:
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
  "transcript_path": "/tmp/transcript.jsonl",
  "cwd": "/tmp",
  "tool_name": "Bash",
  "tool_input": {
    "command": "rm -rf /"
  }
}
```

Run evaluation:

```bash
cupcake eval --harness claude < test-event.json
```

Expected output:

```json
{
  "continue": false,
  "stopReason": "Dangerous recursive delete command blocked"
}
```

### 2. Test with Claude Code CLI

```bash
# Use the --debug flag to see hook execution
claude -p "hello world" --debug
```

Check `.cupcake/debug/` for detailed evaluation logs.

### 3. Enable Debug Mode

```bash
cupcake eval --harness claude --debug-files < test-event.json
```

This creates `.cupcake/debug/` with detailed evaluation logs.

---

## Troubleshooting

### Claude Code Isn't Calling Cupcake

**Check Claude Code settings:**
```bash
cat .claude/settings.json | grep cupcake
```

**Verify hook configuration:**
- Ensure `hooks` object exists in settings
- Verify command path is correct
- Check that `--harness claude` flag is present
- Ensure hooks have proper structure with `matcher` and `hooks` array

### Policies Not Loading

**Verify policy directory:**
```bash
ls -la .cupcake/policies/claude/
```

Policies must be in the `claude/` subdirectory, not the root `policies/` directory.

**Check policy syntax:**
```bash
opa fmt --check .cupcake/policies/claude/*.rego
```

### Hook Not Firing for Specific Tools

**Check matcher configuration:**

```json
{
  "PreToolUse": [
    {
      "matcher": "Bash",  // Exact match
      "hooks": [...]
    }
  ]
}
```

**Matcher patterns:**
- `"Bash"` - Exact tool name
- `"Write|Edit"` - Multiple tools
- `"mcp__memory__.*"` - MCP tools with regex
- `"*"` or `""` - All tools (wildcard)

### Context Not Injecting

Context injection only works on specific events:
- ✅ `UserPromptSubmit`
- ✅ `SessionStart`
- ✅ `PreCompact`
- ❌ `PreToolUse` (not supported)
- ❌ `PostToolUse` (not supported for context)

### Permission Errors

Ensure Cupcake binary is executable:
```bash
chmod +x $(which cupcake)
```

---

## Security Disclaimer

**⚠️ USE AT YOUR OWN RISK**: Claude Code hooks execute arbitrary shell commands on your system automatically. By using hooks, you acknowledge that:

- You are solely responsible for the commands you configure
- Hooks can modify, delete, or access any files your user account can access
- Malicious or poorly written hooks can cause data loss or system damage
- Anthropic provides no warranty and assumes no liability for any damages
- You should thoroughly test hooks in a safe environment before production use
- Cupcake policies add a layer of protection but are not foolproof

**IMPORTANT**: Always review and understand the policies you enable. Test thoroughly in isolated environments before deploying to production systems.

---

## Advanced Configuration

### Global Policies

Place policies in `~/.config/cupcake/policies/claude/` (Linux/macOS) or `%APPDATA%\cupcake\policies\claude\` (Windows) to apply them across all projects.

Global policies are evaluated **first** and can enforce organization-wide rules.

### Signals for Dynamic Data

Use signals to gather runtime information:

```yaml
# .cupcake/rulebook.yml
signals:
  git_branch:
    command: "git rev-parse --abbrev-ref HEAD"
    timeout_seconds: 2

  check_tests:
    command: "pgrep -f pytest || pgrep -f 'cargo test'"
    timeout_seconds: 1
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
    CLAUDE-SHELL-001:
      - command: "echo 'Dangerous command blocked' >> /var/log/cupcake.log"

  on_any_denial:
    - command: "notify-send 'Cupcake blocked an action'"
```

### Tool Matchers

Claude Code uses regex matchers for flexible tool selection:

```json
{
  "PreToolUse": [
    {
      "matcher": "Write|Edit|MultiEdit",
      "hooks": [...]
    },
    {
      "matcher": "mcp__.*",
      "hooks": [...]
    },
    {
      "matcher": "Notebook.*",
      "hooks": [...]
    }
  ]
}
```

### MCP Tool Control

Control MCP (Model Context Protocol) tools:

```rego
# METADATA
# scope: package
# custom:
#   routing:
#     required_events: ["PreToolUse"]
#     required_tools: ["mcp__memory__.*"]
package cupcake.policies.mcp_control

import rego.v1

deny contains decision if {
    # Match MCP memory tools
    startswith(input.tool_name, "mcp__memory__")

    # Block write operations
    contains(input.tool_name, "write")

    decision := {
        "rule_id": "MCP-MEMORY-001",
        "reason": "MCP memory write operations are disabled",
        "severity": "HIGH"
    }
}
```

---

## Next Steps

- [Architecture: Harness Model](../architecture/harness-model.md) - Understand how harnesses work
- [Writing Policies](../policies/writing-policies.md) - Complete policy authoring guide
- [Built-in Policies Reference](../policies/builtin-policies-reference.md) - Available builtins
- [Signals](../configuration/signals.md) - Gather dynamic data for policies
- [Actions](../configuration/actions.md) - Execute commands on policy decisions

---

## Comparison with Cursor

See [Harness Comparison Matrix](harness-comparison.md) for detailed differences between Claude Code and Cursor integration.

---

## Advanced Topics

### Session Lifecycle Management

```rego
# Load company guidelines at session start
add_context contains company_policy if {
    input.hook_event_name == "SessionStart"
    input.source in ["startup", "resume"]

    company_policy := "Company Policy: All code changes require tests. Use trunk-based development."
}

# Log session end for analytics
add_context contains session_log if {
    input.hook_event_name == "SessionEnd"

    # Trigger logging action
    session_log := ""
}
```

### Preventing Agent Interruption

```rego
# Block Stop event when critical work is in progress
ask contains decision if {
    input.hook_event_name == "Stop"

    # Check if in critical section (via signal or state)
    in_critical_section := check_critical_work()

    in_critical_section

    decision := {
        "rule_id": "STOP-CRITICAL",
        "reason": "Critical operation in progress",
        "question": "A critical operation is running. Stop anyway?",
        "severity": "HIGH"
    }
}
```

### Pre-Compaction Context Injection

```rego
# Add context before conversation compaction
add_context contains compact_reminder if {
    input.hook_event_name == "PreCompact"

    compact_reminder := "Important: The project uses React 18 with TypeScript. Tests must pass before deployment."
}
```

### Tool-Specific Routing

Policies automatically route to matching tools via metadata:

```rego
# METADATA
# scope: package
# custom:
#   routing:
#     required_events: ["PreToolUse"]
#     required_tools: ["Bash"]
# This policy ONLY evaluates for Bash tool use
```

Without `required_tools`, the policy becomes a wildcard matching all tools for that event.

---

## Integration Patterns

### Pattern 1: Pre/Post Validation

```rego
# Pre-validation: Check before file write
deny contains decision if {
    input.tool_name == "Write"
    input.hook_event_name == "PreToolUse"
    endswith(input.tool_input.file_path, ".json")

    # Validate JSON structure before write
    not valid_json(input.tool_input.content)

    decision := {
        "rule_id": "JSON-PRE-VALIDATE",
        "reason": "Invalid JSON structure",
        "severity": "HIGH"
    }
}

# Post-validation: Check after file write
deny contains decision if {
    input.tool_name == "Write"
    input.hook_event_name == "PostToolUse"
    endswith(input.tool_input.file_path, ".json")

    # Run linter via signal after write
    lint_result := input.signals.json_lint.exit_code
    lint_result != 0

    decision := {
        "rule_id": "JSON-POST-VALIDATE",
        "reason": concat("", ["JSON lint failed: ", input.signals.json_lint.error]),
        "severity": "HIGH"
    }
}
```

### Pattern 2: Context-Aware Blocking

```rego
deny contains decision if {
    input.tool_name == "Bash"
    contains(input.tool_input.command, "npm publish")

    # Check if on correct branch
    branch := input.signals.git_branch.output
    not contains(branch, "release")

    decision := {
        "rule_id": "NPM-PUBLISH-BRANCH",
        "reason": "npm publish only allowed from release branches",
        "severity": "CRITICAL"
    }
}
```

### Pattern 3: Progressive Warnings

```rego
# Warn first
ask contains decision if {
    input.tool_name == "Bash"
    contains(input.tool_input.command, "git push --force")

    # Not on feature branch
    branch := input.signals.git_branch.output
    not startswith(branch, "feature/")

    decision := {
        "rule_id": "GIT-FORCE-PUSH",
        "reason": "Force push detected",
        "question": "Force push can rewrite history. Continue?",
        "severity": "HIGH"
    }
}
```
