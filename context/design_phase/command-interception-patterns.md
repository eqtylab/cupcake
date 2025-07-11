This doc supplements ./command-execution-patterns.md

## Command Interception Pattern

### Validating Commands Before Execution

A powerful pattern is intercepting commands (like git operations) before they execute, allowing custom validation logic:

```toml
[[policy]]
name = "Validate commit safety before committing"
hook_event = "PreToolUse"
matcher = "Bash"
conditions = [
  # Trigger this policy whenever Claude tries to run `git commit`
  { type = "command_regex", value = "^git commit" }
]
action = {
  # The action is simply to run our script.
  type = "run_command",
  # We pass the full command as an argument to the script.
  # Note: {{tool_input.command}} would contain 'git commit -m "WIP: stuff"'
  command = "./scripts/validate-commit.sh \"{{tool_input.command}}\"",

  # If the script exits with non-zero, block the operation.
  on_failure = "block",

  # Use the script's output as the feedback to Claude.
  on_failure_feedback = "{{stderr}}"
}
```

### How Command Interception Works

1. **PreToolUse hook fires** before the Bash tool executes
2. **Condition matches** the command pattern
3. **Validation script runs** with the full command as argument
4. **Exit code determines** whether original command proceeds
5. **Script output becomes** feedback to Claude if blocked

### Example Validation Script

```bash
#!/bin/bash
# validate-commit.sh

COMMAND="$1"

# Extract commit message
MESSAGE=$(echo "$COMMAND" | grep -oP '(?<=-m ")[^"]+')

# Check for WIP commits
if [[ "$MESSAGE" =~ ^WIP ]]; then
    echo "❌ WIP commits are not allowed on main branch" >&2
    echo "Please use a feature branch for work-in-progress" >&2
    exit 1
fi

# Check for conventional commits
if ! [[ "$MESSAGE" =~ ^(feat|fix|docs|style|refactor|test|chore): ]]; then
    echo "❌ Commit message must follow conventional format" >&2
    echo "Example: feat: add new validation system" >&2
    exit 1
fi

# Check if tests have been run recently
if ! [ -f ".last-test-run" ] || [ $(find ".last-test-run" -mmin +30) ]; then
    echo "❌ Tests haven't been run in the last 30 minutes" >&2
    echo "Run 'npm test' before committing" >&2
    exit 1
fi

# All checks passed
exit 0
```

### More Command Interception Examples

#### 1. Prevent Dangerous Operations

```toml
[[policy]]
name = "Block dangerous rm commands"
hook_event = "PreToolUse"
matcher = "Bash"
conditions = [
  { type = "command_regex", value = "^rm.*-rf.*/$|^rm.*-rf.*\\*" }
]
action = {
  type = "run_command",
  command = "./scripts/validate-rm-safety.sh \"{{tool_input.command}}\"",
  on_failure = "block",
  on_failure_feedback = "{{stderr}}"
}
```

#### 2. Validate Deployment Commands

```toml
[[policy]]
name = "Pre-deployment checks"
hook_event = "PreToolUse"
matcher = "Bash"
conditions = [
  { type = "command_regex", value = "deploy|release" }
]
action = {
  type = "run_command",
  command = "./scripts/pre-deploy-checks.sh \"{{tool_input.command}}\"",
  on_failure = "block",
  on_failure_feedback = "Deployment blocked:\n{{stderr}}"
}
```

#### 3. Branch Protection

```toml
[[policy]]
name = "Protect main branch"
hook_event = "PreToolUse"
matcher = "Bash"
conditions = [
  { type = "command_regex", value = "^git (push|merge).*main" }
]
action = {
  type = "run_command",
  command = "./scripts/check-branch-protection.sh \"{{tool_input.command}}\"",
  on_failure = "block",
  on_failure_feedback = "{{stderr}}"
}
```

#### 4. API Key Detection

```toml
[[policy]]
name = "Prevent API key exposure"
hook_event = "PreToolUse"
matcher = "Bash"
conditions = [
  { type = "command_regex", value = "^git (add|commit|push)" }
]
action = {
  type = "run_command",
  command = "./scripts/scan-for-secrets.sh",
  on_failure = "block",
  on_failure_feedback = "Secret detected! Details:\n{{stdout}}"
}
```

### Advanced Validation Script Patterns

#### Multi-Stage Validation

```bash
#!/bin/bash
# comprehensive-git-validate.sh

COMMAND="$1"

# Stage 1: Command syntax validation
if ! ./scripts/validate-git-syntax.sh "$COMMAND"; then
    echo "Invalid git command syntax" >&2
    exit 1
fi

# Stage 2: Permission check
if ! ./scripts/check-git-permissions.sh "$COMMAND"; then
    echo "Insufficient permissions for this operation" >&2
    exit 1
fi

# Stage 3: Business logic validation
if ! ./scripts/validate-business-rules.sh "$COMMAND"; then
    echo "Operation violates business rules" >&2
    exit 1
fi

# Stage 4: External service check (e.g., JIRA)
if ! ./scripts/check-jira-status.sh "$COMMAND"; then
    echo "Related JIRA ticket is not in correct status" >&2
    exit 1
fi

echo "All validation checks passed ✅"
exit 0
```

#### Context-Aware Validation

```bash
#!/bin/bash
# context-aware-commit.sh

COMMAND="$1"
CURRENT_BRANCH=$(git branch --show-current)
MODIFIED_FILES=$(git diff --name-only --cached)

# Different rules for different branches
if [[ "$CURRENT_BRANCH" == "main" ]]; then
    # Stricter rules for main branch
    if ! echo "$COMMAND" | grep -qE "^git commit -m '(fix|feat|docs):"; then
        echo "❌ Main branch requires conventional commits" >&2
        exit 1
    fi
elif [[ "$CURRENT_BRANCH" =~ ^feature/ ]]; then
    # Relaxed rules for feature branches
    if echo "$COMMAND" | grep -qi "TODO\|FIXME\|XXX"; then
        echo "⚠️  Warning: commit contains TODO markers" >&2
        # Don't block, just warn
    fi
fi

# Check modified files
for file in $MODIFIED_FILES; do
    if [[ "$file" =~ \.(test|spec)\. ]]; then
        continue  # Test files are OK
    fi

    # Ensure tests exist for source files
    test_file=$(echo "$file" | sed 's/\.\(js\|ts\)$/.test.\1/')
    if ! [ -f "$test_file" ]; then
        echo "❌ Missing test file for $file" >&2
        echo "Expected: $test_file" >&2
        exit 1
    fi
done

exit 0
```

### Best Practices for Command Interception

1. **Clear Error Messages**: Always output helpful messages to stderr
2. **Fast Execution**: Keep validation scripts fast to avoid hook timeouts
3. **Fail Safe**: In case of script errors, decide whether to block or allow
4. **Contextual Validation**: Use git state, environment variables, etc.
5. **Logging**: Consider logging intercepted commands for audit purposes

## Summary

The `run_command` action type with template variables provides a powerful and flexible system for integrating any repository tooling, linters, validators, or custom scripts into Cupcake's policy enforcement. The command interception pattern is particularly powerful for validating operations before they execute, ensuring that Claude Code respects your project's specific validation requirements and workflows.
