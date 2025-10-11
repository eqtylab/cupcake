# Cursor Hooks vs Claude Code Hooks: Technical Comparison

## Executive Summary

Both Cursor and Claude Code implement hooks systems for extending AI agent behavior, but they differ significantly in architecture, event models, and permission handling. This document provides a comprehensive technical comparison.

## 1. Configuration Architecture

### Configuration File Structure

| Aspect                | Cursor Hooks                   | Claude Code Hooks                   |
| --------------------- | ------------------------------ | ----------------------------------- |
| **Format**            | JSON (`hooks.json`)            | JSON (in `settings.json` files)     |
| **Location**          | Standalone file                | Integrated into settings system     |
| **User Config**       | `~/.cursor/hooks.json`         | `~/.claude/settings.json`           |
| **Project Config**    | Relative to `hooks.json`       | `.claude/settings.json`             |
| **Local Overrides**   | Not supported                  | `.claude/settings.local.json`       |
| **Enterprise Config** | Platform-specific global paths | Enterprise managed policy settings  |
| **Priority**          | User > Global                  | Local > Project > User > Enterprise |

### Configuration Schema Comparison

**Cursor:**

```json
{
  "version": 1,
  "hooks": {
    "hookEventName": [{ "command": "path/to/script.sh" }]
  }
}
```

**Claude Code:**

```json
{
  "hooks": {
    "EventName": [
      {
        "matcher": "ToolPattern",
        "hooks": [
          {
            "type": "command",
            "command": "path/to/script.sh",
            "timeout": 60
          }
        ]
      }
    ]
  }
}
```

### Key Architectural Differences

| Feature             | Cursor                 | Claude Code                    |
| ------------------- | ---------------------- | ------------------------------ |
| **Matcher System**  | Not present            | Required for tool-based events |
| **Timeout Config**  | Global (60s implied)   | Per-command configurable       |
| **Hook Type Field** | Implicit               | Explicit (`"type": "command"`) |
| **Versioning**      | Explicit version field | No version field               |
| **Nesting**         | Flat array             | Matcher → Hooks array          |

## 2. Hook Events Matrix

### Event Coverage Comparison

| Hook Event                   | Cursor                    | Claude Code                               | Notes                                        |
| ---------------------------- | ------------------------- | ----------------------------------------- | -------------------------------------------- |
| **Pre-execution validation** | ✅ `beforeShellExecution` | ✅ `PreToolUse`                           | Claude Code covers all tools, not just shell |
| **Pre-execution (MCP)**      | ✅ `beforeMCPExecution`   | ✅ `PreToolUse` (via matcher)             | Claude Code uses unified matcher             |
| **Post-execution**           | ❌                        | ✅ `PostToolUse`                          | Cursor lacks post-execution hooks            |
| **File editing**             | ✅ `afterFileEdit`        | ✅ `PostToolUse` (matcher: `Write\|Edit`) | Different approaches                         |
| **File reading**             | ✅ `beforeReadFile`       | ✅ `PreToolUse` (matcher: `Read`)         | Different approaches                         |
| **Prompt submission**        | ✅ `beforeSubmitPrompt`   | ✅ `UserPromptSubmit`                     | Similar functionality                        |
| **Agent stop**               | ✅ `stop`                 | ✅ `Stop`                                 | Similar functionality                        |
| **Subagent stop**            | ❌                        | ✅ `SubagentStop`                         | Claude Code specific                         |
| **Notifications**            | ❌                        | ✅ `Notification`                         | Claude Code specific                         |
| **Compaction**               | ❌                        | ✅ `PreCompact`                           | Claude Code specific                         |
| **Session lifecycle**        | ❌                        | ✅ `SessionStart`, `SessionEnd`           | Claude Code specific                         |

### Event Granularity

**Cursor**: Task-specific events (shell, MCP, file operations)

- More explicit event names
- Separate events for different operation types
- Limited to predefined operations

**Claude Code**: Tool-based events with matchers

