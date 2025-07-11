# Cupcake Hook Events Handling

## Overview

This document details how Cupcake handles all Claude Code hook events, ensuring comprehensive policy enforcement across the entire Claude Code lifecycle.

## Supported Hook Events

### 1. PreToolUse

**Purpose**: Enforce policies before tool execution
**Common Use Cases**:
- Block dangerous commands
- Enforce coding standards before file edits
- Require prerequisites (e.g., must read docs before editing)

**Example Policies**:
```toml
[[policy]]
name = "Block force push to main"
hook_event = "PreToolUse"
matcher = "Bash"
conditions = [
  { type = "command_regex", value = "git push.*--force.*main" }
]
action = {
  type = "block_with_feedback",
  feedback_message = "Force pushing to main branch is prohibited"
}
```

### 2. PostToolUse

**Purpose**: React to completed tool operations
**Common Use Cases**:
- Run formatters after file edits
- Update state tracking
- Verify operation results

**Example Policies**:
```toml
[[policy]]
name = "Format Python files after edit"
hook_event = "PostToolUse"
matcher = "Write|Edit"
conditions = [
  { type = "filepath_regex", value = "\\.py$" }
]
action = {
  type = "run_command",
  command = "black {{tool_input.file_path}}",
  on_failure = "continue"
}
```

### 3. Notification

**Purpose**: Handle Claude Code notifications
**When Triggered**:
- Claude needs permission to use a tool
- Prompt input idle for 60+ seconds

**Example Policies**:
```toml
[[policy]]
name = "Custom notification handler"
hook_event = "Notification"
conditions = []  # Always match
action = {
  type = "run_command",
  command = "notify-send 'Claude Code' '{{message}}'",
  on_failure = "continue"
}
```

### 4. Stop

**Purpose**: Execute policies when main agent stops
**Common Use Cases**:
- Session cleanup
- Final validation
- Ensure work is saved

**Example Policies**:
```toml
[[policy]]
name = "Ensure all changes are committed"
hook_event = "Stop"
conditions = [
  { type = "state_exists", tool = "Write", since_minutes = 30 }
]
action = {
  type = "block_with_feedback",
  feedback_message = "You have uncommitted changes. Please commit or stash them."
}
```

### 5. SubagentStop

**Purpose**: Execute policies when Task subagents complete
**Common Use Cases**:
- Validate subagent work
- Aggregate results
- Chain workflows

**Example Policies**:
```toml
[[policy]]
name = "Validate subagent test results"
hook_event = "SubagentStop"
conditions = [
  { type = "state_query", query = {
    tool = "Task",
    description_contains = "run tests",
    result = "success"
  }}
]
action = {
  type = "update_state",
  event = "SubagentTestsComplete",
  data = { timestamp = "{{now}}" }
}
```

### 6. PreCompact

**Purpose**: Control memory compaction
**Matchers**:
- `manual` - User-initiated via `/compact`
- `auto` - Automatic due to full context

**Example Policies**:
```toml
[[policy]]
name = "Save work before manual compact"
hook_event = "PreCompact"
matcher = "manual"
conditions = []
action = {
  type = "run_command",
  command = "git stash push -m 'Pre-compact autosave'",
  on_failure = "continue"
}

[[policy]]
name = "Log auto-compact events"
hook_event = "PreCompact"
matcher = "auto"
conditions = []
action = {
  type = "run_command",
  command = "echo '{{now}}: Auto-compact triggered' >> ~/.cupcake/compact.log",
  on_failure = "continue"
}
```

## Hook Event Input Schemas

### Common Fields (All Events)
```json
{
  "session_id": "string",
  "transcript_path": "string",
  "hook_event_name": "string"
}
```

### Event-Specific Fields

**PreToolUse/PostToolUse**:
```json
{
  "tool_name": "string",
  "tool_input": { /* tool-specific */ },
  "tool_response": { /* PostToolUse only */ }
}
```

**Notification**:
```json
{
  "message": "string"
}
```

**Stop/SubagentStop**:
```json
{
  "stop_hook_active": "boolean"
}
```

**PreCompact**:
```json
{
  "trigger": "manual|auto",
  "custom_instructions": "string"  // manual only
}
```

## Decision Logic by Event

### PreToolUse
- Can block operations before they happen
- Provides feedback to Claude for correction
- Most common hook for enforcement

### PostToolUse
- Cannot prevent operations (already completed)
- Can provide feedback for future operations
- Useful for reactive policies (formatting, logging)

### Notification
- Cannot block Claude's workflow
- Primarily for user notifications
- Exit code 2 only shows to user, not Claude

### Stop/SubagentStop
- Can prevent Claude from stopping
- Forces continuation with specific instructions
- Useful for ensuring work completion

### PreCompact
- Cannot block compaction
- Used for pre-compaction tasks
- Different behavior for manual vs auto triggers

## Best Practices

1. **Use Appropriate Events**:
   - Prevention → PreToolUse
   - Reaction → PostToolUse
   - User alerts → Notification
   - Session management → Stop/SubagentStop

2. **Consider Event Timing**:
   - PreToolUse: Before operation
   - PostToolUse: After success only
   - Stop: End of agent response
   - Notification: Async from workflow

3. **Handle Edge Cases**:
   - Check `stop_hook_active` to prevent loops
   - Use `on_failure = "continue"` for non-critical commands
   - Consider timeout implications (60s default)

## Integration Examples

### Multi-Event Workflow
```toml
# Track when files are edited
[[policy]]
name = "Track file edits"
hook_event = "PostToolUse"
matcher = "Write|Edit"
action = {
  type = "update_state",
  event = "FileModified",
  data = { path = "{{tool_input.file_path}}" }
}

# Ensure changes are saved before stopping
[[policy]]
name = "Save before stop"
hook_event = "Stop"
conditions = [
  { type = "state_exists", event = "FileModified" }
]
action = {
  type = "block_with_feedback",
  feedback_message = "Please save or commit your changes before stopping"
}
```

### Notification Enhancement
```toml
[[policy]]
name = "Enhanced notifications"
hook_event = "Notification"
conditions = [
  { type = "message_contains", value = "permission" }
]
action = {
  type = "run_command",
  command = "osascript -e 'display notification \"{{message}}\" with title \"Claude Code\"'",
  on_failure = "continue"
}
```