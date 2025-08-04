# Policy Format Guide

<!-- Last Verified: 2025-08-04 -->

## Overview

Cupcake policies are written in YAML and define rules that govern AI agent behavior. Policies can match specific tools and conditions, then take actions like providing feedback or blocking operations.

## Directory Structure

### Standard Layout (Recommended)

```
your-project/
└── guardrails/
    ├── cupcake.yaml          # Root configuration
    └── policies/             # Policy fragments
        ├── 00-base.yaml
        ├── 10-security.yaml
        └── 20-git.yaml
```

### Alternative: Direct Config File

```bash
cupcake run --config /path/to/my-policy.yaml
```

Use this for testing or when the standard directory structure doesn't fit your needs.

## Root Configuration

The `guardrails/cupcake.yaml` file controls settings and imports:

```yaml
settings:
  debug_mode: false       # Extra debug output
  allow_shell: false      # Enable shell mode execution (security risk)
  default_timeout: 120000 # Default command timeout in milliseconds

imports:
  - "policies/*.yaml"     # Import all .yaml files from policies/
```

## Policy Structure

Policies use a "grouped map" format organized by hook event and tool/matcher:

```yaml
PreToolUse:              # When to evaluate (before tool use)
  "Bash":                # Which tool to match
    - name: "Block rm -rf"
      conditions:        # All conditions must match
        - type: "pattern"
          field: "tool_input.command"
          regex: "^rm\\s+.*-rf"
      action:            # What to do when conditions match
        type: "block_with_feedback"
        feedback_message: "Dangerous command blocked"
```

### Hook Events

- `PreToolUse` - Evaluate before tool execution
- `PostToolUse` - Evaluate after tool execution
- `UserPromptSubmit` - Evaluate user prompts before processing
- `Notification` - When Claude requests permission
- `Stop` - When session ends
- `SubagentStop` - When a subagent session ends
- `PreCompact` - Before context compaction
- `SessionStart` - When a session starts (new in July 2024)

### Tool Matchers

For tool events (PreToolUse/PostToolUse):
- Exact match: `"Bash"`
- Multiple tools: `"Edit|Write"`
- All tools: `"*"`
- Regex pattern: `".*Test$"` (matches any tool ending with "Test")
- MCP tools: `"mcp__.*"` (matches all MCP tools)

For non-tool events:
- **UserPromptSubmit/SessionStart**: Matcher filters against the event itself
  - `""` or `"*"` - Match all events
- **PreCompact**: Matcher compares against trigger field ("manual"/"auto")
  - `"manual"` - Only manual compactions
  - `"auto"` - Only automatic compactions
  - `""` or `"*"` - All compactions
- **SessionStart**: Matcher compares against source field ("startup"/"resume"/"clear")
  - `"startup"` - New sessions
  - `"resume"` - Resumed sessions
  - `"clear"` - Cleared sessions
  - `""` or `"*"` - All session types
- **Other events**: Use `""` or `"*"` to match

**Important Note on Empty Matchers:**
- Both `""` (empty string) and `"*"` (wildcard) match ALL events, including tool events
- This aligns with Claude Code's behavior where empty matcher means "match all tools"
- For tool events like PreToolUse/PostToolUse, empty matcher will match any tool

## Conditions

### Available Fields

Common fields (all events):
- `event_type` - Hook event name (e.g., "PreToolUse", "UserPromptSubmit")
- `session_id` - Unique session identifier
- `transcript_path` - Path to conversation transcript
- `cwd` - Current working directory
- `env.*` - Environment variables (e.g., `env.USER`, `env.HOME`)

Tool event fields (PreToolUse/PostToolUse):
- `tool_name` - Tool being invoked (e.g., "Bash", "Write", "Edit")
- `tool_input.*` - Tool-specific parameters:
  - `tool_input.command` - For Bash tool
  - `tool_input.file_path` - For Write/Edit tools
  - `tool_input.content` - For Write tool
  - And other tool-specific fields
- `tool_response.*` - (PostToolUse only) Tool execution results:
  - `tool_response.success` - Boolean indicating success/failure
  - `tool_response.output` - Tool output text
  - `tool_response.error` - Error message if failed

UserPromptSubmit fields:
- `prompt` - The user's input text

SessionStart fields:
- `source` - Session start type: "startup", "resume", or "clear"

PreCompact fields:
- `trigger` - Compaction trigger: "manual" or "auto"
- `custom_instructions` - Optional custom summarization instructions

Stop/SubagentStop fields:
- `stop_hook_active` - Boolean indicating if stop hook is active

Notification fields:
- `message` - Notification message content

PreCompact fields:
- `trigger` - Compaction trigger: "manual" or "auto"
- `custom_instructions` - Optional custom summarization instructions

Stop/SubagentStop fields:
- `stop_hook_active` - Boolean indicating if stop hook is active (prevents infinite loops)

Note: The current working directory from hook data is automatically used for all command executions.

### Pattern Matching

```yaml
conditions:
  - type: "pattern"
    field: "tool_input.command"    # Field to check
    regex: "git\\s+commit"         # Regex pattern
```

### Exact Matching

```yaml
conditions:
  - type: "match"
    field: "tool_name"
    value: "Write"
```

### Command Execution