- Unified `PreToolUse`/`PostToolUse` events
- Flexible regex matchers for tool selection
- Covers any tool (Bash, Read, Write, Edit, Task, MCP, etc.)

## 3. Permission & Control Models

### Permission Decision Matrix

| Decision Type            | Cursor                  | Claude Code                         |
| ------------------------ | ----------------------- | ----------------------------------- |
| **Allow**                | `"permission": "allow"` | `"permissionDecision": "allow"`     |
| **Deny**                 | `"permission": "deny"`  | `"permissionDecision": "deny"`      |
| **Ask User**             | `"permission": "ask"`   | `"permissionDecision": "ask"`       |
| **Approve (deprecated)** | N/A                     | `"decision": "approve"` (old)       |
| **Block**                | N/A                     | `"decision": "block"` (PostToolUse) |

### Permission Flow Comparison

**Cursor `beforeShellExecution`:**

```json
{
  "permission": "allow" | "deny" | "ask",
  "userMessage": "Message shown in client",
  "agentMessage": "Message sent to agent"
}
```

**Claude Code `PreToolUse`:**

```json
{
  "hookSpecificOutput": {
    "hookEventName": "PreToolUse",
    "permissionDecision": "allow" | "deny" | "ask",
    "permissionDecisionReason": "Reason here"
  }
}
```

### Control Mechanisms

| Feature                  | Cursor                   | Claude Code                      |
| ------------------------ | ------------------------ | -------------------------------- |
| **Block tool execution** | ✅ `permission: deny`    | ✅ `permissionDecision: deny`    |
| **Prompt user**          | ✅ `permission: ask`     | ✅ `permissionDecision: ask`     |
| **Auto-approve**         | ✅ `permission: allow`   | ✅ `permissionDecision: allow`   |
| **Stop continuation**    | Via `stop` event         | ✅ `continue: false`             |
| **Block prompt**         | Via `beforeSubmitPrompt` | ✅ `UserPromptSubmit` decision   |
| **Prevent stoppage**     | ❌                       | ✅ `Stop` with `decision: block` |
| **Add context**          | ❌                       | ✅ via `additionalContext`       |

## 4. Input Schema Comparison

### Common Fields

| Field                 | Cursor                    | Claude Code       | Notes                                 |
| --------------------- | ------------------------- | ----------------- | ------------------------------------- |
| **Session ID**        | `conversation_id`         | `session_id`      | Different naming                      |
| **Generation ID**     | `generation_id`           | N/A               | Cursor specific                       |
| **Event name**        | `hook_event_name`         | `hook_event_name` | Same                                  |
| **Workspace**         | `workspace_roots` (array) | N/A               | Cursor provides workspace roots       |
| **Working directory** | Via context               | `cwd`             | Claude Code explicit                  |
| **Transcript path**   | ❌                        | `transcript_path` | Claude Code provides conversation log |

### Tool Execution Input

**Cursor `beforeShellExecution`:**

```json
{
  "command": "<full terminal command>",
  "cwd": "<current working directory>",
  "conversation_id": "...",
  "generation_id": "...",
  "hook_event_name": "beforeShellExecution",
  "workspace_roots": ["<path>"]
}
```

**Claude Code `PreToolUse` (Bash):**

```json
{
  "session_id": "abc123",
  "transcript_path": "/Users/.../transcript.jsonl",
  "cwd": "/Users/...",
  "hook_event_name": "PreToolUse",
  "tool_name": "Bash",
  "tool_input": {
    "command": "<command string>"
  }
}
```

### File Operation Input

**Cursor `afterFileEdit`:**

```json
{
  "file_path": "<absolute path>",
  "edits": [{ "old_string": "<search>", "new_string": "<replace>" }]
}
```

**Claude Code `PostToolUse` (Write):**

```json
{
  "tool_name": "Write",
  "tool_input": {
    "file_path": "/path/to/file.txt",
    "content": "file content"
  },
  "tool_response": {
    "filePath": "/path/to/file.txt",
    "success": true
  }
}
```

## 5. Output Schema Comparison

### Exit Code Behavior

