# Builtin Policies Reference

Cupcake provides 11 builtin policies that implement common security patterns without writing Rego code. Enable them via `cupcake init --builtins` or by editing `guidebook.yml`.

## Quick Reference

| Builtin | Scope | Purpose | Blocks |
|---------|-------|---------|--------|
| `always_inject_on_prompt` | Project | Add context to every prompt | Nothing |
| `global_file_lock` | Project | Prevent ALL file writes | All writes |
| `git_pre_check` | Project | Validate before git operations | Git ops if checks fail |
| `post_edit_check` | Project | Validate after file edits | Future edits if checks fail |
| `rulebook_security_guardrails` | Project | Protect `.cupcake/` directory | Reads & writes to protected paths |
| `protected_paths` | Project | User-defined read-only paths | Writes to specified paths |
| `git_block_no_verify` | Project | Prevent bypassing hooks | `--no-verify` flag |
| `enforce_full_file_read` | Project | Require full file reads | Partial reads of small files |
| `system_protection` | Global | Protect OS paths | Writes to system directories |
| `sensitive_data_protection` | Global | Protect credentials | Reads of sensitive files |
| `cupcake_exec_protection` | Global | Block cupcake execution | Direct cupcake binary calls |

## Project-Level Builtins

### always_inject_on_prompt

Injects additional context with every user prompt. Useful for project guidelines, coding standards, or reminders.

**Configuration:**
```yaml
builtins:
  always_inject_on_prompt:
    context:
      # Static text
      - "Follow SOLID principles"
      - "Write comprehensive tests"

      # Dynamic from command
      - command: "git status --short"

      # From file
      - file: ".cupcake/coding-standards.md"
```

**Use Cases:**
- Enforce coding standards
- Provide project context
- Add current state awareness

---

### global_file_lock

Prevents ALL file modifications session-wide. Most restrictive builtin.

**Configuration:**
```yaml
builtins:
  global_file_lock:
    message: "Session is in read-only mode"
```

**Use Cases:**
- Code review sessions
- Learning/exploration mode
- Production safety

---

### git_pre_check

Runs validation commands before git operations.

**Configuration:**
```yaml
builtins:
  git_pre_check:
    checks:
      - command: "cargo test"
        message: "Tests must pass before git operations"
      - command: "cargo fmt --check"
        message: "Code must be formatted"
```

**Events:** PreToolUse with Bash tool
**Triggers:** Commands containing `git commit`, `git push`, `git merge`

---

### post_edit_check

Runs validation after file edits based on file extension.

**Configuration:**
```yaml
builtins:
  post_edit_check:
    by_extension:
      "py":
        command: "python -m py_compile"
        message: "Python syntax error"
      "rs":
        command: "cargo check"
        message: "Rust compilation error"
      "ts":
        command: "npx tsc --noEmit"
        message: "TypeScript error"
```

**Events:** PostToolUse with Edit/Write/MultiEdit tools

---

### rulebook_security_guardrails

Total lockdown of critical configuration paths. Blocks both reads AND writes.

**Configuration:**
```yaml
builtins:
  rulebook_security_guardrails:
    message: "Cupcake configuration is protected"
    protected_paths:
      - ".cupcake/"
      - ".git/hooks/"
```

**Use Cases:**
- Prevent policy tampering
- Protect git hooks
- Secure CI/CD configs

---

### protected_paths

Makes specified paths read-only (read allowed, write blocked).

**Configuration:**
```yaml
builtins:
  protected_paths:
    message: "This path is read-only"
    paths:
      - "/etc/"
      - "/System/"
      - "~/.ssh/"
      - "*.production.yml"
```

**Supports:** Glob patterns
**Default paths when enabled via CLI:** `/etc/`, `/System/`, `~/.ssh/`

---

### git_block_no_verify

Prevents bypassing git hooks with `--no-verify` flag.

**Configuration:**
```yaml
builtins:
  git_block_no_verify:
    message: "Git hooks must run"
    exceptions: []  # Can allow specific contexts
```

**Blocks:** `git commit --no-verify`, `git push --no-verify`

---

### enforce_full_file_read

Requires reading entire files under a configurable line limit.

**Configuration:**
```yaml
builtins:
  enforce_full_file_read:
    max_lines: 2000  # Files under this must be read completely
    message: "Please read the entire file first"
```

**Use Cases:**
- Ensure context awareness
- Prevent cherry-picking code
- Enforce thorough review

## Global Builtins

Global builtins provide machine-wide security policies that apply to ALL projects.

### system_protection

Protects critical system paths from modification.

**Configuration:**
```yaml
builtins:
  system_protection:
    additional_paths:
      - "/custom/system/path"
    message: "System path access blocked"
```

**Default Protected Paths:**
- `/etc/`, `/System/`, `/Library/`
- `/usr/` (except `/usr/local/`)
- `/bin/`, `/sbin/`
- `/boot/`, `/dev/`, `/proc/`
- Windows: `C:\Windows\`, `C:\Program Files\`

---

### sensitive_data_protection

Blocks reading of credential and sensitive files.

**Configuration:**
```yaml
builtins:
  sensitive_data_protection:
    additional_patterns:
      - "*.pem"
      - "*_secret*"
```

**Default Protected Patterns:**
- SSH keys: `*/.ssh/*`, `*_rsa`, `*.pem`
- Cloud: `*/.aws/*`, `*/.gcloud/*`
- Env files: `.env*`, `*.env`
- Browser: `*/Cookies`, `*/Login Data`
- Crypto: `*/wallet.dat`, `*/.bitcoin/*`

---

### cupcake_exec_protection

Prevents direct execution of cupcake binary.

**Configuration:**
```yaml
builtins:
  cupcake_exec_protection:
    allowed_commands:
      - "version"
      - "help"
    message: "Direct cupcake execution blocked"
```

**Use Case:** Prevent recursive policy evaluation

## Builtin Priority

When multiple builtins could apply:

1. **Halt** (highest priority - stops immediately)
2. **Deny/Block** (prevents action)
3. **Ask** (requires user confirmation)
4. **Allow** (default if no deny)

## Enabling Builtins

### Via CLI (Recommended)

```bash
# Single builtin
cupcake init --builtins git_pre_check

# Multiple builtins
cupcake init --builtins git_pre_check,protected_paths,global_file_lock

# Global builtins
cupcake init --global --builtins system_protection
```

### Via Configuration

Edit `.cupcake/guidebook.yml`:

```yaml
builtins:
  git_pre_check:
    enabled: true  # Optional, defaults to true when configured
    checks:
      - command: "make test"
        message: "Tests must pass"
```

## Performance Notes

- Builtins compile to WASM for fast execution
- Only enabled builtins are loaded into memory
- Static configs are injected directly (no shell overhead)
- Dynamic signals spawn processes only when needed

## Troubleshooting

**Builtin not working?**
1. Check it's enabled in `guidebook.yml`
2. Verify with `cupcake verify --policy-dir .cupcake`
3. Enable debug logging: `cupcake eval --log-level debug`

**Too restrictive?**
- Disable temporarily: Set `enabled: false`
- Adjust configuration to be less broad
- Use `protected_paths` instead of `global_file_lock`

**Not restrictive enough?**
- Combine multiple builtins
- Write custom Rego policies for complex logic
- Use `rulebook_security_guardrails` for total lockdown