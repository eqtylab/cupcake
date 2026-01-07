# Claude Code Hooks: Security Policy Reference

## Overview

Hooks are user-configurable automation points that execute shell commands or LLM prompts at specific lifecycle events. They can intercept, modify, allow, or deny tool operations.

**Key Security Concern**: Hooks execute arbitrary shell commands with the user's permissions. Enterprise administrators can enforce `allowManagedHooksOnly` to restrict hook sources.

---

## Configuration Sources (Priority Order)

1. Enterprise managed policy settings
2. `~/.claude/settings.json` (User)
3. `.claude/settings.json` (Project)
4. `.claude/settings.local.json` (Local, not committed)
5. Plugin hooks (`hooks/hooks.json` within plugin directory)

---

## Hook Events

| Event               | Trigger                      | Matcher Support                            | Can Block?           |
| ------------------- | ---------------------------- | ------------------------------------------ | -------------------- |
| `PreToolUse`        | Before tool execution        | Yes (tool name)                            | Yes                  |
| `PermissionRequest` | User shown permission dialog | Yes (tool name)                            | Yes                  |
| `PostToolUse`       | After tool completes         | Yes (tool name)                            | Feedback only        |
| `UserPromptSubmit`  | User submits prompt          | No                                         | Yes                  |
| `Stop`              | Main agent finishes          | No                                         | Yes (force continue) |
| `SubagentStop`      | Subagent (Task) finishes     | No                                         | Yes (force continue) |
| `Notification`      | System notification          | Yes (notification type)                    | No                   |
| `PreCompact`        | Before context compaction    | Yes (`manual`/`auto`)                      | No                   |
| `SessionStart`      | Session begins/resumes       | Yes (`startup`/`resume`/`clear`/`compact`) | No                   |
| `SessionEnd`        | Session terminates           | No                                         | No                   |

### Tool Matchers (for PreToolUse/PermissionRequest/PostToolUse)

- Exact match: `"Write"`, `"Bash"`, `"Read"`
- Regex: `"Edit|Write"`, `"Notebook.*"`, `"mcp__.*"`
- Wildcard: `"*"` or `""` (matches all)
- MCP tools: `mcp__<server>__<tool>` (e.g., `mcp__github__search_repositories`)

### Notification Types

`permission_prompt`, `idle_prompt`, `auth_success`, `elicitation_dialog`

---

## Hook Input Schema (stdin JSON)

### Common Fields (All Events)

```typescript
{
  session_id: string;
  transcript_path: string; // Path to conversation JSONL
  cwd: string; // Current working directory
  permission_mode: "default" | "plan" | "acceptEdits" | "bypassPermissions";
  hook_event_name: string;
}
```

### PreToolUse

```typescript
{
  ...common,
  hook_event_name: "PreToolUse",
  tool_name: string,
  tool_input: Record<string, any>,  // Tool-specific schema
  tool_use_id: string
}
```

### PostToolUse

```typescript
{
  ...common,
  hook_event_name: "PostToolUse",
  tool_name: string,
  tool_input: Record<string, any>,
  tool_response: Record<string, any>,  // Tool-specific
  tool_use_id: string
}
```

### UserPromptSubmit

```typescript
{
  ...common,
  hook_event_name: "UserPromptSubmit",
  prompt: string
}
```

### Stop / SubagentStop

```typescript
{
  ...common,
  hook_event_name: "Stop" | "SubagentStop",
  stop_hook_active: boolean  // True if already continuing from a stop hook
}
```

### Notification

```typescript
{
  ...common,
  hook_event_name: "Notification",
  message: string,
  notification_type: string
}
```

### SessionStart

```typescript
{
  ...common,
  hook_event_name: "SessionStart",
  source: "startup" | "resume" | "clear" | "compact"
}
```

### SessionEnd

```typescript
{
  ...common,
  hook_event_name: "SessionEnd",
  reason: "clear" | "logout" | "prompt_input_exit" | "other"
}
```

---

## Hook Output Schema

### Exit Code Semantics

| Exit Code | Behavior                                                   |
| --------- | ---------------------------------------------------------- |
| `0`       | Success. Parse stdout for JSON control.                    |
| `2`       | Blocking error. stderr shown to Claude/user. JSON ignored. |
| Other     | Non-blocking error. stderr logged. Execution continues.    |

### JSON Output Structure (exit code 0 only)