| Exit Code | Cursor  | Claude Code                                                             |
| --------- | ------- | ----------------------------------------------------------------------- |
| **0**     | Success | Success (stdout to transcript, except UserPromptSubmit adds to context) |
| **2**     | N/A     | Blocking error (stderr to Claude)                                       |
| **Other** | Error   | Non-blocking error (stderr to user)                                     |

### JSON Output Fields

| Field                      | Cursor | Claude Code     | Purpose                     |
| -------------------------- | ------ | --------------- | --------------------------- |
| `continue`                 | ❌     | ✅              | Stop execution flow         |
| `stopReason`               | ❌     | ✅              | Reason for stopping         |
| `suppressOutput`           | ❌     | ✅              | Hide stdout from transcript |
| `systemMessage`            | ❌     | ✅              | Warning to user             |
| `permission`               | ✅     | ❌ (deprecated) | Permission decision         |
| `permissionDecision`       | ❌     | ✅              | Permission decision (new)   |
| `permissionDecisionReason` | ❌     | ✅              | Reason for decision         |
| `userMessage`              | ✅     | ❌              | Message to user             |
| `agentMessage`             | ✅     | ❌              | Message to agent            |
| `decision`                 | ❌     | ✅              | Block/approve decision      |
| `reason`                   | ❌     | ✅              | Decision reason             |
| `additionalContext`        | ❌     | ✅              | Context injection           |

## 6. Tool Matching & Filtering

### Matcher System

**Cursor:**

- No matcher system
- Separate hook events for different tools
- Fixed event names map to specific operations

**Claude Code:**

- Sophisticated regex matcher system
- Match by tool name patterns
- Examples:
  - `"Write"` - Exact match
  - `"Edit|Write"` - Multiple tools
  - `"Notebook.*"` - Regex pattern
  - `"*"` or `""` - All tools
  - `"mcp__memory__.*"` - MCP tool patterns

### MCP Integration

| Aspect              | Cursor                                     | Claude Code                       |
| ------------------- | ------------------------------------------ | --------------------------------- |
| **MCP Event**       | Separate `beforeMCPExecution`              | Unified `PreToolUse` with matcher |
| **MCP Input**       | `tool_name`, `tool_input`, `url`/`command` | Same via matcher pattern          |
| **MCP Tool Naming** | Standard MCP format                        | `mcp__<server>__<tool>`           |
| **MCP Filtering**   | Via dedicated event                        | Via regex matcher                 |

**Example Claude Code MCP matcher:**

```json
{
  "matcher": "mcp__memory__.*",
  "hooks": [...]
}
```

## 7. Advanced Features

### Context Injection

| Feature                               | Cursor | Claude Code                          |
| ------------------------------------- | ------ | ------------------------------------ |
| **Add context to prompt**             | ❌     | ✅ Via `additionalContext`           |
| **Add context at session start**      | ❌     | ✅ Via `SessionStart` hook           |
| **Add context from UserPromptSubmit** | ❌     | ✅ Via stdout or `additionalContext` |

### Session Lifecycle

| Feature                | Cursor | Claude Code                                         |
| ---------------------- | ------ | --------------------------------------------------- |
| **Session start hook** | ❌     | ✅ `SessionStart` (startup, resume, clear, compact) |
| **Session end hook**   | ❌     | ✅ `SessionEnd` (cleanup, logging)                  |
| **Subagent lifecycle** | ❌     | ✅ `SubagentStop`                                   |
| **Compaction hooks**   | ❌     | ✅ `PreCompact` (manual, auto)                      |

### Notification System

| Feature                  | Cursor | Claude Code                              |
| ------------------------ | ------ | ---------------------------------------- |
| **Notification hooks**   | ❌     | ✅ Permission requests, idle prompts     |
| **Custom notifications** | ❌     | Hook can respond to system notifications |

## 8. Execution Behavior

### Parallel Execution

