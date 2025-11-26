# Harness Comparison Matrix

This guide compares Cupcake's support for different AI coding agents (harnesses), helping you understand their capabilities, differences, and how to write policies for each.

## Supported Harnesses

| Harness         | Status             | Description                                          |
| --------------- | ------------------ | ---------------------------------------------------- |
| **Claude Code** | ✅ Fully Supported | Anthropic's official CLI for Claude (claude.ai/code) |
| **Cursor**      | ✅ Fully Supported | AI-powered code editor (cursor.com)                  |
| **Factory AI**  | ✅ Fully Supported | Autonomous coding agent (factory.ai)                 |
| **OpenCode**    | ✅ Fully Supported | Open-source AI coding assistant (opencode.ai)        |

---

## Quick Comparison

| Feature                 | Claude Code              | Cursor                           | Factory AI               | OpenCode          |
| ----------------------- | ------------------------ | -------------------------------- | ------------------------ | ----------------- |
| **Hook Events**         | 5 events                 | 6 events                         | 6 events                 | 2 events          |
| **File Content Access** | Limited                  | Full access via `beforeReadFile` | Limited                  | Limited           |
| **Prompt Filtering**    | Yes (UserPromptSubmit)   | Yes (beforeSubmitPrompt)         | Yes (UserPromptSubmit)   | No                |
| **MCP Tool Control**    | No                       | Yes (beforeMCPExecution)         | No                       | No                |
| **Post-Action Hooks**   | Yes (PostToolUse)        | Yes (afterFileEdit)              | Yes (PostToolUse)        | Yes (PostToolUse) |
| **Context Injection**   | Yes (persistent context) | No¹                              | Yes                      | No                |
| **Configuration File**  | `.claude/settings.json`  | `~/.cursor/hooks.json`           | `.factory/settings.json` | Plugin-based      |

¹ Cursor's `agentMessage` provides agent-specific feedback when blocking, but does not support context injection.

---

## Event Comparison

### Claude Code Events

| Event Name         | When It Fires                  | Can Block | Can Add Context |
| ------------------ | ------------------------------ | --------- | --------------- |
| `UserPromptSubmit` | Before sending prompt to LLM   | ✅ Yes    | ✅ Yes          |
| `PreToolUse`       | Before executing any tool      | ✅ Yes    | ❌ No           |
| `PostToolUse`      | After tool execution           | ✅ Yes    | ✅ Yes          |
| `SessionStart`     | When session starts/resumes    | ✅ Yes    | ✅ Yes          |
| `PreCompact`       | Before compacting conversation | ❌ No     | ✅ Yes          |

**Total**: 5 events

### Cursor Events

| Event Name             | When It Fires                 | Can Block | Can Add Context |
| ---------------------- | ----------------------------- | --------- | --------------- |
| `beforeSubmitPrompt`   | Before sending prompt to LLM  | ✅ Yes    | ❌ No           |
| `beforeShellExecution` | Before running shell commands | ✅ Yes    | ❌ No           |
| `beforeMCPExecution`   | Before calling MCP tools      | ✅ Yes    | ❌ No           |
| `beforeReadFile`       | Before reading file contents  | ✅ Yes    | ❌ No           |
| `afterFileEdit`        | After file modifications      | ❌ No     | ❌ No           |
| `stop`                 | When agent stops              | ❌ No     | ❌ No           |

**Total**: 6 events

---

## Event Data Structure Comparison

### Shell Command Execution

**Claude Code** (PreToolUse with Bash tool):

```json
{
  "hook_event_name": "PreToolUse",
  "tool_name": "Bash",
  "tool_input": {
    "command": "git commit -m 'fix'"
  },
  "session_id": "session_123",
  "transcript_path": "/path/to/transcript",
  "cwd": "/working/dir"
}
```

**Cursor** (beforeShellExecution):

```json
{
  "hook_event_name": "beforeShellExecution",
  "command": "git commit -m 'fix'",
  "conversation_id": "conv_123",
  "generation_id": "gen_456",
  "workspace_roots": ["/working/dir"]
}
```

### File Access

**Claude Code** (PreToolUse with Read tool):

```json
{
  "hook_event_name": "PreToolUse",
  "tool_name": "Read",
  "tool_input": {
    "file_path": "/path/to/file.txt"
  },
  "session_id": "session_123",
  "transcript_path": "/path/to/transcript",
  "cwd": "/working/dir"
}
```

