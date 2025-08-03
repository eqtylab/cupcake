# Claude Code Hook Events

Cupcake provides comprehensive support for all Claude Code hook events, enabling fine-grained governance over AI agent behavior. This document provides an overview of supported events and their available condition fields.

## Supported Hook Events

### SessionStart

Loads initial context and configuration at session start.

**Available Fields:**

- `session_id`, `transcript_path`, `cwd` - Common event data
- `source` - Session start type: "startup", "resume", or "clear"

### UserPromptSubmit

Validates user prompts and injects dynamic context before agent processing.

**Available Fields:**

- `session_id`, `transcript_path`, `cwd` - Common event data
- `prompt` - The user's submitted prompt text

### PreToolUse

Intercepts tool calls before execution, allowing governance policies to allow, deny, or ask for confirmation.

**Available Fields:**

- `session_id` - Unique session identifier
- `transcript_path` - Path to conversation transcript
- `cwd` - Current working directory
- `tool_name` - Name of the tool being called (e.g., "Bash", "Write", "Read")
- `tool_input.*` - Tool-specific input parameters (e.g., `tool_input.command`, `tool_input.file_path`)

### PostToolUse

Reacts to completed tool executions, enabling validation, logging, and feedback injection.

**Available Fields:**

- `session_id`, `transcript_path`, `cwd` - Common event data
- `tool_name` - Name of the executed tool
- `tool_input.*` - Input parameters that were provided
- `tool_response.*` - Response from tool execution (e.g., `tool_response.success`, `tool_response.output`)

### PreCompact

Influences conversation summarization with custom instructions.

**Available Fields:**

- `session_id`, `transcript_path`, `cwd` - Common event data
- `trigger` - Compaction trigger: "manual" or "auto"
- `custom_instructions` - Optional custom summarization instructions

### Stop & SubagentStop

Controls when the agent concludes its turn, enabling iterative workflows.

**Available Fields:**

- `session_id`, `transcript_path`, `cwd` - Common event data
- `stop_hook_active` - Boolean indicating if stop hook is active (prevents infinite loops)

### Notification

Triggers external notification systems without affecting agent behavior.

**Available Fields:**

- `session_id`, `transcript_path`, `cwd` - Common event data
- `message` - Notification message content

## Example Policy Conditions

```yaml
# Block dangerous bash commands
PreToolUse:
  "Bash":
    - name: block-rm-rf
      conditions:
        - type: pattern
          field: tool_input.command
          regex: "rm\\s+-rf\\s+/"
      action:
        type: block_with_feedback
        feedback_message: "Dangerous rm -rf command blocked"

# Log successful write operations
PostToolUse:
  "Write":
    - name: log-writes
      conditions:
        - type: match
          field: tool_response.success
          value: "true"
      action:
        type: provide_feedback
        message: "File written: {{tool_input.file_path}}"

# Inject context for startup sessions
SessionStart:
  "*":
    - name: startup-context
      conditions:
        - type: match
          field: source
          value: "startup"
      action:
        type: inject_context
        context: "Welcome! This is a new session."
```

## Multi-Agent Architecture

Cupcake's event system is designed for multi-agent support. Claude Code events are handled by the `claude_code` module, with the architecture ready to support additional agents in the future.

For detailed implementation information, see the [source code documentation](../../src/engine/events/claude_code/README.md).