| Aspect               | Cursor               | Claude Code                                       |
| -------------------- | -------------------- | ------------------------------------------------- |
| **Multiple hooks**   | Sequential (implied) | Parallel execution                                |
| **Deduplication**    | Not mentioned        | ✅ Automatic                                      |
| **Timeout**          | 60s (implied)        | 60s default, per-command configurable             |
| **Timeout behavior** | Unclear              | Individual command timeout, doesn't affect others |

### Environment

| Feature                     | Cursor                   | Claude Code                              |
| --------------------------- | ------------------------ | ---------------------------------------- |
| **Working directory**       | Current directory        | Current directory + `CLAUDE_PROJECT_DIR` |
| **Project path variable**   | Relative to `hooks.json` | `$CLAUDE_PROJECT_DIR` env var            |
| **Plugin support**          | ❌                       | ✅ `$CLAUDE_PLUGIN_ROOT`                 |
| **Environment inheritance** | Yes                      | Yes (Claude Code's environment)          |

### Configuration Updates

| Aspect                    | Cursor           | Claude Code                          |
| ------------------------- | ---------------- | ------------------------------------ |
| **Hot reload**            | Requires restart | Requires review via `/hooks` menu    |
| **External modification** | Requires restart | Warns and requires approval          |
| **Safety mechanism**      | None specified   | Snapshot at startup, review required |

## 9. Plugin System

| Feature                      | Cursor           | Claude Code                                  |
| ---------------------------- | ---------------- | -------------------------------------------- |
| **Plugin hooks**             | ❌ Not supported | ✅ First-class support                       |
| **Plugin integration**       | N/A              | Automatic merge with user/project hooks      |
| **Plugin-specific env vars** | N/A              | `${CLAUDE_PLUGIN_ROOT}`                      |
| **Plugin hook format**       | N/A              | Same as regular hooks + optional description |

**Claude Code Plugin Hook Example:**

```json
{
  "description": "Automatic code formatting",
  "hooks": {
    "PostToolUse": [
      {
        "matcher": "Write|Edit",
        "hooks": [
          {
            "type": "command",
            "command": "${CLAUDE_PLUGIN_ROOT}/scripts/format.sh",
            "timeout": 30
          }
        ]
      }
    ]
  }
}
```

## 10. Security & Safety

### Security Features

| Feature                     | Cursor                          | Claude Code                      |
| --------------------------- | ------------------------------- | -------------------------------- |
| **Path validation**         | Developer responsibility        | Developer responsibility         |
| **Input sanitization**      | Developer responsibility        | Developer responsibility         |
| **Execution limits**        | 60s timeout                     | Configurable timeout per command |
| **Configuration safety**    | Immediate effect                | Snapshot + review mechanism      |
| **SSH support**             | ❌ Not yet supported            | Not mentioned                    |
| **Sensitive file handling** | `beforeReadFile` with redaction | `PreToolUse` with Read matcher   |

### Best Practices Emphasis

Both systems emphasize:

- Quote shell variables
- Validate inputs
- Use absolute paths
- Block path traversal
- Skip sensitive files

**Claude Code additionally emphasizes:**

- Testing in safe environments
- Structured logging
- Resource monitoring

## 11. Debugging & Observability

| Feature               | Cursor                | Claude Code                         |
| --------------------- | --------------------- | ----------------------------------- |
| **Debug UI**          | Hooks tab in settings | `/hooks` command                    |
| **Output channel**    | Hooks output channel  | Debug mode (`--debug`)              |
| **Transcript mode**   | Not mentioned         | Ctrl-R for hook progress            |
| **Error visibility**  | Settings UI           | Debug logs + transcript             |
| **Execution details** | Settings UI           | Detailed debug output with matchers |

## 12. Use Case Alignment

### Common Use Cases

| Use Case                     | Cursor Implementation             | Claude Code Implementation                         |
| ---------------------------- | --------------------------------- | -------------------------------------------------- |
| **Format after edit**        | `afterFileEdit` → formatter       | `PostToolUse` (matcher: `Write\|Edit`)             |
| **Block dangerous commands** | `beforeShellExecution` → deny     | `PreToolUse` (matcher: `Bash`) → deny              |
| **Redact secrets**           | `beforeReadFile` → content filter | `PreToolUse` (matcher: `Read`) → deny              |
| **Audit operations**         | Multiple event handlers           | Single `PreToolUse`/`PostToolUse` with `*` matcher |
| **Analytics**                | Hook for each event type          | Unified hooks with tool filtering                  |

### Unique Use Cases

**Cursor-specific:**

- Workspace-aware operations (via `workspace_roots`)
- Separate MCP and shell handling

**Claude Code-specific:**

- Session initialization context loading (`SessionStart`)
- Preventing agent stoppage (`Stop` with block)
- Auto-compaction customization (`PreCompact`)
- Subagent task monitoring (`SubagentStop`)
- Notification-based workflows (`Notification`)

## 13. Technical Recommendations

### Choose Cursor Hooks when:

- You need explicit workspace context
- You prefer simpler, task-specific event names
- You want separate MCP/shell handling
- Your use cases fit predefined events

### Choose Claude Code Hooks when:

- You need flexible tool matching with regex
- You want session lifecycle hooks
- You require context injection capabilities
- You need plugin system integration
- You want to prevent agent stoppage
- You need post-execution hooks
- You want parallel hook execution

## 14. Migration Considerations

### Cursor → Claude Code

| Cursor Hook            | Claude Code Equivalent                                   |
| ---------------------- | -------------------------------------------------------- |
| `beforeShellExecution` | `PreToolUse` with `"matcher": "Bash"`                    |
| `beforeMCPExecution`   | `PreToolUse` with `"matcher": "mcp__.*"`                 |
| `afterFileEdit`        | `PostToolUse` with `"matcher": "Write\|Edit\|MultiEdit"` |
| `beforeReadFile`       | `PreToolUse` with `"matcher": "Read"`                    |
| `beforeSubmitPrompt`   | `UserPromptSubmit`                                       |
| `stop`                 | `Stop`                                                   |

**Key migration challenges:**

1. Matcher system requires pattern definition
2. JSON output schema differs
3. Configuration nesting changes
4. Permission field names changed

### Example Migration

**Cursor:**

```json
{
  "version": 1,
  "hooks": {
    "beforeShellExecution": [{ "command": "./hooks/audit.sh" }]
  }
}
```

**Claude Code:**

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Bash",
        "hooks": [
          {
            "type": "command",
            "command": "$CLAUDE_PROJECT_DIR/.claude/hooks/audit.sh"
          }
        ]
      }
    ]
  }
}
```

## 15. Summary Matrix

| Dimension             | Cursor         | Claude Code         | Winner      |
| --------------------- | -------------- | ------------------- | ----------- |
| **Event Coverage**    | 6 events       | 11 events           | Claude Code |
| **Flexibility**       | Fixed events   | Regex matchers      | Claude Code |
| **Configuration**     | Standalone     | Integrated settings | Tie         |
| **Permission Model**  | 3-state        | 3-state + block     | Claude Code |
| **Context Injection** | ❌             | ✅                  | Claude Code |
| **Plugin Support**    | ❌             | ✅                  | Claude Code |
| **Post-execution**    | Limited        | Full support        | Claude Code |
| **Simplicity**        | Simpler        | More complex        | Cursor      |
| **Session Lifecycle** | ❌             | ✅                  | Claude Code |
| **Debugging**         | UI-based       | CLI + UI            | Tie         |
| **MCP Integration**   | Explicit event | Unified matcher     | Tie         |
| **Safety Mechanisms** | Basic          | Snapshot + review   | Claude Code |

## Conclusion

**Cursor Hooks** offers a simpler, more straightforward approach with explicit event names that are easy to understand. It's ideal for straightforward use cases with predefined operations.

**Claude Code Hooks** provides a more powerful and flexible system with regex matchers, comprehensive session lifecycle support, context injection, and plugin integration. It's better suited for complex workflows and enterprise deployments requiring fine-grained control.

Both systems share core concepts (JSON I/O, permission control, shell execution) but differ significantly in architecture and advanced capabilities. The choice depends on your specific needs for simplicity versus flexibility.