**Cursor** (beforeReadFile):

```json
{
  "hook_event_name": "beforeReadFile",
  "file_path": "/path/to/file.txt",
  "content": "actual file contents here...",
  "conversation_id": "conv_123",
  "generation_id": "gen_456",
  "workspace_roots": ["/working/dir"],
  "attachments": []
}
```

**Key Difference**: Cursor provides full `content` field in the event, enabling content-based policies without signals.

### Prompt Submission

**Claude Code** (UserPromptSubmit):

```json
{
  "hook_event_name": "UserPromptSubmit",
  "prompt": "Help me implement authentication",
  "session_id": "session_123",
  "transcript_path": "/path/to/transcript",
  "cwd": "/working/dir"
}
```

**Cursor** (beforeSubmitPrompt):

```json
{
  "hook_event_name": "beforeSubmitPrompt",
  "prompt": "Help me implement authentication",
  "conversation_id": "conv_123",
  "generation_id": "gen_456",
  "workspace_roots": ["/working/dir"]
}
```

---

## Policy Field Access Comparison

### Accessing Shell Commands

**Claude Code**:

```rego
deny contains decision if {
    input.tool_name == "Bash"
    contains(input.tool_input.command, "rm -rf")
    decision := {...}
}
```

**Cursor**:

```rego
deny contains decision if {
    input.hook_event_name == "beforeShellExecution"
    contains(input.command, "rm -rf")
    decision := {...}
}
```

### Accessing File Paths

**Claude Code**:

```rego
deny contains decision if {
    input.tool_name == "Read"
    contains(input.tool_input.file_path, ".env")
    decision := {...}
}
```

**Cursor**:

```rego
deny contains decision if {
    input.hook_event_name == "beforeReadFile"
    contains(input.file_path, ".env")
    decision := {...}
}
```

### Accessing Prompts

**Claude Code**:

```rego
deny contains decision if {
    input.hook_event_name == "UserPromptSubmit"
    contains(input.prompt, "proprietary")
    decision := {...}
}
```

**Cursor**:

```rego
deny contains decision if {
    input.hook_event_name == "beforeSubmitPrompt"
    contains(input.prompt, "proprietary")
    decision := {...}
}
```

---

## Response Format Comparison

### Allow Response

**Claude Code**:

```json
{
  "continue": true
}
```

**Cursor**:

```json
{
  "permission": "allow"
}
```

### Deny Response

**Claude Code**:

```json
{
  "continue": false,
  "stopReason": "Dangerous command blocked"
}
```

**Cursor**:

```json
{
  "permission": "deny",
  "userMessage": "Dangerous command blocked",
  "agentMessage": "Policy CURSOR-SHELL-001 blocked this command"
}
```

### Context Injection (Allow with Context)

**Claude Code**:

```json
{
  "continue": true,
  "hookSpecificOutput": {
    "additionalContext": "Remember to run tests before committing"
  }
}
```

**Cursor**:
Context injection is not supported in Cursor hooks. You can only provide `userMessage` and `agentMessage` fields for informational purposes.

---

## Configuration File Comparison

### Claude Code Configuration

**File**: `.claude/settings.json` (project-specific)

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

### Cursor Configuration

**File**: `~/.cursor/hooks.json` (global)

```json
{
  "version": 1,
  "hooks": {
    "beforeShellExecution": [{ "command": "cupcake eval --harness cursor" }],
    "beforeMCPExecution": [{ "command": "cupcake eval --harness cursor" }],
    "afterFileEdit": [{ "command": "cupcake eval --harness cursor" }],
    "beforeReadFile": [{ "command": "cupcake eval --harness cursor" }],
    "beforeSubmitPrompt": [{ "command": "cupcake eval --harness cursor" }],
    "stop": [{ "command": "cupcake eval --harness cursor" }]
  }
}
```

**Key Difference**: Claude Code uses project-specific configuration, Cursor uses global machine-wide configuration.

---

## Feature Matrix

### Context Injection Support

| Event                  | Claude Code        | Cursor |
| ---------------------- | ------------------ | ------ |
| Prompt submission      | ✅ Yes             | ❌ No  |
| Tool/command execution | ❌ No (PreToolUse) | ❌ No  |
| Post-execution hooks   | ✅ Yes             | ❌ No  |
| Session start          | ✅ Yes             | N/A    |
| Stop/cleanup           | N/A                | ❌ No  |

