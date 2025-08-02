# Run Command Configuration Loading

When using `cupcake run --config <file>`, Cupcake supports two configuration formats:

## 1. Root Configuration (with settings and imports)

If your config file contains a `settings` section with non-default values or an `imports` section, it's treated as a root configuration file. In this case, policies must be defined in separate files and imported:

```yaml
# cupcake.yaml - Root config
settings:
  timeout_ms: 5000
  allow_shell: true
  debug_mode: false

imports:
  - "policies/*.yaml"
  - "specific-policy.yaml"
```

The imported files should contain policy fragments:

```yaml
# policies/security.yaml - Policy fragment
UserPromptSubmit:
  "*":
    - name: security-check
      conditions: []
      action:
        type: inject_context
        context: "Remember security best practices"
```

## 2. Policy Fragment (direct policy definitions)

If your config file doesn't have settings/imports (or only has default settings), you can define policies directly:

```yaml
# simple-policy.yaml - Direct policy fragment
UserPromptSubmit:
  "*":
    - name: context-injection
      conditions: []
      action:
        type: inject_context
        context: "Project context"
        
SessionStart:
  "startup":
    - name: welcome
      conditions: []
      action:
        type: inject_context
        context: "Welcome to the project"
```

## Important Notes

- If you need custom settings (like `timeout_ms` or `allow_shell`), you must use the root configuration format with imports
- The distinction is made by checking if the file has meaningful content beyond defaults
- This aligns with how Cupcake loads configurations from the `guardrails/` directory structure

## Examples

### Example 1: Simple policy file (works with --config)
```yaml
PreToolUse:
  "Bash":
    - name: validate-commands
      conditions: []
      action:
        type: allow
```

### Example 2: Config with settings (requires imports)
```yaml
settings:
  timeout_ms: 30000  # Non-default value triggers root config mode
  
# This won't work - policies must be in imported files
UserPromptSubmit:
  "*":
    - name: test
      conditions: []
      action:
        type: allow
```

### Example 3: Proper root config with imports
```yaml
settings:
  timeout_ms: 30000
  allow_shell: true
  
imports:
  - "my-policies.yaml"
```