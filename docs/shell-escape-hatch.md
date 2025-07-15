# Shell Escape Hatch

> Secure shell command execution with explicit governance and comprehensive auditing

## Overview

While Cupcake's secure `array:` and `string:` command formats eliminate shell injection vulnerabilities, some scenarios require actual shell execution:

- Legacy scripts with complex shell-specific syntax
- Build tools that generate shell commands dynamically
- System administration tasks requiring shell features
- Third-party tools that output shell commands

The shell escape hatch provides this capability with multiple security layers:

1. **Explicit opt-in** via `allow_shell` setting
2. **Comprehensive auditing** with correlation IDs
3. **Sandboxing controls** including UID drop and timeouts
4. **Migration tools** to convert shell to secure formats

## Security Warning ⚠️

**Shell commands bypass Cupcake's injection protections**. Before enabling shell:

1. Understand the security implications
2. Audit all shell command usage
3. Use `cupcake encode` to migrate to secure formats where possible
4. Enable comprehensive audit logging
5. Restrict shell access in production environments

## Configuration

### Basic Settings

```yaml
# guardrails/cupcake.yaml
settings:
  # SECURITY: Must be explicitly enabled (default: false)
  allow_shell: true
  
  # RECOMMENDED: Always enable audit logging with shell
  audit_logging: true
  
  # OPTIONAL: Drop privileges for shell commands
  sandbox_uid: "nobody"  # or numeric: "65534"
  
  # OPTIONAL: Command timeout in milliseconds (default: 30000)
  timeout_ms: 60000
```

### Environment-Specific Configuration

**Development** (more permissive):
```yaml
settings:
  allow_shell: true
  audit_logging: true
  debug_mode: true  # Skip UID drop for debugging
```

**Production** (restrictive):
```yaml
settings:
  allow_shell: false  # Block all shell commands
  audit_logging: true
```

## Shell Command Format

### YAML Syntax

```yaml
policies:
  - name: "Run deployment script"
    conditions:
      - tool: "Task"
    action:
      run_command:
        spec:
          mode: shell
          script: |
            set -euo pipefail
            source ~/.bashrc
            deploy-app --env=production
```

### JSON Syntax

```json
{
  "spec": {
    "mode": "shell",
    "script": "npm test && npm run build"
  }
}
```

## Security Controls

### 1. Governance (allow_shell)

Shell execution is **disabled by default**. Attempting to run shell commands without enabling `allow_shell` results in:

```
Error: Shell command execution is disabled. Set allow_shell=true in settings to enable.
```

### 2. Sandboxing

**Privilege Dropping** (Unix/Linux):
- Configurable UID via `sandbox_uid` setting
- Supports numeric UIDs (65534) or usernames ("nobody")
- Automatic GID adjustment to match UID
- Skipped in debug_mode for development

**Timeout Protection**:
- Default: 30 seconds
- Configurable via `timeout_ms`
- Prevents runaway scripts
- Applies to all commands (not just shell)

### 3. Audit Logging

Every shell execution is logged to `~/.cupcake/audit/exec-YYYYMMDD.jsonl`:

```json
{
  "graph": "550e8400-e29b-41d4-a716-446655440000",
  "mode": "shell",
  "argv": ["/bin/sh", "-c", "echo 'Hello World'"],
  "cwd": "/home/user/project",
  "env": {},
  "timestamp": "2025-01-15T10:00:00Z",
  "exit_code": 0,
  "duration_ms": 25,
  "shell_used": true
}
```

Key audit fields:
- `graph`: Unique execution ID for correlation
- `mode`: Command type (array/string/shell)
- `shell_used`: Boolean flag for shell usage
- `argv`: Actual command array passed to OS
- `duration_ms`: Execution time for performance monitoring

## Migration with `cupcake encode`

Convert shell commands to secure array format:

### Basic Usage

```bash
# Simple command
$ cupcake encode "echo 'Hello World'"
```

Output:
```yaml
command:
- echo
args:
- Hello World
```

### Complex Examples

**Pipes**:
```bash
$ cupcake encode "ps aux | grep node | awk '{print $2}'"
```

