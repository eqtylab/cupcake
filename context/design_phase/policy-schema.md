# Cupcake Policy Schema Design

> **DEPRECATED:** This document describes the legacy TOML format. Plan 005 migrated Cupcake to YAML format. See current implementation in `src/config/` for the active YAML-based system using `guardrails/cupcake.yaml` and `guardrails/policies/*.yaml`.

## Overview

This document defines the schema for `cupcake.toml` - the policy configuration file that drives Cupcake's enforcement engine.

## File Structure

```toml
# Policy schema version for forward compatibility
policy_schema_version = "1.0"

# Global settings
[settings]
audit_logging = false  # Enable audit trail
debug_mode = false     # Verbose logging

# Policy definitions
[[policy]]
name = "Human-readable policy name"
description = "Optional longer description"
hook_event = "PreToolUse"  # When to evaluate (PreToolUse, PostToolUse, Notification, Stop, SubagentStop, PreCompact)
matcher = "Bash|Edit"      # Tool name pattern (regex) - only for PreToolUse/PostToolUse
                           # For PreCompact use: "manual" or "auto"
                           # For other events: omit or use empty string

# Conditions array - ALL must be true for policy to trigger
conditions = [
  { type = "condition_type", value = "pattern", options = {} }
]

# Action to take when all conditions match
action = { type = "action_type", parameters... }
```

## Condition Types

### 1. Basic Conditions

```toml
# Match against tool input command (Bash tool)
{ type = "command_regex", value = "git\\s+commit" }

# Match against file paths
{ type = "filepath_regex", value = "\\.rs$" }
{ type = "filepath_glob", value = "src/**/*.ts" }

# Match file content (for Edit/Write tools)
{ type = "file_content_regex", value = "<button", flags = ["multiline"] }

# Logical operators
{ type = "not", condition = { type = "filepath_regex", value = "test" } }
{ type = "and", conditions = [...] }
{ type = "or", conditions = [...] }
```

### 2. State-Aware Conditions

Cupcake automatically tracks all tool usage (Read, Write, Edit, Bash, etc.) internally. You can query this history without needing to explicitly record events.

```toml
# Check if a file has been read
{ type = "state_exists", tool = "Read", path = "README.md" }

# Check if file has NOT been read
{ type = "state_missing", tool = "Read", path = "doc.xyz" }

# Check if tests have passed recently
{ type = "state_query", query = {
  tool = "Bash",
  command_contains = "npm test",
  result = "success",
  within_minutes = 30
}}

# Check for custom events (these need explicit update_state)
{ type = "state_exists", event = "FeatureComplete" }
```

### 3. Advanced Conditions

```toml
# File system checks
{ type = "file_exists", path = "./package-lock.json" }
{ type = "file_modified_within", path = "src/", minutes = 5 }

# Environment checks
{ type = "env_var_equals", name = "NODE_ENV", value = "production" }
{ type = "working_dir_contains", value = "/production/" }

# Time-based
{ type = "time_window", start = "09:00", end = "17:00", timezone = "America/New_York" }
{ type = "day_of_week", days = ["Mon", "Tue", "Wed", "Thu", "Fri"] }
```

## Action Types

Actions are categorized as either "soft" (feedback only) or "hard" (decision-making):

### 1. Soft Actions (Pass 1)

```toml
# Provide feedback without blocking
action = {
  type = "provide_feedback",
  message = "• Use <Button> instead of <button>"
}
```

### 2. Hard Actions (Pass 2)

```toml
# Block with feedback to Claude
action = {
  type = "block_with_feedback",
  feedback_message = "Policy violation: Use 'rg' instead of 'grep' for better performance."
}

# Auto-approve (bypass permission prompt)
action = {
  type = "approve",
  reason = "Auto-approved by security policy"
}
```

### 3. Command Execution Actions

