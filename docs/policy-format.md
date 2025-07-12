# Policy Format Guide

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
  audit_logging: true     # Log all policy decisions
  debug_mode: false       # Extra debug output

imports:
  - "policies/*.yaml"     # Import all .yaml files from policies/
```

## Policy Structure

Policies use a "grouped map" format organized by hook event and tool:

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
- `Notification` - When Claude requests permission
- `Stop` - When session ends

### Tool Matchers

- Exact match: `"Bash"`
- Multiple tools: `"Edit|Write"`
- All tools: `"*"`

## Conditions

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

```yaml
conditions:
  - type: "check"
    command: "test -f package.json"
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

### Soft Feedback (Non-blocking)

```yaml
action:
  type: "provide_feedback"
  message: "Remember to run tests"
  include_context: false
```

### Hard Block

```yaml
action:
  type: "block_with_feedback"
  feedback_message: "Operation not allowed"
  include_context: true          # Include tool details in message
```

### Auto-approve

```yaml
action:
  type: "approve"
  reason: "Pre-approved safe operation"
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
          command: "git diff --quiet && git diff --cached --quiet"
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

## Best Practices

1. **Use numbered prefixes** for policy files to control load order
2. **One domain per file** - security policies in `10-security.yaml`
3. **Unique policy names** across all files for clear debugging
4. **Test policies** with `cupcake inspect` before deployment
5. **Start with soft feedback** before using hard blocks