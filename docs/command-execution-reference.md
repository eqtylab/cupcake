# Command Execution Reference

## CommandSpec Structure

```rust
pub enum CommandSpec {
    Array(ArrayCommandSpec),
}
```

## ArrayCommandSpec Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `command` | `Vec<String>` | ✅ | Program to execute (first element) and initial args |
| `args` | `Option<Vec<String>>` | ❌ | Additional arguments |
| `workingDir` | `Option<String>` | ❌ | Working directory (defaults to current) |
| `env` | `Option<Vec<EnvVar>>` | ❌ | Environment variables (extends system env) |
| `pipe` | `Option<Vec<PipeCommand>>` | ❌ | Commands to pipe stdout through |
| `redirectStdout` | `Option<String>` | ❌ | File to write stdout (truncate) |
| `appendStdout` | `Option<String>` | ❌ | File to append stdout |
| `redirectStderr` | `Option<String>` | ❌ | File to write stderr |
| `mergeStderr` | `Option<bool>` | ❌ | Merge stderr into stdout |
| `onSuccess` | `Option<Vec<ArrayCommandSpec>>` | ❌ | Commands to run if exit code == 0 |
| `onFailure` | `Option<Vec<ArrayCommandSpec>>` | ❌ | Commands to run if exit code != 0 |

## Execution Flow

1. **Graph Construction**
   - Validate command array not empty
   - Check command path for template syntax (rejected)
   - Build internal CommandGraph representation

2. **Process Spawning**
   - Direct `tokio::process::Command` usage
   - No shell involvement
   - Proper stdio configuration for pipes/redirects

3. **I/O Handling**
   - Async pipe chains without deadlocks
   - File operations with proper permissions
   - Stderr merging when requested

4. **Exit Code Processing**
   - Capture and propagate exit codes
   - Evaluate conditional execution
   - Return final execution result

## Template Substitution Rules

| Context | Allowed | Example |
|---------|---------|---------|
| Command path | ❌ | `command: ["{{cmd}}"]` → Error |
| Arguments | ✅ | `args: ["--file={{path}}"]` → OK |
| Environment values | ✅ | `value: "{{session_id}}"` → OK |
| Working directory | ✅ | `workingDir: "{{project_dir}}"` → OK |
| Pipe commands | ❌ | `cmd: ["{{filter}}"]` → Error |
| File paths | ✅ | `redirectStdout: "{{log_file}}"` → OK |

## Migration from String Commands

### Before (vulnerable)
```yaml
action:
  type: run_command
  command: "echo $USER && ls -la | grep important"
```

### After (secure)
```yaml
action:
  type: run_command
  spec:
    mode: array
    command: ["sh", "-c", "echo $USER"]  # If shell needed
    onSuccess:
      - command: ["ls", "-la"]
        pipe:
          - cmd: ["grep", "important"]
```

### Better (shell-free)
```yaml
action:
  type: run_command
  spec:
    mode: array
    command: ["echo"]
    args: ["{{user}}"]
    onSuccess:
      - command: ["ls", "-la"]
        pipe:
          - cmd: ["grep", "important"]
```

## Common Patterns

### Run Tests with Filtered Output
```yaml
spec:
  mode: array
  command: ["npm", "test"]
  pipe:
    - cmd: ["grep", "-v", "PASS"]
  redirectStdout: "failures.log"
```

### Build with Error Notification
```yaml
spec:
  mode: array
  command: ["make", "all"]
  redirectStdout: "/dev/null"
  onFailure:
    - command: ["notify-send"]
      args: ["Build Failed", "Check error.log"]
```

### Safe File Processing
```yaml
spec:
  mode: array
  command: ["find", ".", "-name", "*.tmp"]
  pipe:
    - cmd: ["xargs", "-r", "rm", "-f"]
```

## Best Practices

1. **Always use array mode** for new policies
2. **Avoid shell wrappers** - use native commands
3. **Template safety** - never in command paths
4. **Exit code handling** - use onSuccess/onFailure
5. **Resource cleanup** - commands complete before continuing