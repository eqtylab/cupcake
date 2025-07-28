# Command Execution

Cupcake provides secure, shell-free command execution with familiar syntax. Commands are executed directly via process spawning, eliminating shell injection vulnerabilities.

## Execution Modes

### Array Mode (Recommended)

Kubernetes-style arrays with explicit operators for composition. No shell involved.

```yaml
action:
  type: run_command
  spec:
    mode: array
    command: ["npm"]
    args: ["test", "--coverage"]
    workingDir: "./frontend"
    env:
      - name: NODE_ENV
        value: production
```

**Composition operators:**

| Operator | Shell Equivalent | Purpose |
|----------|-----------------|---------|
| `pipe` | `\|` | Connect stdout to next command |
| `redirectStdout` | `>` | Write stdout to file |
| `appendStdout` | `>>` | Append stdout to file |
| `redirectStderr` | `2>` | Redirect stderr to file |
| `mergeStderr` | `2>&1` | Merge stderr into stdout |
| `onSuccess` | `&&` | Run if exit code = 0 |
| `onFailure` | `\|\|` | Run if exit code ≠ 0 |

### String Mode

Shell-like syntax parsed into secure operations. Supports: `| > >> && ||`

```yaml
action:
  type: run_command
  spec:
    mode: string
    command: "npm test | grep PASS > results.log"
```

Parser rejects dangerous syntax:
- Command substitution: `$(...)`, `` `...` ``
- Unsupported redirects: `2>&1`, `<`, `<<`
- Glob patterns (passed literally in v1.0)

## Security Features

### No Shell Execution
Commands execute via direct process spawning (`execve`). Shell metacharacters in arguments are passed literally:

```yaml
# Safe - semicolon is just an argument
command: ["echo"]
args: ["test; rm -rf /"]  # Output: "test; rm -rf /"
```

### Template Safety
Templates (`{{var}}`) only expand in:
- Arguments
- Environment values
- File paths for redirects

Never in command paths:
```yaml
# ❌ Blocked
command: ["{{cmd}}"]  # Error: Templates not allowed in command paths

# ✅ Safe
command: ["cat"]
args: ["{{file_path}}"]
```

## Complex Examples

### Pipe Chain with Conditionals
```yaml
spec:
  mode: array
  command: ["docker", "ps"]
  args: ["-a"]
  pipe:
    - cmd: ["grep", "backend"]
    - cmd: ["awk", "{print $1}"]
  redirectStdout: "container-ids.txt"
  onSuccess:
    - command: ["echo", "Found backend containers"]
  onFailure:
    - command: ["echo", "No backend containers running"]
```

### Multi-stage Validation
```yaml
spec:
  mode: string
  command: "cargo fmt --check && cargo clippy -- -D warnings && cargo test"
```

## Template Variables

Available in all command contexts:

| Variable | Description | Example |
|----------|-------------|---------|
| `{{file_path}}` | Current file being processed | `/path/to/file.js` |
| `{{tool_name}}` | Tool invoking the command | `Bash`, `Write`, etc. |
| `{{session_id}}` | Claude session identifier | `abc-123-def` |
| `{{env.VAR}}` | Environment variable | `{{env.USER}}`, `{{env.PATH}}` |
| `{{env.CLAUDE_PROJECT_DIR}}` | Project root directory (when set) | `/Users/alice/myproject` |
| `{{match.N}}` | Regex capture group from conditions | `{{match.1}}` from pattern |

### Using $CLAUDE_PROJECT_DIR

The `CLAUDE_PROJECT_DIR` environment variable is automatically set by Claude Code when spawning hooks. This enables portable, project-aware policies:

```yaml
# Example: Run project-specific linter
action:
  type: run_command
  spec:
    mode: array
    command: ["{{env.CLAUDE_PROJECT_DIR}}/.cupcake/scripts/lint.sh"]
    args: ["{{file_path}}"]

# Example: Check if file exists relative to project root
conditions:
  - type: check
    spec:
      mode: array
      command: ["test"]
      args: ["-f", "{{env.CLAUDE_PROJECT_DIR}}/config/settings.json"]
    expect_success: true
```

## Exit Code Handling

Control flow based on exit codes:

```yaml
action:
  type: run_command
  spec:
    mode: string
    command: "npm test"
  on_failure: block  # Non-zero exit blocks operation
  on_failure_feedback: "Tests must pass:\n{{stderr}}"
```

Options:
- `block`: Stop operation on non-zero exit
- `continue`: Log failure but continue

## Performance

- Zero shell overhead
- Direct process spawning
- Async I/O for pipes/redirects
- Typical overhead: <1ms per command

## Migration Guide

From shell strings:
```yaml
# Old (vulnerable)
command: "cat $FILE | grep -v warning > output.txt"

# New (secure)
spec:
  mode: array
  command: ["cat"]
  args: ["{{file_path}}"]
  pipe:
    - cmd: ["grep", "-v", "warning"]
  redirectStdout: "output.txt"
```

Or use string mode for convenience:
```yaml
spec:
  mode: string
  command: "cat {{file_path}} | grep -v warning > output.txt"
```