```toml
# Run command - becomes "hard" action if it fails with on_failure = "block"
action = {
  type = "run_command",
  command = "cargo fmt --check",
  on_failure = "block",  # Makes this a hard action
  on_failure_feedback = "Please run 'cargo fmt' before editing Rust files"
}

# Run command without blocking - remains "soft" action
action = {
  type = "run_command",
  command = "notify-send 'File modified: {{tool_input.file_path}}'",
  on_failure = "continue",  # Just for notification
  background = true
}
```

### 4. State Modification Actions

```toml
# Record custom events (tool usage is tracked automatically)
action = {
  type = "update_state",
  event = "FeatureComplete",  # Custom business logic event
  data = { feature = "user-auth", version = "2.0" }
}

# Note: You do NOT need update_state for tracking tool usage like:
# - File reads (Read tool)
# - File writes (Write/Edit tools)  
# - Commands run (Bash tool)
# These are automatically tracked by Cupcake

# Conditional action based on state
action = {
  type = "conditional",
  if = { type = "state_exists", event = "DebugMode" },
  then = { type = "block_with_feedback", feedback_message = "Debug mode: operation blocked" },
  else = { type = "approve" }
}
```

## Complete Policy Examples

### Example 1: Enforce Code Review

```toml
[[policy]]
name = "Require PR review before merging"
hook_event = "PreToolUse"
matcher = "Bash"
conditions = [
  { type = "command_regex", value = "git\\s+merge|git\\s+push.*main" },
  { type = "state_missing", event = "PRApproved", since = "last_commit" }
]
action = {
  type = "block_with_feedback",
  feedback_message = "Cannot merge without PR approval. Please create a PR and get it reviewed first."
}
```

### Example 2: Component Directory Enforcement

```toml
[[policy]]
name = "Enforce component directory structure"
hook_event = "PreToolUse"
matcher = "Write"
conditions = [
  { type = "file_content_regex", value = "export\\s+(default\\s+)?function\\s+\\w+Component" },
  { type = "not", condition = { type = "filepath_glob", value = "src/components/**/*" } }
]
action = {
  type = "block_with_feedback",
  feedback_message = "React components must be placed in src/components/ directory"
}
```

### Example 3: Test Before Commit

```toml
[[policy]]
name = "Run tests before git commit"
hook_event = "PreToolUse"
matcher = "Bash"
conditions = [
  { type = "command_regex", value = "git\\s+commit" }
]
action = {
  type = "run_command",
  command = "npm test",
  on_failure = "block",
  on_failure_feedback = "Tests must pass before committing. Fix the failing tests:\n{{stderr}}"
}
```

### Example 4: Multiple Policies Aggregation

This example shows how multiple policies work together when editing a TSX file:

```toml
[[policy]]
name = "No console.log in production"
hook_event = "PreToolUse"
matcher = "Write|Edit"
conditions = [
  { type = "filepath_glob", value = "src/**/*.{ts,tsx}" },
  { type = "file_content_regex", value = "console\\.(log|debug|info)" }
]
action = {
  type = "block_with_feedback",
  feedback_message = "[Security] Remove console statements before committing"
}

[[policy]]
name = "Use design system components"
hook_event = "PreToolUse"
matcher = "Write|Edit"
conditions = [
  { type = "filepath_regex", value = "\\.tsx$" },
  { type = "file_content_regex", value = "<(button|a|input|select)\\s" }
]
action = {
  type = "provide_feedback",
  message = "[Style] Use design system components: <Button>, <Link>, <Input>, <Select>"
}

[[policy]]
name = "Run prettier on TSX files"
hook_event = "PostToolUse"
matcher = "Write|Edit"
conditions = [
  { type = "filepath_regex", value = "\\.tsx$" }
]
action = {
  type = "run_command",
  command = "prettier --write {{tool_input.file_path}}",
  background = true
}
```

When editing a file with both `console.log` and `<button>`:
- **Pass 1:** Collects feedback from design system policy
- **Pass 2:** Finds console.log block (hard action) and stops
- **Result:** Operation blocked with both messages:
  - [Security] Remove console statements before committing
  - [Style] Use design system components: <Button>, <Link>, <Input>, <Select>