```typescript
{
  // Common optional fields
  continue?: boolean;        // false = stop Claude entirely
  stopReason?: string;       // Message when continue=false
  suppressOutput?: boolean;  // Hide from transcript
  systemMessage?: string;    // Warning shown to user

  // Event-specific control
  decision?: "block";        // For PostToolUse, Stop, SubagentStop
  reason?: string;           // Explanation for decision

  hookSpecificOutput?: {
    hookEventName: string;
    // Event-specific fields below
  }
}
```

---

## Decision Control by Event

### PreToolUse

```typescript
hookSpecificOutput: {
  hookEventName: "PreToolUse",
  permissionDecision: "allow" | "deny" | "ask",
  permissionDecisionReason?: string,
  updatedInput?: Record<string, any>  // Modify tool input
}
```

| Decision | Effect                                 |
| -------- | -------------------------------------- |
| `allow`  | Bypass permission system, execute tool |
| `deny`   | Block tool, reason shown to Claude     |
| `ask`    | Prompt user for confirmation           |

### PermissionRequest

```typescript
hookSpecificOutput: {
  hookEventName: "PermissionRequest",
  decision: {
    behavior: "allow" | "deny",
    updatedInput?: Record<string, any>,  // For "allow"
    message?: string,                     // For "deny"
    interrupt?: boolean                   // For "deny", stops Claude
  }
}
```

### PostToolUse

```typescript
{
  decision?: "block",      // Prompts Claude with reason
  reason?: string,
  hookSpecificOutput: {
    hookEventName: "PostToolUse",
    additionalContext?: string
  }
}
```

### UserPromptSubmit

```typescript
{
  decision?: "block",      // Prevents prompt processing
  reason?: string,         // Shown to user only
  hookSpecificOutput: {
    hookEventName: "UserPromptSubmit",
    additionalContext?: string  // Injected into context
  }
}
```

### Stop / SubagentStop

```typescript
{
  decision?: "block",      // Forces Claude to continue
  reason: string           // Required when blocking
}
```

### SessionStart

```typescript
hookSpecificOutput: {
  hookEventName: "SessionStart",
  additionalContext?: string  // Added to session context
}
```

---

## Environment Variables

| Variable             | Availability      | Description                                |
| -------------------- | ----------------- | ------------------------------------------ |
| `CLAUDE_PROJECT_DIR` | All hooks         | Absolute path to project root              |
| `CLAUDE_ENV_FILE`    | SessionStart only | File path for persisting env vars          |
| `CLAUDE_CODE_REMOTE` | All hooks         | `"true"` if remote/web, unset if local CLI |
| `CLAUDE_PLUGIN_ROOT` | Plugin hooks      | Absolute path to plugin directory          |

---

## Execution Constraints

- **Timeout**: 60 seconds default, configurable per hook
- **Parallelization**: All matching hooks run concurrently
- **Deduplication**: Identical commands deduplicated automatically
- **Snapshot**: Hook config captured at startup; runtime changes require `/hooks` review

---

## Security-Relevant Configuration Schema

```json
{
  "hooks": {
    "<EventName>": [
      {
        "matcher": "<pattern>",  // Optional for some events
        "hooks": [
          {
            "type": "command" | "prompt",
            "command": "<shell command>",  // For type: command
            "prompt": "<LLM prompt>",      // For type: prompt
            "timeout": 60                  // Optional, seconds
          }
        ]
      }
    ]
  }
}
```

### Prompt-Based Hooks (type: "prompt")

LLM evaluates and returns:

```typescript
{
  decision: "approve" | "block",
  reason: string,
  continue?: boolean,
  stopReason?: string,
  systemMessage?: string
}
```

---

## Policy Considerations

1. **Input Validation**: All tool inputs flow through `tool_input` — validate file paths, commands, patterns
2. **Permission Escalation**: `permissionDecision: "allow"` bypasses user consent
3. **Input Modification**: `updatedInput` can alter tool parameters before execution
4. **MCP Surface**: Third-party MCP tools expose additional attack surface via `mcp__*` pattern
5. **Environment Persistence**: `CLAUDE_ENV_FILE` allows hooks to inject env vars into all subsequent Bash executions
6. **Transcript Access**: All hooks receive `transcript_path` — full conversation history

---

