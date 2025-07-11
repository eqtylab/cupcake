# Command Execution Patterns in Cupcake

## User Question

"Does this design support things like:

- file: \*.tsx (this is abstract, doesn't need to match perfectly)
- check: repo_command(pnpm lint <matched_file>.tsx)
- result: if fail / exit 0 - do whatever action built in with claude, maybe a custom action for other use cases? (custom.sh for example)
- check: custom_validation.sh <matched_file>.tsx
- result: if fail, etc (same as above)"

## Answer: Yes, Fully Supported

The Cupcake design fully supports running repository commands and custom scripts on matched files through the `run_command` action type with template variable substitution.

## Core Mechanism

The `run_command` action provides:

1. **File matching** via conditions (glob patterns, regex)
2. **Command execution** with template variables
3. **Exit code handling** to determine success/failure
4. **Conditional actions** based on command results

## Implementation Patterns

### 1. Basic Linting Pattern

```toml
[[policy]]
name = "Lint TypeScript files"
hook_event = "PreToolUse"
matcher = "Write|Edit"
conditions = [
  { type = "filepath_glob", value = "*.tsx" }
]
action = {
  type = "run_command",
  command = "pnpm lint {{tool_input.file_path}}",
  on_failure = "block",
  on_failure_feedback = "Linting failed:\n{{stderr}}"
}
```

### 2. Custom Validation Script

```toml
[[policy]]
name = "Custom validation"
hook_event = "PreToolUse"
matcher = "Write|Edit"
conditions = [
  { type = "filepath_glob", value = "*.tsx" }
]
action = {
  type = "run_command",
  command = "./scripts/custom_validation.sh {{tool_input.file_path}}",
  on_failure = "block",
  on_failure_feedback = "Custom validation failed:\n{{stdout}}\n{{stderr}}"
}
```

### 3. Multiple Checks in Sequence

```toml
[[policy]]
name = "Full validation pipeline"
hook_event = "PreToolUse"
matcher = "Write|Edit"
conditions = [
  { type = "filepath_glob", value = "src/**/*.tsx" }
]
action = {
  type = "run_command",
  command = """
    pnpm lint {{tool_input.file_path}} && \
    pnpm typecheck {{tool_input.file_path}} && \
    ./scripts/custom_validation.sh {{tool_input.file_path}}
  """,
  on_failure = "block",
  on_failure_feedback = "Validation failed. See errors above."
}
```

## Exit Code Behavior

The `on_failure` parameter controls what happens based on exit codes:

- **`on_failure = "block"`**: Non-zero exit code blocks the operation
- **`on_failure = "continue"`**: Non-zero exit code is logged but operation continues

```toml
# Example: Block on lint errors
action = {
  type = "run_command",
  command = "eslint {{tool_input.file_path}}",
  on_failure = "block",  # Exit code != 0 blocks the edit
  on_failure_feedback = "Fix these lint errors first:\n{{stderr}}"
}

# Example: Try to auto-fix but don't block
action = {
  type = "run_command",
  command = "prettier --write {{tool_input.file_path}}",
  on_failure = "continue",  # Exit code != 0 just logs warning
  background = true
}
```

## Template Variable System

### File-Related Variables

For Write/Edit/Read operations:

```toml
{{tool_input.file_path}}  # Full path to the file
{{tool_input.content}}    # New content (Write/Edit only)
```

### Command-Related Variables

For Bash operations:

```toml
{{tool_input.command}}      # The command being executed
{{tool_input.description}}  # Command description
```

### Global Variables

Available in all contexts:

```toml
{{tool_name}}      # Name of the tool (Write, Edit, Bash, etc.)
{{session_id}}     # Current Claude session ID
{{now}}            # Current timestamp
{{env.VAR}}        # Environment variables
{{match.N}}        # Regex capture groups from conditions
```

## Advanced Patterns

### 1. Different Commands for Different File Types

```toml
# Python files
[[policy]]
name = "Python validation"
matcher = "Write|Edit"
conditions = [
  { type = "filepath_glob", value = "*.py" }
]
action = {
  type = "run_command",
  command = "black --check {{tool_input.file_path}} && mypy {{tool_input.file_path}}",
  on_failure = "block"
}

# TypeScript files
[[policy]]
name = "TypeScript validation"
matcher = "Write|Edit"
conditions = [
  { type = "filepath_glob", value = "*.{ts,tsx}" }
]
action = {
  type = "run_command",
  command = "pnpm lint {{tool_input.file_path}} && pnpm typecheck",
  on_failure = "block"
}

# Rust files
[[policy]]
name = "Rust validation"
matcher = "Write|Edit"
conditions = [
  { type = "filepath_glob", value = "*.rs" }
]
action = {
  type = "run_command",
  command = "cargo fmt --check && cargo clippy -- -D warnings",
  on_failure = "block"
}
```

### 2. Pre and Post Processing

```toml
# Before edit: Check if file is locked
[[policy]]
name = "Check file lock"
hook_event = "PreToolUse"
matcher = "Edit"
conditions = [
  { type = "filepath_glob", value = "**/*.tsx" }
]
action = {
  type = "run_command",
  command = "./scripts/check-file-lock.sh {{tool_input.file_path}}",
  on_failure = "block",
  on_failure_feedback = "File is locked for editing"
}

# After edit: Update dependencies
[[policy]]
name = "Update component index"
hook_event = "PostToolUse"
matcher = "Write"
conditions = [
  { type = "filepath_glob", value = "src/components/**/index.tsx" }
]
action = {
  type = "run_command",
  command = "./scripts/update-component-index.sh {{tool_input.file_path}}",
  on_failure = "continue",
  background = true
}
```

### 3. Conditional Validation Based on Content

```toml
[[policy]]
name = "Validate API endpoints"
matcher = "Write|Edit"
conditions = [
  { type = "filepath_glob", value = "src/api/**/*.ts" },
  { type = "file_content_regex", value = "@endpoint|router\\." }
]
action = {
  type = "run_command",
  command = "./scripts/validate-api-endpoint.sh {{tool_input.file_path}}",
  on_failure = "block",
  on_failure_feedback = "API endpoint validation failed:\n{{stderr}}"
}
```

### 4. Integration with CI/CD Tools

```toml
[[policy]]
name = "Run affected tests"
hook_event = "PostToolUse"
matcher = "Write|Edit"
conditions = [
  { type = "filepath_glob", value = "src/**/*.{ts,tsx}" }
]
action = {
  type = "run_command",
  command = "nx affected:test --files={{tool_input.file_path}}",
  on_failure = "block",
  on_failure_feedback = "Tests failed for affected code:\n{{stdout}}"
}
```

## Best Practices

1. **Use Specific Globs**: More specific patterns reduce unnecessary command executions
2. **Chain Related Commands**: Use `&&` to ensure all checks pass
3. **Provide Clear Feedback**: Include `{{stdout}}` and `{{stderr}}` in feedback messages
4. **Consider Performance**: Use `background = true` for non-critical post-processing
5. **Handle Missing Tools**: Ensure required commands are available in the environment

## Limitations and Workarounds

### Current Limitations

1. **Single Action Per Policy**: Each policy can only have one action
2. **No Built-in Retry**: Failed commands aren't automatically retried
3. **Simple Exit Code Logic**: Only supports success (0) vs failure (non-zero)

### Workarounds

For complex logic, wrap in a script:

```bash
#!/bin/bash
# custom-validation.sh
set -e

file="$1"

# Run multiple checks
eslint "$file" || echo "ESLint warnings (non-blocking)"
tsc --noEmit || exit 1  # This will block
./business-logic-check.sh "$file" || exit 1

echo "All validations passed"
```

Then use in policy:

```toml
action = {
  type = "run_command",
  command = "./scripts/custom-validation.sh {{tool_input.file_path}}",
  on_failure = "block"
}
```