### File Access Capabilities

| Capability                    | Claude Code              | Cursor                            |
| ----------------------------- | ------------------------ | --------------------------------- |
| Block file reads              | ✅ Yes (via PreToolUse)  | ✅ Yes (via beforeReadFile)       |
| Block file writes             | ✅ Yes (via PreToolUse)  | ✅ Yes (via beforeShellExecution) |
| Access file content in policy | ❌ No (needs signal)     | ✅ Yes (via `content` field)      |
| Post-edit validation          | ✅ Yes (via PostToolUse) | ✅ Yes (via afterFileEdit)        |

### MCP Tool Control

| Feature                | Claude Code | Cursor                      |
| ---------------------- | ----------- | --------------------------- |
| Hook MCP calls         | ❌ No       | ✅ Yes (beforeMCPExecution) |
| Block MCP tools        | ❌ No       | ✅ Yes                      |
| Inspect MCP parameters | ❌ No       | ✅ Yes                      |

---

## Policy Portability

### Cross-Harness Policies with Shared Logic

To write policies that work across both harnesses, use the shared module pattern:

**Common Logic** (`.cupcake/policies/common/dangerous_commands.rego`):

```rego
package common.dangerous_commands

import rego.v1

is_dangerous_rm(cmd) {
    contains(lower(cmd), "rm")
    contains(cmd, "-rf")
    regex.match(`(/|~|\$HOME)`, cmd)
}
```

**Claude Code Policy** (`.cupcake/policies/claude/block_rm.rego`):

```rego
package cupcake.policies.block_rm

import rego.v1
import data.common.dangerous_commands.is_dangerous_rm

deny contains decision if {
    input.tool_name == "Bash"
    is_dangerous_rm(input.tool_input.command)
    decision := {
        "rule_id": "DANGEROUS-RM",
        "reason": "Dangerous recursive delete command blocked",
        "severity": "CRITICAL"
    }
}
```

**Cursor Policy** (`.cupcake/policies/cursor/block_rm.rego`):

```rego
package cupcake.policies.cursor.block_rm

import rego.v1
import data.common.dangerous_commands.is_dangerous_rm

deny contains decision if {
    input.hook_event_name == "beforeShellExecution"
    is_dangerous_rm(input.command)
    decision := {
        "rule_id": "DANGEROUS-RM",
        "reason": "Dangerous recursive delete command blocked",
        "severity": "CRITICAL"
    }
}
```

### Portability Table

| Aspect          | Portability Level | Notes                        |
| --------------- | ----------------- | ---------------------------- |
| Business logic  | ✅ High           | Extract to `common/` modules |
| Event routing   | ❌ Low            | Harness-specific event names |
| Field access    | ❌ Low            | Different field structures   |
| Response format | ✅ High           | Engine handles translation   |
| Signals         | ✅ High           | Same signal API for both     |
| Actions         | ✅ High           | Same action system for both  |

---

## Built-in Policy Support

Most built-in policies are implemented for both harnesses with harness-specific logic:

| Builtin                               | Claude Code | Cursor              | Notes                           |
| ------------------------------------- | ----------- | ------------------- | ------------------------------- |
| `git_block_no_verify`                 | ✅ Yes      | ✅ Yes              | Blocks git --no-verify          |
| `protected_paths`                     | ✅ Yes      | ✅ Yes              | Protects sensitive files        |
| `system_protection`                   | ✅ Yes      | ✅ Yes              | Protects system directories     |
| `sensitive_data_protection`           | ✅ Yes      | ✅ Yes              | Blocks SSH keys, credentials    |
| `cupcake_exec_protection`             | ✅ Yes      | ✅ Yes              | Prevents cupcake manipulation   |
| `global_file_lock`                    | ✅ Yes      | ✅ Yes              | Blocks all file modifications   |
| `claude_code_enforce_full_file_read`  | ✅ Yes      | ❌ No (Claude-only) | Requires full file reads        |
| `claude_code_always_inject_on_prompt` | ✅ Yes      | ❌ No (Claude-only) | Adds context to prompts         |
| `git_pre_check`                       | ✅ Yes      | ✅ Yes              | Validates before git operations |
| `post_edit_check`                     | ✅ Yes      | ✅ Yes              | Validates after edits           |
| `rulebook_security_guardrails`        | ✅ Yes      | ✅ Yes              | Protects .cupcake directory     |