Hooks can return structured JSON in stdout for more sophisticated control.
JSON output is only processed when the hook exits with code 0. If your hook exits with code 2 (blocking error), stderr text is used directly—any JSON in stdout is ignored. For other non-zero exit codes, only stderr is shown to the user in verbose mode (ctrl+o).
​
Common JSON Fields
All hook types can include these optional fields:
{
"continue": true, // Whether Claude should continue after hook execution (default: true)
"stopReason": "string", // Message shown when continue is false

"suppressOutput": true, // Hide stdout from transcript mode (default: false)
"systemMessage": "string" // Optional warning message shown to the user
}
If continue is false, Claude stops processing after the hooks run.
For PreToolUse, this is different from "permissionDecision": "deny", which only blocks a specific tool call and provides automatic feedback to Claude.
For PostToolUse, this is different from "decision": "block", which provides automated feedback to Claude.
For UserPromptSubmit, this prevents the prompt from being processed.
For Stop and SubagentStop, this takes precedence over any "decision": "block" output.
In all cases, "continue" = false takes precedence over any "decision": "block" output.
stopReason accompanies continue with a reason shown to the user, not shown to Claude.
​
PreToolUse Decision Control
PreToolUse hooks can control whether a tool call proceeds.
"allow" bypasses the permission system. permissionDecisionReason is shown to the user but not to Claude.
"deny" prevents the tool call from executing. permissionDecisionReason is shown to Claude.
"ask" asks the user to confirm the tool call in the UI. permissionDecisionReason is shown to the user but not to Claude.
Additionally, hooks can modify tool inputs before execution using updatedInput:
updatedInput allows you to modify the tool’s input parameters before the tool executes.
This is most useful with "permissionDecision": "allow" to modify and approve tool calls.
{
"hookSpecificOutput": {
"hookEventName": "PreToolUse",
"permissionDecision": "allow"
"permissionDecisionReason": "My reason here",
"updatedInput": {
"field_to_modify": "new value"
}
}
}
The decision and reason fields are deprecated for PreToolUse hooks. Use hookSpecificOutput.permissionDecision and hookSpecificOutput.permissionDecisionReason instead. The deprecated fields "approve" and "block" map to "allow" and "deny" respectively.
​
PermissionRequest Decision Control
PermissionRequest hooks can allow or deny permission requests shown to the user.
For "behavior": "allow" you can also optionally pass in an "updatedInput" that modifies the tool’s input parameters before the tool executes.
For "behavior": "deny" you can also optionally pass in a "message" string that tells the model why the permission was denied, and a boolean "interrupt" which will stop Claude.
{
"hookSpecificOutput": {
"hookEventName": "PermissionRequest",
"decision": {
"behavior": "allow",
"updatedInput": {
"command": "npm run lint"
}
}
}
}
​
PostToolUse Decision Control
PostToolUse hooks can provide feedback to Claude after tool execution.
"block" automatically prompts Claude with reason.
undefined does nothing. reason is ignored.
"hookSpecificOutput.additionalContext" adds context for Claude to consider.
{
"decision": "block" | undefined,
"reason": "Explanation for decision",
"hookSpecificOutput": {
"hookEventName": "PostToolUse",
"additionalContext": "Additional information for Claude"
}
}
​
UserPromptSubmit Decision Control
UserPromptSubmit hooks can control whether a user prompt is processed and add context.
Adding context (exit code 0): There are two ways to add context to the conversation:
Plain text stdout (simpler): Any non-JSON text written to stdout is added as context. This is the easiest way to inject information.
JSON with additionalContext (structured): Use the JSON format below for more control. The additionalContext field is added as context.
Both methods work with exit code 0. Plain stdout is shown as hook output in the transcript; additionalContext is added more discretely.
Blocking prompts:
"decision": "block" prevents the prompt from being processed. The submitted prompt is erased from context. "reason" is shown to the user but not added to context.
"decision": undefined (or omitted) allows the prompt to proceed normally.
{
"decision": "block" | undefined,
"reason": "Explanation for decision",
"hookSpecificOutput": {
"hookEventName": "UserPromptSubmit",
"additionalContext": "My additional context here"
}
}
The JSON format isn’t required for simple use cases. To add context, you can print plain text to stdout with exit code 0. Use JSON when you need to block prompts or want more structured control.
​
Stop/SubagentStop Decision Control
Stop and SubagentStop hooks can control whether Claude must continue.
"block" prevents Claude from stopping. You must populate reason for Claude to know how to proceed.
undefined allows Claude to stop. reason is ignored.
{
"decision": "block" | undefined,
"reason": "Must be provided when Claude is blocked from stopping"
}
​
SessionStart Decision Control
SessionStart hooks allow you to load in context at the start of a session.
"hookSpecificOutput.additionalContext" adds the string to the context.
Multiple hooks’ additionalContext values are concatenated.
{
"hookSpecificOutput": {
"hookEventName": "SessionStart",
"additionalContext": "My additional context here"
}
}
​
SessionEnd Decision Control
SessionEnd hooks run when a session ends. They cannot block session termination but can perform cleanup tasks.
