# Trust System Resolution Plan

## Executive Summary

This plan addresses the critical mismatch between trust system documentation and implementation, prioritizing user experience and system integrity.

## Current Issues

1. **Critical Error Message Bug**: Error messages reference non-existent `cupcake trust reset` command
2. **Missing Commands**: Three documented commands (`disable`, `enable`, `reset`) are not implemented
3. **No Graceful Recovery**: Users encountering trust issues have no clean resolution path
4. **Test Coverage Gap**: No tests verify command availability or error message accuracy

## Resolution Strategy

### Phase 1: Critical Error Message Fix (Immediate)

**File**: `cupcake-core/src/trust/error.rs`

**Current Problem**:
```rust
#[error("SECURITY ALERT: Trust manifest has been tampered with!\n\n...
3. Re-initialize trust with: cupcake trust reset && cupcake trust init")]
```

**Fix**:
```rust
#[error("SECURITY ALERT: Trust manifest has been tampered with!\n\n...
3. Re-initialize trust by removing .cupcake/.trust and running: cupcake trust init")]
```

**Timeline**: 15 minutes

### Phase 2: Implement Missing Commands

#### 2.1 Add Command Definitions

**File**: `cupcake-cli/src/trust_cli.rs`

Add three new command variants to the `TrustCommand` enum:

```rust
/// Temporarily disable trust verification
Disable {
    #[clap(long, default_value = ".")]
    project_dir: PathBuf,
}

/// Re-enable trust verification
Enable {
    #[clap(long, default_value = ".")]
    project_dir: PathBuf,
    
    #[clap(long)]
    verify: bool,
}

/// Remove trust manifest and disable trust mode
Reset {
    #[clap(long, default_value = ".")]
    project_dir: PathBuf,
    
    #[clap(long)]
    force: bool,
}
```

#### 2.2 Add Trust Mode Support

**File**: `cupcake-core/src/trust/manifest.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TrustMode {
    Enabled,
    Disabled,
}

impl TrustManifest {
    pub fn set_mode(&mut self, mode: TrustMode) -> Result<()> {
        self.mode = mode;
        self.timestamp = Utc::now();
        // Recompute HMAC with new mode
        self.update_hmac()?;
        Ok(())
    }
    
    pub fn is_enabled(&self) -> bool {
        matches!(self.mode, TrustMode::Enabled)
    }
}
```

#### 2.3 Implement Command Logic

**File**: `cupcake-cli/src/trust_cli.rs`

```rust
async fn trust_disable(project_dir: &Path) -> Result<()> {
    let trust_file = project_dir.join(".cupcake/.trust");
    
    if !trust_file.exists() {
        println!("ℹ️  Trust is not initialized");
        return Ok(());
    }
    
    let mut manifest = TrustManifest::load(&trust_file)?;
    manifest.set_mode(TrustMode::Disabled)?;
    manifest.save(&trust_file)?;
    
    println!("⚠️  Trust verification DISABLED");
    println!("   Scripts will execute without integrity checks");
    println!("   Run 'cupcake trust enable' to re-enable");
    
    Ok(())
}

async fn trust_enable(project_dir: &Path, verify: bool) -> Result<()> {
    let trust_file = project_dir.join(".cupcake/.trust");
    
    if !trust_file.exists() {
        println!("❌ No trust manifest found");
        println!("   Run 'cupcake trust init' first");
        return Ok(());
    }
    
    let mut manifest = TrustManifest::load(&trust_file)?;
    
    if verify {
        println!("🔍 Verifying all scripts...");
        let verifier = TrustVerifier::with_manifest(manifest.clone(), project_dir);
        // Perform verification of all scripts
        match verifier.verify_all().await {
            Ok(_) => println!("✅ All scripts verified"),
            Err(e) => {
                println!("❌ Verification failed: {}", e);
                println!("   Run 'cupcake trust update' to approve changes");
                return Err(e.into());
            }
        }
    }
    
    manifest.set_mode(TrustMode::Enabled)?;
    manifest.save(&trust_file)?;
    
    println!("✅ Trust verification ENABLED");
    Ok(())
}

async fn trust_reset(project_dir: &Path, force: bool) -> Result<()> {
    use std::io::{self, Write};
    
    let trust_file = project_dir.join(".cupcake/.trust");
    
    if !trust_file.exists() {
        println!("ℹ️  No trust manifest to reset");
        return Ok(());
    }
    
    if !force {
        print!("⚠️  This will delete the trust manifest and disable integrity verification.\n");
        print!("   All script approvals will be lost.\n");
        print!("Continue? [y/N]: ");
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        
        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Cancelled");
            return Ok(());
        }
    }
    
    std::fs::remove_file(&trust_file)?;
    println!("🗑️  Trust manifest removed");
    println!("   Run 'cupcake trust init' to re-initialize");
    
    Ok(())
}
```

#### 2.4 Update Engine to Respect Trust Mode

**File**: `cupcake-core/src/engine/mod.rs`

