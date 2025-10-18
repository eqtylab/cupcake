# Key Differences from Claude Code Evaluation

This Cursor evaluation is adapted from the Claude Code version with these key differences:

## 1. Hook Configuration

| Aspect | Claude Code | Cursor |
|--------|-------------|---------|
| Config Location | `.claude/settings.json` (project) | `~/.cursor/hooks.json` (global) |
| Config Format | Nested hooks with matchers | Version 1 hooks structure |
| Hook Names | `PreToolUse`, `PostToolUse` | `beforeShellExecution`, `afterFileEdit`, etc. |

## 2. Event Structure

### Claude Code Event
```json
{
  "hook_event_name": "PreToolUse",
  "tool_name": "Bash",
  "tool_input": {
    "command": "rm -rf /tmp"
  }
}
```

### Cursor Event
```json
{
  "hook_event_name": "beforeShellExecution",
  "conversation_id": "conv_123",
  "command": "rm -rf /tmp",  // Direct field
  "cwd": "/tmp"
}
```

## 3. Response Format

### Claude Code Response
```json
{
  "continue": false,
  "stopReason": "Dangerous command blocked"
}
```

### Cursor Response
```json
{
  "permission": "deny",
  "userMessage": "Dangerous command blocked",
  "agentMessage": "rm -rf detected. Use 'trash' command..."  // Separate agent guidance
}
```

## 4. Policy Differences

### Routing
- **Claude Code**: Routes on `tool_name` and `hook_event_name`
- **Cursor**: Routes primarily on `hook_event_name`

### Agent Feedback
- **Claude Code**: Single `stopReason` message
- **Cursor**: Separate `userMessage` and `agentMessage` via `agent_context` field

### Example Policy
```rego
# Cursor version with differentiated messages
deny contains decision if {
    input.hook_event_name == "beforeShellExecution"
    contains(input.command, "sudo")
    decision := {
        "reason": "Elevated privileges required",  // User sees this
        "agent_context": "sudo detected. Consider: 1) Use without sudo...",  // Agent guidance
        "severity": "HIGH",
        "rule_id": "CURSOR-SUDO-001"
    }
}
```

## 5. Feature Support

| Feature | Claude Code | Cursor |
|---------|-------------|---------|
| Context Injection | ✅ `additionalContext` | ❌ Not supported |
| Agent Feedback | Single message | Separate user/agent messages |
| File Content | Via tool response | Direct in event |
| Attachments | Not applicable | Supported (files/rules) |
| Stop Event | Multiple variants | Single with status field |

## 6. Testing Approach

- **Claude Code**: Uses `claude` CLI with `--dangerously-skip-permissions`
- **Cursor**: Interactive in editor, no CLI mode

## 7. Global vs Project Config

- **Claude Code**: Project-specific `.claude/settings.json`
- **Cursor**: Global `~/.cursor/hooks.json` affects all projects

This evaluation demonstrates that while the core Cupcake engine is the same, each harness integration is tailored to the specific capabilities and conventions of the target AI coding agent.