Output:
```yaml
command:
- ps
args:
- aux
pipe:
- cmd:
  - grep
  - node
- cmd:
  - awk
  - '{print $2}'
```

**Redirects**:
```bash
$ cupcake encode "echo 'test' > output.txt"
```

Output:
```yaml
command:
- echo
args:
- test
redirect_stdout: output.txt
```

### Format Options

```bash
# YAML output (default)
cupcake encode "npm test"

# JSON output
cupcake encode "npm test" --format json

# Full template with metadata
cupcake encode "npm test" --template
```

## Claude Code Integration

### Hook Configuration

Cupcake integrates seamlessly with Claude Code hooks:

```json
{
  "hooks": {
    "PreToolUse": [{
      "matcher": "Bash",
      "hooks": [{
        "type": "command",
        "command": "cupcake run --hook-mode"
      }]
    }]
  }
}
```

### Policy Example

Block shell commands in production:

```yaml
policies:
  - name: "Block shell in production"
    hook_event: PreToolUse
    matcher: "Bash"
    conditions:
      - state_exists: "production_env"
    action:
      block_with_feedback:
        feedback_message: "Shell commands not allowed in production. Use array format."
```

## Best Practices

### 1. Progressive Security

Start restrictive and relax as needed:

1. Begin with `allow_shell: false`
2. Convert commands using `cupcake encode`
3. Enable shell only for specific environments
4. Monitor audit logs regularly

### 2. Shell Script Guidelines

When shell is necessary:

```bash
#!/bin/sh
# Always use 'set -euo pipefail' for safety
set -euo pipefail

# Quote all variables
echo "Processing file: ${FILE_PATH}"

# Use explicit paths
/usr/bin/npm test

# Check return codes
if ! command -v node >/dev/null 2>&1; then
    echo "Error: node not found" >&2
    exit 1
fi
```

### 3. Audit Log Monitoring

Regular audit review script:

```bash
#!/bin/bash
# Find all shell executions today
AUDIT_DIR="$HOME/.cupcake/audit"
TODAY=$(date +%Y%m%d)

echo "Shell executions today:"
jq -r 'select(.shell_used == true) | 
  "\(.timestamp) | \(.argv | join(" ")) | Exit: \(.exit_code)"' \
  "$AUDIT_DIR/exec-$TODAY.jsonl"
```

### 4. Environment Isolation

Use different policies per environment:

```yaml
# Development allows shell
imports:
  - policies/dev/*.yaml

# Production blocks shell
imports:
  - policies/prod/*.yaml
```

## Troubleshooting

### Shell Command Blocked

**Error**: "Shell command execution is disabled"
**Solution**: Set `allow_shell: true` in settings

### UID Drop Failures

**Error**: "Unknown user: nobody"
**Solution**: Use numeric UID (65534) instead of username

### Timeout Issues

**Error**: "Command execution timeout"
**Solution**: Increase `timeout_ms` for long-running scripts

### Audit Logs Missing

**Issue**: No logs in ~/.cupcake/audit/
**Solution**: Enable `audit_logging: true` in settings

## Security Considerations

1. **Template Injection**: Shell templates can execute arbitrary commands
2. **Environment Variables**: Shell inherits full environment
3. **Path Traversal**: Shell can access any user-accessible file
4. **Resource Consumption**: No built-in CPU/memory limits
5. **Signal Handling**: Shell scripts may ignore signals

Always prefer `array:` or `string:` formats when possible. Use `shell:` only when absolutely necessary and with appropriate controls.

## Future Enhancements

Planned security improvements:

1. **Seccomp Filters**: Restrict system calls available to shell
2. **Resource Limits**: CPU, memory, and I/O restrictions
3. **Command Allowlists**: Restrict which shell commands can run
4. **Pattern Detection**: Warn on dangerous shell patterns
5. **Rate Limiting**: Prevent shell command flooding

## Related Documentation

- [Secure Command Execution](secure-command-execution.md) - Array and string formats
- [Policy Format](policy-format.md) - Writing security policies
- [Claude Code Hooks](https://docs.anthropic.com/en/docs/claude-code/hooks) - Integration guide