- Prettier won't run because operation was blocked

### Example 5: Must Read Documentation Before Editing

This example shows how Cupcake's automatic state tracking simplifies policies:

```toml
[[policy]]
name = "Must read context before editing"
hook_event = "PreToolUse"
matcher = "Edit|Write"
conditions = [
  { type = "filepath_regex", value = "src/core/engine\\.rs" },
  { type = "state_missing", tool = "Read", path = "docs/engine-architecture.md" }
]
action = {
  type = "block_with_feedback",
  feedback_message = "You must read docs/engine-architecture.md before modifying the engine code"
}
```

No explicit tracking policy needed! Cupcake automatically records all Read operations, so it knows whether the documentation was read in this session.

### Example 6: Complex Stateful Workflow

For more complex workflows that need custom events:

```toml
[[policy]]
name = "Mark feature as reviewed"
hook_event = "PostToolUse"
matcher = "Bash"
conditions = [
  { type = "command_regex", value = "code-review --approve" }
]
action = {
  type = "update_state",
  event = "FeatureReviewed",
  data = { reviewer = "{{env.USER}}", timestamp = "{{now}}" }
}

[[policy]]
name = "Require review before merge"
hook_event = "PreToolUse"
matcher = "Bash"
conditions = [
  { type = "command_regex", value = "git\\s+merge" },
  { type = "state_missing", event = "FeatureReviewed" }
]
action = {
  type = "block_with_feedback",
  feedback_message = "Cannot merge without code review approval. Run 'code-review --approve' first."
}
```

## Template Variables

Available in action parameters:

- `{{tool_name}}` - Name of the tool being called
- `{{tool_input.FIELD}}` - Fields from tool input (e.g., file_path, command)
- `{{session_id}}` - Current Claude session ID
- `{{now}}` - Current timestamp
- `{{env.VAR}}` - Environment variables
- `{{match.N}}` - Regex capture groups from conditions

## Two-Pass Evaluation Model

### How It Works

Cupcake evaluates policies using a two-pass model on a single, ordered list:

1. **Policy Loading:** Project policies (`./cupcake.toml`) followed by User policies (`~/.claude/cupcake.toml`)
2. **Pass 1:** Iterate through ALL policies, collecting feedback from every match
3. **Pass 2:** Re-iterate to find the FIRST hard action (block/approve)
4. **Result:** Combine all feedback with the final decision

### Example: Multiple Style Rules

```toml
# In project cupcake.toml
[[policy]]
name = "Use Button component"
conditions = [{ type = "file_content_regex", value = "<button" }]
action = { type = "provide_feedback", message = "• Use <Button> not <button>" }

# In user cupcake.toml  
[[policy]]
name = "Use Link component"
conditions = [{ type = "file_content_regex", value = "<a\\s" }]
action = { type = "provide_feedback", message = "• Use <Link> not <a>" }
```

**Result when both match:**
```
Policy Feedback:
• Use <Button> not <button>
• Use <Link> not <a>
```

### Example: Conflict Resolution

```toml
# In project cupcake.toml
[[policy]]
name = "Block commits during code freeze"
conditions = [{ type = "command_regex", value = "git commit" }]
action = { type = "block_with_feedback", feedback_message = "Code freeze active" }

# In user cupcake.toml
[[policy]]
name = "Allow my commits"
conditions = [{ type = "command_regex", value = "git commit" }]
action = { type = "approve" }
```

**Result:** Project's block wins (first hard action in the ordered list)

## Validation Rules

1. Each policy must have: name, hook_event, conditions, action
2. Matcher patterns must be valid regex
3. State event names must be alphanumeric + underscores
4. Template variables must reference valid fields
5. Time windows must be valid HH:MM format

## Future Extensions

Reserved fields for future use:
- `enabled` - Toggle policies on/off
- `tags` - Categorize policies
- `owner` - Policy ownership for enterprises
- `expires` - Time-based policy expiration