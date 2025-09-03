# Global Builtins Implementation Plan

## Overview
Two new global-level security builtins to protect the host system and prevent credential harvesting:
1. **System Protection** - Prevents modification of critical OS paths
2. **Sensitive Data Protection** - Blocks reading of credentials and secrets

## Current Builtin System Understanding

### How Builtins Work:
1. **Configuration**: Defined in `guidebook.yml` under `builtins:` section
2. **Policy Storage**: `.cupcake/policies/builtins/` directory  
3. **Selective Loading**: Scanner only loads policies for enabled builtins
4. **Signal Integration**: Builtins can receive dynamic configuration via signals
5. **Namespace**: Global builtins use `cupcake.global.policies.builtins.*`

### Key Components:
- `builtins.rs`: Configuration structures and signal generation
- `scanner.rs`: Filters policies based on enabled builtins
- Policy files: Standard Rego with metadata-driven routing

## Implementation Steps

### 1. Add Builtin Configurations to `builtins.rs`

```rust
// In BuiltinsConfig struct
pub struct BuiltinsConfig {
    // ... existing fields ...
    
    /// System protection configuration (global only)
    #[serde(default)]
    pub system_protection: Option<SystemProtectionConfig>,
    
    /// Sensitive data protection configuration (global only)
    #[serde(default)]
    pub sensitive_data_protection: Option<SensitiveDataConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemProtectionConfig {
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    
    #[serde(default)]
    pub additional_paths: Vec<String>,  // User can add more protected paths
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensitiveDataConfig {
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    
    #[serde(default)]
    pub additional_patterns: Vec<String>,  // User can add more patterns
}
```

### 2. Update `enabled_builtins()` Method

```rust
pub fn enabled_builtins(&self) -> Vec<String> {
    let mut enabled = Vec::new();
    
    // ... existing builtins ...
    
    if self.system_protection.as_ref().map_or(false, |c| c.enabled) {
        enabled.push("system_protection".to_string());
    }
    
    if self.sensitive_data_protection.as_ref().map_or(false, |c| c.enabled) {
        enabled.push("sensitive_data_protection".to_string());
    }
    
    enabled
}
```

### 3. Global Configuration Structure

When initializing global configuration in `global_config.rs`:

```
~/.config/cupcake/           # Linux
~/Library/Application Support/cupcake/  # macOS
%APPDATA%\cupcake\           # Windows
├── guidebook.yml
├── policies/
│   ├── system/
│   │   └── evaluate.rego
│   └── builtins/
│       ├── system_protection.rego
│       └── sensitive_data_protection.rego
├── signals/
└── actions/
```

### 4. Default Global `guidebook.yml`

```yaml
# Global Cupcake Configuration
# Applies to ALL projects on this machine

signals: {}
actions: {}

builtins:
  # Protect critical system paths from modification
  system_protection:
    enabled: true
    additional_paths: []  # Add custom protected system paths
  
  # Block reading of credentials and sensitive data
  sensitive_data_protection:
    enabled: true
    additional_patterns: []  # Add custom sensitive file patterns
```

### 5. Installation Process

The `cupcake init --global` command should:

1. Create the global config directory structure
2. Copy the builtin policy files to `policies/builtins/`
3. Create the default `guidebook.yml` with builtins enabled
4. Set appropriate permissions (read-only for policies)

### 6. Policy File Deployment

The builtin policy files should be:
- Bundled with the Cupcake binary or
- Downloaded from a secure repository during `init --global`
- Stored in `~/.config/cupcake/policies/builtins/`

## Security Considerations

### Preventing Cupcake Binary Execution
As you noted, we need a third builtin to prevent Claude Code from calling the cupcake binary directly (while allowing hooks):

```rego
# Block direct cupcake binary execution via Bash
deny contains decision if {
    input.hook_event_name == "PreToolUse"
    input.tool_name == "Bash"
    
    command := lower(input.tool_input.command)
    
    # Check for cupcake binary execution
    cupcake_patterns := {
        "cupcake ",
        "./cupcake ",
        "cargo run --bin cupcake",
        "target/release/cupcake",
        "target/debug/cupcake",
    }
    
    some pattern in cupcake_patterns
    contains(command, pattern)
    
    decision := {
        "rule_id": "GLOBAL-BUILTIN-NO-CUPCAKE-EXEC",
        "reason": "Direct execution of Cupcake binary is not permitted",
        "severity": "HIGH"
    }
}
```

This is different from hooks because:
- **Hooks**: Claude Code's built-in hook system calls Cupcake with structured JSON events
- **Direct execution**: User/AI trying to run `cupcake` commands via Bash tool

## Testing Strategy

### Unit Tests
1. Test builtin configuration parsing
2. Test enabled_builtins() returns correct list
3. Test policy loading with builtin filter

### Integration Tests  
1. Test system_protection blocks system path modifications
2. Test sensitive_data_protection blocks credential reads
3. Test that enabled=false properly disables builtins
4. Test global builtins take precedence over project policies

### Manual Testing
```bash
# Initialize global config
cupcake init --global

# Verify structure
ls -la ~/.config/cupcake/

# Test with Claude Code
# 1. Try to read ~/.ssh/id_rsa (should block)
# 2. Try to edit /etc/hosts (should block)
# 3. Try to search for *.env files (should block)
```

## Benefits

1. **Zero-configuration Security**: Users get protection immediately upon global init
2. **Machine-wide Protection**: Applies to ALL projects, can't be overridden
3. **Prevents Common Attacks**: 
   - System compromise via file modification
   - Credential theft via file reading
   - Secret discovery via glob patterns
4. **Extensible**: Users can add custom patterns via configuration

## Next Steps

1. ✅ Created `system_protection.rego` policy
2. ✅ Created `sensitive_data_protection.rego` policy  
3. ⏳ Update `builtins.rs` with new configurations
4. ⏳ Implement `cupcake init --global` to deploy builtins
5. ⏳ Add third builtin for blocking cupcake binary execution
6. ⏳ Write tests for global builtin functionality
7. ⏳ Update documentation