# Secure Command Execution

Cupcake eliminates shell injection vulnerabilities through direct process spawning with Kubernetes-style array syntax.

## Array Mode

The `array:` mode provides shell-free command execution with composition operators for common shell patterns.

### Basic Syntax

```yaml
action:
  type: run_command
  spec:
    mode: array
    command: ["/usr/bin/git"]
    args: ["status", "-s"]
    workingDir: /home/project
    env:
      - name: GIT_PAGER
        value: cat
```

### Composition Operators

Seven operators replace shell metacharacters without invoking a shell:

| Operator | Shell Equivalent | Purpose |
|----------|-----------------|---------|
| `pipe` | `\|` | Connect stdout to next command's stdin |
| `redirectStdout` | `>` | Write stdout to file (truncate) |
| `appendStdout` | `>>` | Append stdout to file |
| `redirectStderr` | `2>` | Write stderr to file |
| `mergeStderr` | `2>&1` | Merge stderr into stdout |
| `onSuccess` | `&&` | Run if exit code == 0 |
| `onFailure` | `\|\|` | Run if exit code != 0 |

### Examples

#### Pipe Chain
```yaml
spec:
  mode: array
  command: ["npm"]
  args: ["test"]
  pipe:
    - cmd: ["grep", "-v", "WARNING"]
    - cmd: ["tee", "test.log"]
```

#### Conditional Execution
```yaml
spec:
  mode: array
  command: ["cargo"]
  args: ["test"]
  onSuccess:
    - command: ["echo"]
      args: ["Tests passed"]
  onFailure:
    - command: ["echo"]
      args: ["Tests failed"]
      redirectStderr: "error.log"
```

#### Complex Composition
```yaml
spec:
  mode: array
  command: ["docker"]
  args: ["ps", "-a"]
  pipe:
    - cmd: ["grep", "backend"]
    - cmd: ["awk", "{print $1}"]
  redirectStdout: "containers.txt"
  mergeStderr: true
```

### Template Variables

Templates are substituted only in safe contexts:
- ✅ Arguments (`args`)
- ✅ Environment values (`env[].value`)
- ✅ Working directory (`workingDir`)
- ❌ Command paths (security protection)

```yaml
spec:
  mode: array
  command: ["cat"]  # No templates allowed here
  args: ["{{file_path}}"]  # Templates OK in args
  env:
    - name: SESSION_ID
      value: "{{session_id}}"  # Templates OK in env values
```

### Security Model

1. **Direct Process Spawning**: Uses `execve()`/`CreateProcess()` directly
2. **No Shell Parsing**: Metacharacters like `;`, `&`, `$()` are literal strings
3. **Template Safety**: Command paths cannot contain template variables
4. **Process Isolation**: Each command runs in its own process

### YAML Format

```yaml
policies:
  - name: secure-build
    trigger:
      event: tool_called
      name: bash
      pattern: "cargo build"
    actions:
      - type: run_command
        spec:
          mode: array
          command: ["cargo"]
          args: ["build", "--release"]
          env:
            - name: RUSTFLAGS
              value: "-C target-cpu=native"
          redirectStdout: "build.log"
          redirectStderr: "error.log"
          onFailure:
            - command: ["notify-send"]
              args: ["Build failed"]
```

### Error Handling

Commands fail with clear error messages:
- Empty command arrays
- Template syntax in command paths
- Invalid file paths for redirects
- Process spawn failures

### Performance

- Sub-100ms execution overhead
- Direct process spawning (no shell startup)
- Async I/O for pipes and redirects
- Proper child process reaping (no zombies)