# Security Tests - Safety Guidelines

## Core Principle
**Security tests must prove that malicious input is neutralized, not demonstrate actual attacks.**

## Mandatory Safety Practices

### ✅ SAFE Test Patterns
- **Unit tests only** - Test `build_graph()` parsing, not execution
- **Assert literal neutralization** - Verify malicious input becomes literal arguments
- **Use safe commands** - `echo`, `pwd`, `env` for validation
- **Template variable testing** - Prove `{{malicious_var}}` becomes literal string

### ❌ PROHIBITED Test Patterns
- **No actual destructive commands** - Never test `rm`, `dd`, `curl` execution
- **No real file system operations** - Use `/tmp` paths only if needed
- **No network operations** - No real HTTP requests or downloads
- **No privilege escalation** - No `sudo` or root operations

## Required Safety Validation
Before any security test:
1. **Review all command strings** - Ensure they become literal arguments
2. **Verify test scope** - Unit tests for parsing, not execution
3. **Check for `#[tokio::test]`** - Only use for proven-safe command execution
4. **Validate assertions** - Tests must prove neutralization, not demonstrate attacks

## Example Safe Pattern
```rust
// SAFE: Tests that malicious input becomes literal
let malicious = "; rm -rf /";
let graph = executor.build_graph(&spec).unwrap();
assert_eq!(node.command.args, vec!["; rm -rf /"]); // Literal, not executed
```

## Emergency Protocol
If any test could cause system damage, **immediately stop and review** with team before proceeding.

**Remember: We validate security by proving attacks fail, not by demonstrating they work.**