Check conditions support two command modes:
- `array`: Secure array-based execution (recommended)
- `shell`: Direct shell execution (requires `allow_shell: true`)

```yaml
conditions:
  - type: "check"
    spec:
      mode: array
      command: ["test"]
      args: ["-f", "package.json"]
    expect_success: true          # true = exit 0 means match
```

### Logical Operators

```yaml
conditions:
  - type: "and"
    conditions:
      - type: "pattern"
        field: "tool_input.file_path"
        regex: "\\.py$"
      - type: "not"
        condition:
          type: "pattern"
          field: "tool_input.file_path"
          regex: "test_"
```

## Actions

Actions define what happens when policy conditions match. There are two categories:

### Soft Actions (Continue Policy Evaluation)
These provide feedback but don't stop policy evaluation:

#### Soft Feedback (Non-blocking)

```yaml
action:
  type: "provide_feedback"
  message: "Remember to run tests"
  include_context: false
```

### Hard Actions (Stop Policy Evaluation)
These make final decisions and stop further policy evaluation:

#### Hard Block

```yaml
action:
  type: "block_with_feedback"
  feedback_message: "Operation not allowed"
  include_context: true          # Include tool details in message
```

#### Auto-allow

```yaml
action:
  type: "allow"
  reason: "Pre-approved safe operation"
```

#### Request User Confirmation

```yaml
action:
  type: "ask"
  reason: "Please confirm this {{tool_name}} operation"
```

### Soft Actions (Continued)

#### Context Injection

```yaml
action:
  type: "inject_context"
  context: "Remember to follow coding standards when editing {{tool_input.file_path}}"
  use_stdout: true  # true = stdout method, false = JSON method
```

#### Run Command

```yaml
action:
  type: "run_command"
  spec:
    mode: array
    command: ["cargo"]
    args: ["fmt", "--all"]
  on_failure: "continue"  # or "block" - determines if action is soft or hard
  timeout_seconds: 30
```

## Common Fields

### Available Context Fields

- `tool_name` - Name of the tool being used
- `tool_input.command` - Command for Bash tool
- `tool_input.file_path` - File path for Read/Write/Edit tools
- `tool_input.*` - Any field from the tool's input
- `session_id` - Current session identifier
- `event_type` - Type of hook event

## Validation & Inspection

### Validate Policies

```bash
# Validate directory structure
cupcake validate

# Validate specific file
cupcake validate --policy-file /path/to/config.yaml
```

### Inspect Loaded Policies

```bash
# View all active policies
cupcake inspect

# Inspect specific config
cupcake inspect --config my-policy.yaml
```

Output:
```
NAME                    EVENT       TOOL    ACTION              CONDITIONS
Git Commit Reminder     PreToolUse  Bash    provide_feedback    tool_input.command ~ "git commit"
Block Dangerous Cmds    PreToolUse  Bash    block_with_feedback tool_input.command ~ "^rm.*-rf"
```

## Examples

### Security Policy

```yaml
PreToolUse:
  "Bash":
    - name: "Block AWS Credential Display"
      conditions:
        - type: "pattern"
          field: "tool_input.command"
          regex: "(cat|echo|print).*(AWS_SECRET|aws_secret)"
      action:
        type: "block_with_feedback"
        feedback_message: "Cannot display AWS credentials"
```

### Universal Tool Monitoring

```yaml
PreToolUse:
  "":  # Empty matcher - applies to ALL tools
    - name: "Log sensitive file access"
      conditions:
        - type: "pattern"
          field: "tool_input.file_path"
          regex: "(secrets|passwords|keys)\\.txt"
      action:
        type: "provide_feedback"
        message: "Accessing potentially sensitive file"
```

### Development Workflow

```yaml
PreToolUse:
  "Bash":
    - name: "Require Clean Git Status"
      conditions:
        - type: "pattern"
          field: "tool_input.command"
          regex: "^git\\s+push"
        - type: "check"
          spec:
            mode: array
            command: ["git"]
            args: ["diff", "--quiet"]
          expect_success: false
        - type: "check"
          spec:
            mode: array
            command: ["git"]
            args: ["diff", "--cached", "--quiet"]
          expect_success: false
      action:
        type: "block_with_feedback"
        feedback_message: "Commit your changes before pushing"
```

### File Protection

```yaml
PreToolUse:
  "Write|Edit":
    - name: "Protect Production Config"
      conditions:
        - type: "pattern"
          field: "tool_input.file_path"
          regex: "production\\.env$"
      action:
        type: "block_with_feedback"
        feedback_message: "Cannot modify production environment file"
```

### Prompt Security

```yaml
UserPromptSubmit:
  "":  # Empty string matcher required in YAML (map key)
    - name: "Block Secrets in Prompts"
      conditions:
        - type: "pattern"
          field: "prompt"
          regex: "(api[_-]?key|token|password)\\s*[:=]\\s*[\"']?[a-zA-Z0-9]{16,}"
      action:
        type: "block_with_feedback"
        feedback_message: "Detected potential secret in your prompt. Please remove sensitive information."
```

## Best Practices

1. **Use numbered prefixes** for policy files to control load order
2. **One domain per file** - security policies in `10-security.yaml`
3. **Unique policy names** across all files for clear debugging
4. **Test policies** with `cupcake inspect` before deployment
5. **Start with soft feedback** before using hard blocks