```rust
// In Engine::initialize_trust()
match TrustManifest::load(&trust_path) {
    Ok(manifest) => {
        if manifest.is_enabled() {
            let verifier = TrustVerifier::new(&self.paths.root).await?;
            self.trust_verifier = Some(verifier);
            info!("Trust mode ENABLED - script integrity verification active");
        } else {
            info!("Trust mode DISABLED - scripts will execute without verification");
            self.show_trust_disabled_notification();
        }
    }
    Err(TrustError::NotInitialized) => {
        info!("Trust mode not initialized (optional)");
        self.show_trust_startup_notification();
    }
    Err(e) => {
        warn!("Failed to load trust manifest: {}", e);
    }
}
```

**Timeline**: 2-3 hours

### Phase 3: Critical Test Coverage

#### 3.1 Command Availability Test

**File**: `cupcake-cli/tests/trust_commands_test.rs`

```rust
#[test]
fn test_all_documented_commands_exist() {
    let expected_commands = vec![
        "init", "update", "verify", "list", 
        "disable", "enable", "reset"
    ];
    
    for cmd in expected_commands {
        let output = Command::new("cargo")
            .args(&["run", "--", "trust", cmd, "--help"])
            .output()
            .expect("Failed to run command");
        
        assert!(output.status.success(), 
            "Command 'trust {}' should exist", cmd);
    }
}
```

#### 3.2 Trust Mode Toggle Test

**File**: `cupcake-core/tests/trust_mode_test.rs`

```rust
#[tokio::test]
async fn test_trust_disable_enable_cycle() {
    let temp_dir = setup_test_project().await;
    
    // Initialize trust
    trust_init(&temp_dir).await.unwrap();
    
    // Verify trust is active
    let engine = Engine::new(&temp_dir).await.unwrap();
    assert!(engine.has_trust_enabled());
    
    // Disable trust
    trust_disable(&temp_dir).await.unwrap();
    
    // Verify trust is disabled but manifest exists
    let engine = Engine::new(&temp_dir).await.unwrap();
    assert!(!engine.has_trust_enabled());
    assert!(temp_dir.join(".cupcake/.trust").exists());
    
    // Re-enable trust
    trust_enable(&temp_dir, false).await.unwrap();
    
    // Verify trust is active again
    let engine = Engine::new(&temp_dir).await.unwrap();
    assert!(engine.has_trust_enabled());
}
```

#### 3.3 Reset Command Test

```rust
#[tokio::test]
async fn test_trust_reset_removes_manifest() {
    let temp_dir = setup_test_project().await;
    let trust_file = temp_dir.join(".cupcake/.trust");
    
    // Initialize trust
    trust_init(&temp_dir).await.unwrap();
    assert!(trust_file.exists());
    
    // Reset with force flag
    trust_reset(&temp_dir, true).await.unwrap();
    assert!(!trust_file.exists());
    
    // Verify engine loads without trust
    let engine = Engine::new(&temp_dir).await.unwrap();
    assert!(!engine.has_trust_enabled());
}
```

#### 3.4 Error Message Accuracy Test

```rust
#[test]
fn test_error_messages_reference_valid_commands() {
    let tampered_error = TrustError::ManifestTampered;
    let error_msg = format!("{}", tampered_error);
    
    // Verify it doesn't reference non-existent commands
    assert!(!error_msg.contains("trust reset"), 
        "Error should not reference non-existent 'trust reset' command");
    
    // Verify it provides actionable guidance
    assert!(error_msg.contains(".cupcake/.trust"));
    assert!(error_msg.contains("trust init"));
}
```

**Timeline**: 2 hours

### Phase 4: Update Error Message (After Implementation)

Once `trust reset` is implemented, update the error message back:

```rust
#[error("SECURITY ALERT: Trust manifest has been tampered with!\n\n...
3. Re-initialize trust with: cupcake trust reset --force && cupcake trust init")]
```

**Timeline**: 5 minutes

## Implementation Order

1. **Day 1 - Immediate**
   - Fix critical error message (15 min)
   - Implement command definitions (30 min)
   - Implement trust mode support (45 min)

2. **Day 1 - Core Implementation**
   - Implement disable command (30 min)
   - Implement enable command (45 min)
   - Implement reset command (30 min)
   - Update engine integration (30 min)

3. **Day 2 - Testing & Polish**
   - Write command availability tests (30 min)
   - Write mode toggle tests (45 min)
   - Write reset command tests (30 min)
   - Fix error message to reference new command (5 min)
   - Manual testing & edge cases (30 min)

## Success Criteria

- [ ] All 7 documented trust commands work as specified
- [ ] Error messages reference only existing commands
- [ ] Trust can be disabled/enabled without losing configuration
- [ ] Reset command safely removes manifest with confirmation
- [ ] Tests verify all commands exist and function correctly
- [ ] Engine respects disabled trust mode

## Risk Mitigation

1. **Backward Compatibility**: Existing manifests without mode field default to "enabled"
2. **Data Loss Prevention**: Reset command requires confirmation unless --force
3. **Security**: Disabled mode shows clear warning notifications
4. **Testing**: Each command has integration test coverage

## Notes

- Trust mode persists in manifest to survive restarts
- Disabled trust still loads manifest but skips verification
- Enable command can optionally verify all scripts first
- All commands maintain consistent emoji/formatting style

## Estimated Total Effort

- Implementation: 3.5 hours
- Testing: 2.5 hours
- Documentation sync: 0.5 hours
- **Total: 6.5 hours**

This plan ensures the trust system becomes fully functional with proper error handling and user experience.