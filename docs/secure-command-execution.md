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
PreToolUse:
  "Bash":
    - name: "secure-build"
      description: "Secure cargo build execution"
      conditions:
        - type: "pattern"
          field: "tool_input.command"
          regex: "cargo build"
      action:
        type: "run_command"
        spec:
          mode: array
          command: ["cargo"]
          args: ["build", "--release"]
          env:
            RUSTFLAGS: "-C target-cpu=native"
          redirect_stdout: "build.log"
          redirect_stderr: "error.log"
          on_failure:
            - command: ["notify-send"]
              args: ["Build failed"]
        on_failure: "continue"
        background: false
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

## Shell Mode (Escape Hatch)

For legacy scripts or complex shell-specific syntax, Cupcake provides an optional shell escape hatch:

```yaml
spec:
  mode: shell
  script: |
    set -euo pipefail
    source ~/.bashrc
    deploy-app --env=production
```

**⚠️ Security Warning**: Shell mode bypasses injection protections and requires explicit `allow_shell: true` configuration.

See [Shell Escape Hatch](shell-escape-hatch.md) for security controls, migration tools, and best practices.