All builtins are configured identically in `.cupcake/rulebook.yml` regardless of harness. Claude-specific builtins are ignored when using Cursor.

---

## Use Case Recommendations

### Choose Claude Code If:

- ✅ You primarily use Claude for AI assistance
- ✅ You want project-specific hook configuration
- ✅ You need tight integration with Claude's conversation transcript
- ✅ You prefer CLI-based workflows
- ✅ You need to control tool use comprehensively (all tools, not just shell)

### Choose Cursor If:

- ✅ You use Cursor as your primary code editor
- ✅ You need content-based file policies (access to `content`)
- ✅ You want to control MCP tool execution
- ✅ You prefer IDE-integrated AI workflows
- ✅ You need global hook configuration across all projects

### Use Both If:

- ✅ Your team uses mixed tooling
- ✅ You want consistent security policies across tools
- ✅ You're evaluating different AI coding agents
- ✅ You have different policies for different contexts (CLI vs IDE)

---

## Migration Between Harnesses

### Converting Policies from Claude Code to Cursor

**Step 1**: Copy policy to cursor directory

```bash
cp .cupcake/policies/claude/my_policy.rego \
   .cupcake/policies/cursor/my_policy.rego
```

**Step 2**: Update package name

```rego
# Before (Claude Code)
package cupcake.policies.my_policy

# After (Cursor)
package cupcake.policies.cursor.my_policy
```

**Step 3**: Update event and field access

```rego
# Before (Claude Code)
deny contains decision if {
    input.tool_name == "Bash"
    contains(input.tool_input.command, "pattern")
    ...
}

# After (Cursor)
deny contains decision if {
    input.hook_event_name == "beforeShellExecution"
    contains(input.command, "pattern")
    ...
}
```

**Step 4**: Update metadata routing

```rego
# Before (Claude Code)
# METADATA
# custom:
#   routing:
#     required_events: ["PreToolUse"]
#     required_tools: ["Bash"]

# After (Cursor)
# METADATA
# custom:
#   routing:
#     required_events: ["beforeShellExecution"]
```

---

## Performance Characteristics

| Aspect            | Claude Code | Cursor | Notes                               |
| ----------------- | ----------- | ------ | ----------------------------------- |
| Policy evaluation | ~1-2ms      | ~1-2ms | Both use WASM (similar performance) |
| Context injection | ~0.5ms      | ~0.5ms | JSON serialization overhead         |
| Signal execution  | Varies      | Varies | Depends on signal command           |
| Startup time      | ~50ms       | ~50ms  | Engine initialization               |
| Memory usage      | ~10MB       | ~10MB  | WASM runtime overhead               |

Both harnesses have identical performance characteristics because they share the same evaluation engine.

---

## Debugging and Troubleshooting

### Enable Debug Output

**Claude Code**:

```json
{
  "hooks": {
    "UserPromptSubmit": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "cupcake eval --harness claude --debug-files"
          }
        ]
      }
    ]
  }
}
```

**Cursor**:

```json
{
  "version": 1,
  "hooks": {
    "beforeShellExecution": [
      {
        "command": "cupcake eval --harness cursor --debug-files"
      }
    ]
  }
}
```

Debug files are written to `.cupcake/debug/` in both cases.

### Common Issues by Harness

**Claude Code**:

- Hook not firing: Check `.claude/settings.json` exists and is valid JSON
- Wrong event structure: Ensure `hook_event_name` matches Claude Code format
- Context not injecting: PreToolUse doesn't support context injection

**Cursor**:

- Hook not firing: Check `~/.cursor/hooks.json` exists and has correct structure
- Wrong file: Hooks must be in `hooks.json` NOT `settings.json`
- Policies not loading: Ensure policies are in `policies/cursor/` not `policies/claude/`
- File content missing: Only `beforeReadFile` provides `content` field

---

## Next Steps

- **Claude Code Users**: See [Claude Code Integration Guide](claude-code.md)
- **Cursor Users**: See [Cursor Integration Guide](cursor.md)
- **Factory AI Users**: See [Factory AI Integration Guide](factory.md)
- **OpenCode Users**: See [OpenCode Quick Start](../../agents/opencode/quickstart.md)
- **Architecture Deep Dive**: See [Harness-Specific Architecture](../architecture/harness-model.md)
- **Writing Policies**: See [Policy Authoring Guide](../policies/writing-policies.md)
