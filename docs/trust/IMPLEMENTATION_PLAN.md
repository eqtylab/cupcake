# Trust System Missing Commands Implementation Plan

## Problem Statement

Three documented trust commands are missing from the implementation:
- `disable`: Temporarily disable trust verification  
- `enable`: Re-enable trust verification
- `reset`: Remove trust manifest and disable trust mode

The error messages even reference these non-existent commands!

## Implementation Strategy

### 1. Add Missing Command Variants to TrustCommand Enum

```rust
// In cupcake-cli/src/trust_cli.rs

#[derive(Parser, Debug)]
pub enum TrustCommand {
    // ... existing commands ...
    
    /// Temporarily disable trust verification
    Disable {
        /// Project directory (default: current directory)
        #[clap(long, default_value = ".")]
        project_dir: PathBuf,
    },
    
    /// Re-enable trust verification  
    Enable {
        /// Project directory (default: current directory)
        #[clap(long, default_value = ".")]
        project_dir: PathBuf,
        
        /// Verify all scripts before enabling
        #[clap(long)]
        verify: bool,
    },
    
    /// Remove trust manifest and disable trust mode
    Reset {
        /// Project directory (default: current directory)
        #[clap(long, default_value = ".")]
        project_dir: PathBuf,
        
        /// Skip confirmation prompt
        #[clap(long)]
        force: bool,
    },
}
```

### 2. Implement Command Logic

#### `trust_disable` Implementation
```rust
async fn trust_disable(project_dir: &Path) -> Result<()> {
    let trust_file = project_dir.join(".cupcake/.trust");
    
    if !trust_file.exists() {
        println!("ℹ️  Trust is not initialized");
        return Ok(());
    }
    
    // Load manifest and set mode to "disabled"
    let mut manifest = TrustManifest::load(&trust_file)?;
    manifest.set_mode(TrustMode::Disabled)?;
    manifest.save(&trust_file)?;
    
    println!("⚠️  Trust verification DISABLED");
    println!("   Scripts will execute without integrity checks");
    println!("   Run 'cupcake trust enable' to re-enable");
    
    Ok(())
}
```

#### `trust_enable` Implementation  
```rust
async fn trust_enable(project_dir: &Path, verify: bool) -> Result<()> {
    let trust_file = project_dir.join(".cupcake/.trust");
    
    if !trust_file.exists() {
        println!("❌ No trust manifest found");
        println!("   Run 'cupcake trust init' first");
        return Ok(());
    }
    
    // Load manifest and verify if requested
    let mut manifest = TrustManifest::load(&trust_file)?;
    
    if verify {
        println!("🔍 Verifying all scripts before enabling...");
        // Verification logic here
    }
    
    manifest.set_mode(TrustMode::Enabled)?;
    manifest.save(&trust_file)?;
    
    println!("✅ Trust verification ENABLED");
    Ok(())
}
```

#### `trust_reset` Implementation
```rust
async fn trust_reset(project_dir: &Path, force: bool) -> Result<()> {
    let trust_file = project_dir.join(".cupcake/.trust");
    
    if !trust_file.exists() {
        println!("ℹ️  No trust manifest to reset");
        return Ok(());
    }
    
    if !force {
        print!("⚠️  This will delete the trust manifest and disable integrity verification.\n");
        print!("Continue? [y/N]: ");
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        
        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Cancelled");
            return Ok(());
        }
    }
    
    fs::remove_file(&trust_file)?;
    println!("🗑️  Trust manifest removed");
    println!("   Run 'cupcake trust init' to re-initialize");
    
    Ok(())
}
```

### 3. Update TrustManifest to Support Modes

Add a `TrustMode` enum to track enabled/disabled state:

```rust
// In cupcake-core/src/trust/manifest.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TrustMode {
    Enabled,
    Disabled,
}

impl TrustManifest {
    pub fn set_mode(&mut self, mode: TrustMode) -> Result<()> {
        self.mode = mode;
        self.timestamp = Utc::now();
        Ok(())
    }
    
    pub fn is_enabled(&self) -> bool {
        matches!(self.mode, TrustMode::Enabled)
    }
}
```

### 4. Update Engine to Respect Trust Mode

```rust
// In engine initialization
if manifest.is_enabled() {
    self.trust_verifier = Some(verifier);
} else {
    info!("Trust manifest found but DISABLED");
}
```

## Testing Requirements

1. Test disable → enable flow maintains manifest integrity
2. Test reset removes manifest file correctly
3. Test disabled mode allows script execution without verification
4. Test force flag on reset command
5. Test verify flag on enable command

## Documentation Updates

Remove documentation for non-existent commands until implementation is complete, OR implement them according to this plan.

## Estimated Effort

- Implementation: 2-3 hours
- Testing: 1-2 hours  
- Documentation sync: 30 minutes

## Alternative: Remove Documentation

If these commands are deemed unnecessary:
1. Remove them from docs/trust/README.md
2. Update error messages to not reference them
3. Simplify trust to just init/update/verify/list

This is simpler but provides less flexibility for users.