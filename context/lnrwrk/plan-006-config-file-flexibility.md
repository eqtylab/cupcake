# plan 006: Config File Flexibility and Test Support

Created: 2025-07-12T18:00:00Z
Depends: plan-005
Enables: Future DSL support, Enhanced testing capabilities

## Goal

Enable flexible policy configuration loading that supports both production `guardrails/` directory structure and standalone policy files for testing and development. Restore the ability to specify custom policy files via command line while maintaining the YAML format.

## Success Criteria

- `cupcake run --config path/to/policy.yaml` loads and enforces policies from any YAML file
- Existing `guardrails/cupcake.yaml` auto-discovery continues to work when no config specified
- Integration tests can specify isolated policy files without full directory structure
- Support both RootConfig (with imports) and bare PolicyFragment formats
- All existing tests pass, including `test_run_command_with_policy_evaluation`

## Context

During Plan 005 YAML migration, we lost the ability to specify custom policy files. The `--policy-file` parameter exists but is ignored. This breaks integration tests and limits development flexibility. We need to restore this capability while embracing the new YAML format's composability.

## Design Approach

### Current State
```rust
// Currently: policy_file parameter is ignored
fn load_policies(&self) -> Result<Vec<ComposedPolicy>> {
    let current_dir = std::env::current_dir()?;
    loader.load_and_compose_policies(&current_dir)  // Always uses auto-discovery
}
```

### Proposed Implementation
The system should support multiple configuration patterns:

**1. Single file with everything**
```yaml
# my-complete-policy.yaml
settings:
  audit_logging: true

# Inline policies right in the same file!
PreToolUse:
  "Bash":
    - name: "My rule"
      conditions: [...]
      action: {...}
```

**2. Root file referencing others anywhere**
```yaml
# /home/user/my-policies/root.yaml
settings:
  debug_mode: true
imports:
  - "/etc/cupcake/global-policies/*.yaml"
  - "~/my-team/policies/*.yaml"
  - "./local-overrides/*.yaml"
```

**3. Just a fragment (for testing)**
```yaml
# just-my-rules.yaml - no settings, no imports, just policies
PreToolUse:
  "Edit":
    - name: "Python formatting"
      conditions: [...]
      action: {...}
```

### Loading Logic
```rust
fn load_policies(&self) -> Result<Vec<ComposedPolicy>> {
    if !self.policy_file.is_empty() {
        // User specified a file
        let content = read_file(&self.policy_file)?;
        
        // Try to parse as RootConfig first
        if let Ok(root_config) = serde_yaml::from_str::<RootConfig>(&content) {
            // It's a full config - process imports relative to THIS file's location
            return load_from_root_config(root_config, &self.policy_file);
        }
        
        // Otherwise try as a PolicyFragment
        if let Ok(fragment) = serde_yaml::from_str::<PolicyFragment>(&content) {
            // Just a fragment - use default settings
            return Ok(compose_and_flatten(fragment));
        }
        
        // If neither works, return parse error
    } else {
        // No file specified - use auto-discovery
        return discover_and_load();
    }
}
```

## Industry Standards Alignment

### Naming Convention
- Change `--policy-file` to `--config` (follows ESLint, Webpack, TypeScript patterns)
- Support `-c` short form
- Check `CUPCAKE_CONFIG` environment variable as fallback

### Configuration Cascade
Following established patterns from successful tools:

1. **Explicit override**: `cupcake run --config ./my-config.yaml`
2. **Environment variable**: `CUPCAKE_CONFIG=/path/to/config.yaml cupcake run`
3. **Convention-based discovery**: `./guardrails/cupcake.yaml` (current behavior)

### Future Extensibility
The PolicyFragment structure serves as an AST for future DSLs:

```typescript
// Future TypeScript DSL example
const policy = cupcake.policy({
  event: 'PreToolUse',
  tool: 'Bash',
  rules: [
    cupcake.block().when(cmd => cmd.matches(/^rm/))
  ]
});

// This compiles to PolicyFragment YAML
```

## Implementation Steps

1. **Rename parameter**: Change `policy_file` to `config` throughout
2. **Update PolicyLoader**: Add methods for loading single files
3. **Handle both formats**: Support RootConfig and PolicyFragment
4. **Fix path resolution**: Make imports relative to config file location
5. **Update tests**: Ensure integration tests work with new approach
6. **Add examples**: Document different configuration patterns

## Benefits

- **Testing**: Each test can use isolated policy files
- **Development**: Quick iteration without full directory structure  
- **CI/CD**: Different policies per environment via `CUPCAKE_CONFIG`
- **Modularity**: Teams can maintain separate policy files
- **Future-proof**: Foundation for package imports and DSLs

## Migration Notes

- Existing `guardrails/` users unaffected (default behavior unchanged)
- `--policy-file` parameter renamed to `--config` but could support both temporarily
- Integration tests need minimal changes (already using YAML format)