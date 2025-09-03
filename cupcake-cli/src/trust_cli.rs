//! Trust system CLI commands
//! 
//! Provides the user interface for managing script trust: init, update, verify, list

use cupcake_core::trust::{TrustManifest, manifest::{ScriptEntry, ScriptReference}};
use cupcake_core::engine::guidebook::Guidebook;  // Use ENGINE's parser, not trust's!
use anyhow::{Context, Result};
use clap::Parser;
use std::path::{Path, PathBuf};

#[derive(Parser, Debug)]
pub enum TrustCommand {
    /// Initialize trust for this project
    Init {
        /// Project directory (default: current directory)
        #[clap(long, default_value = ".")]
        project_dir: PathBuf,
        
        /// Skip scanning for existing scripts
        #[clap(long)]
        empty: bool,
    },
    
    /// Update trust manifest with current scripts
    Update {
        /// Project directory (default: current directory) 
        #[clap(long, default_value = ".")]
        project_dir: PathBuf,
        
        /// Show diff but don't update
        #[clap(long)]
        dry_run: bool,
        
        /// Auto-approve all changes
        #[clap(long)]
        yes: bool,
    },
    
    /// Verify current scripts against trust manifest
    Verify {
        /// Project directory (default: current directory)
        #[clap(long, default_value = ".")]
        project_dir: PathBuf,
        
        /// Show detailed output
        #[clap(long)]
        verbose: bool,
    },
    
    /// List trusted scripts and their status
    List {
        /// Project directory (default: current directory)
        #[clap(long, default_value = ".")]
        project_dir: PathBuf,
        
        /// Show only modified scripts
        #[clap(long)]
        modified: bool,
        
        /// Show script hashes
        #[clap(long)]
        hashes: bool,
    },
    
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

impl TrustCommand {
    /// Execute the trust command
    pub async fn execute(&self) -> Result<()> {
        match self {
            TrustCommand::Init { project_dir, empty } => {
                trust_init(project_dir, *empty).await
            }
            TrustCommand::Update { project_dir, dry_run, yes } => {
                trust_update(project_dir, *dry_run, *yes).await
            }
            TrustCommand::Verify { project_dir, verbose } => {
                trust_verify(project_dir, *verbose).await
            }
            TrustCommand::List { project_dir, modified, hashes } => {
                trust_list(project_dir, *modified, *hashes).await
            }
            TrustCommand::Disable { project_dir } => {
                trust_disable(project_dir).await
            }
            TrustCommand::Enable { project_dir, verify } => {
                trust_enable(project_dir, *verify).await
            }
            TrustCommand::Reset { project_dir, force } => {
                trust_reset(project_dir, *force).await
            }
        }
    }
}

/// Initialize trust for a project
async fn trust_init(project_dir: &Path, empty: bool) -> Result<()> {
    let cupcake_dir = project_dir.join(".cupcake");
    let trust_file = cupcake_dir.join(".trust");
    
    // Check if already initialized
    if trust_file.exists() {
        println!("✅ Trust already initialized for this project");
        return Ok(());
    }
    
    // Check if cupcake project exists
    if !cupcake_dir.exists() {
        println!("❌ No .cupcake directory found");
        println!("   Run 'cupcake init' first to initialize a Cupcake project");
        return Ok(());
    }
    
    println!("🔐 Initializing trust for Cupcake project...");
    
    let mut manifest = TrustManifest::new();
    
    if !empty {
        println!("📁 Scanning for scripts (guidebook.yml + auto-discovery)...");
        
        // Use ENGINE's parser with auto-discovery!
        let guidebook_path = cupcake_dir.join("guidebook.yml");
        let signals_dir = cupcake_dir.join("signals");
        let actions_dir = cupcake_dir.join("actions");
        
        let guidebook = Guidebook::load_with_conventions(
            &guidebook_path,
            &signals_dir,
            &actions_dir
        ).await
        .context("Failed to load guidebook with auto-discovery")?;
        
        // Get all scripts from the engine's guidebook
        let mut scripts = Vec::new();
        
        // Add all signals
        for (name, signal) in &guidebook.signals {
            scripts.push(("signals".to_string(), name.clone(), signal.command.clone()));
        }
        
        // Add all actions (including on_any_denial)
        for action in &guidebook.actions.on_any_denial {
            scripts.push(("actions".to_string(), "on_any_denial".to_string(), action.command.clone()));
        }
        
        // Add rule-specific actions
        for (rule_id, actions) in &guidebook.actions.by_rule_id {
            for (idx, action) in actions.iter().enumerate() {
                let name = if actions.len() > 1 {
                    format!("{}_{}", rule_id, idx)
                } else {
                    rule_id.clone()
                };
                scripts.push(("actions".to_string(), name, action.command.clone()));
            }
        }
        
        let working_dir = project_dir.to_path_buf();
        
        let mut script_count = 0;
        
        for (category, name, command) in scripts {
            // Create script entry from command
            match ScriptEntry::from_command(&command, &working_dir).await {
                Ok(entry) => {
                    manifest.add_script(&category, &name, entry);
                    script_count += 1;
                }
                Err(e) => {
                    eprintln!("Warning: Failed to process {} script '{}': {}", category, name, e);
                    // Continue processing other scripts
                }
            }
        }
        
        if script_count > 0 {
            println!("📜 Found {} scripts to trust", script_count);
        } else {
            println!("📜 No scripts found in guidebook.yml");
        }
    }
    
    // Save the manifest
    manifest.save(&trust_file)
        .context("Failed to save trust manifest")?;
    
    println!("✅ Trust initialized successfully");
    println!("   Trust manifest saved to: {}", trust_file.display());
    println!("   Use 'cupcake trust update' to add more scripts");
    
    Ok(())
}

/// Update trust manifest with current scripts
async fn trust_update(project_dir: &Path, dry_run: bool, auto_yes: bool) -> Result<()> {
    let cupcake_dir = project_dir.join(".cupcake");
    let trust_file = cupcake_dir.join(".trust");
    
    if !trust_file.exists() {
        println!("❌ Trust not initialized for this project");
        println!("   Run 'cupcake trust init' first");
        return Ok(());
    }
    
    println!("🔄 Checking for script changes...");
    
    // Load existing manifest
    let manifest = TrustManifest::load(&trust_file)
        .context("Failed to load existing trust manifest")?;
    
    // Use ENGINE's parser with auto-discovery!
    let guidebook_path = cupcake_dir.join("guidebook.yml");
    let signals_dir = cupcake_dir.join("signals");
    let actions_dir = cupcake_dir.join("actions");
    
    let guidebook = Guidebook::load_with_conventions(
        &guidebook_path,
        &signals_dir,
        &actions_dir
    ).await
    .context("Failed to load guidebook with auto-discovery")?;
    
    // Get all scripts from the engine's guidebook
    let mut scripts = Vec::new();
    
    // Add all signals
    for (name, signal) in &guidebook.signals {
        scripts.push(("signals".to_string(), name.clone(), signal.command.clone()));
    }
    
    // Add all actions (including on_any_denial)
    for action in &guidebook.actions.on_any_denial {
        scripts.push(("actions".to_string(), "on_any_denial".to_string(), action.command.clone()));
    }
    
    // Add rule-specific actions
    for (rule_id, actions) in &guidebook.actions.by_rule_id {
        for (idx, action) in actions.iter().enumerate() {
            let name = if actions.len() > 1 {
                format!("{}_{}", rule_id, idx)
            } else {
                rule_id.clone()
            };
            scripts.push(("actions".to_string(), name, action.command.clone()));
        }
    }
    
    let working_dir = project_dir.to_path_buf();
    
    // Build current script state
    let mut current_scripts = std::collections::BTreeMap::new();
    for (category, name, command) in scripts {
        match ScriptEntry::from_command(&command, &working_dir).await {
            Ok(entry) => {
                current_scripts
                    .entry(category)
                    .or_insert_with(std::collections::BTreeMap::new)
                    .insert(name, entry);
            }
            Err(e) => {
                eprintln!("Warning: Failed to process script '{}': {}", name, e);
            }
        }
    }
    
    // Compare and detect changes
    let mut changes_detected = false;
    let mut added = Vec::new();
    let mut modified = Vec::new();
    let mut removed = Vec::new();
    
    // Check for added and modified scripts
    for (category, current_category_scripts) in &current_scripts {
        for (name, current_entry) in current_category_scripts {
            match manifest.get_script(category, name) {
                Some(existing_entry) => {
                    if existing_entry.hash() != current_entry.hash() {
                        modified.push((category.clone(), name.clone(), existing_entry.hash().to_string(), current_entry.hash().to_string()));
                        changes_detected = true;
                    }
                }
                None => {
                    added.push((category.clone(), name.clone(), current_entry.command.clone()));
                    changes_detected = true;
                }
            }
        }
    }
    
    // Check for removed scripts  
    for (category, manifest_category_scripts) in manifest.scripts() {
        for (name, _) in manifest_category_scripts {
            if !current_scripts.get(category).map_or(false, |c| c.contains_key(name)) {
                removed.push((category.clone(), name.clone()));
                changes_detected = true;
            }
        }
    }
    
    if !changes_detected {
        println!("✅ No changes detected - all scripts match trust manifest");
        return Ok(());
    }
    
    // Display changes
    println!("\nDetected changes:");
    
    for (category, name, _command) in &added {
        println!("  + {}/{} (added)", category, name);
    }
    
    for (category, name, old_hash, new_hash) in &modified {
        println!("  ~ {}/{} (modified)", category, name);
        println!("    Old hash: {}", &old_hash[0..16]);
        println!("    New hash: {}", &new_hash[0..16]);
    }
    
    for (category, name) in &removed {
        println!("  - {}/{} (removed)", category, name);
    }
    
    if dry_run {
        println!("\n🔍 Dry run complete - no changes made");
        return Ok(());
    }
    
    // Prompt for confirmation if not auto-approved
    if !auto_yes {
        print!("\nUpdate trust manifest? [y/N]: ");
        std::io::Write::flush(&mut std::io::stdout())?;
        
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        let input = input.trim().to_lowercase();
        
        if input != "y" && input != "yes" {
            println!("❌ Trust update cancelled");
            return Ok(());
        }
    }
    
    // Apply changes - rebuild manifest from current state
    let mut updated_manifest = TrustManifest::new();
    for (category, category_scripts) in current_scripts {
        for (name, entry) in category_scripts {
            updated_manifest.add_script(&category, &name, entry);
        }
    }
    
    // Save updated manifest
    updated_manifest.save(&trust_file)
        .context("Failed to save updated trust manifest")?;
    
    println!("✅ Trust manifest updated successfully");
    
    Ok(())
}

/// Verify current scripts against trust manifest
async fn trust_verify(project_dir: &Path, verbose: bool) -> Result<()> {
    let trust_file = project_dir.join(".cupcake/.trust");
    
    if !trust_file.exists() {
        println!("❌ No trust manifest found");
        println!("   Run 'cupcake trust init' to initialize trust");
        return Ok(());
    }
    
    // Load manifest
    println!("🔍 Loading trust manifest...");
    let manifest = TrustManifest::load(&trust_file)
        .context("Failed to load trust manifest")?;
    
    let total_scripts: usize = manifest.scripts().values().map(|s| s.len()).sum();
    println!("📋 Checking {} scripts in manifest...\n", total_scripts);
    
    let mut passed = 0;
    let mut failed = 0;
    let mut missing = 0;
    let mut failed_scripts = Vec::new();
    let mut missing_scripts = Vec::new();
    
    // Verify each script in the manifest
    for (category, scripts) in manifest.scripts() {
        if verbose {
            println!("📁 Verifying {} scripts:", category);
        }
        
        for (name, entry) in scripts {
            // Parse the command to get script reference
            let script_ref = ScriptReference::parse(&entry.command, project_dir);
            
            // Compute current hash
            match script_ref.compute_hash_sync() {
                Ok(current_hash) => {
                    if current_hash == entry.hash {
                        passed += 1;
                        if verbose {
                            println!("   ✅ {} - unmodified", name);
                        }
                    } else {
                        failed += 1;
                        failed_scripts.push((category.clone(), name.clone(), entry.command.clone()));
                        if verbose {
                            println!("   ❌ {} - MODIFIED", name);
                            println!("      Expected: {}", &entry.hash[..16]);
                            println!("      Actual:   {}", &current_hash[..16]);
                        }
                    }
                }
                Err(_) => {
                    // Script file doesn't exist or can't be read
                    missing += 1;
                    missing_scripts.push((category.clone(), name.clone(), entry.command.clone()));
                    if verbose {
                        println!("   ⚠️  {} - NOT FOUND", name);
                    }
                }
            }
        }
    }
    
    // Print summary
    println!("\n📊 Verification Summary:");
    println!("   ✅ Passed:   {} scripts", passed);
    if failed > 0 {
        println!("   ❌ Modified: {} scripts", failed);
    }
    if missing > 0 {
        println!("   ⚠️  Missing:  {} scripts", missing);
    }
    
    // Show details of failures
    if !failed_scripts.is_empty() {
        println!("\n❌ Modified scripts:");
        for (category, name, command) in &failed_scripts {
            println!("   - {}/{}: {}", category, name, 
                if command.len() > 50 { format!("{}...", &command[..50]) } else { command.clone() });
        }
    }
    
    if !missing_scripts.is_empty() {
        println!("\n⚠️  Missing scripts:");
        for (category, name, command) in &missing_scripts {
            println!("   - {}/{}: {}", category, name,
                if command.len() > 50 { format!("{}...", &command[..50]) } else { command.clone() });
        }
    }
    
    // Final status
    if failed > 0 || missing > 0 {
        println!("\n❌ Verification FAILED");
        println!("   Run 'cupcake trust update' to approve changes");
        std::process::exit(1);
    } else {
        println!("\n✅ All scripts verified successfully");
    }
    
    Ok(())
}

/// List trusted scripts and their status
async fn trust_list(project_dir: &Path, show_modified: bool, show_hashes: bool) -> Result<()> {
    let trust_file = project_dir.join(".cupcake/.trust");
    
    if !trust_file.exists() {
        println!("❌ No trust manifest found");
        println!("   Run 'cupcake trust init' to initialize trust");
        return Ok(());
    }
    
    let manifest = TrustManifest::load(&trust_file)
        .context("Failed to load trust manifest")?;
    
    println!("📜 Trusted Scripts:");
    println!("   Manifest: {}", trust_file.display());
    println!("   Created: {}", manifest.created_at().format("%Y-%m-%d %H:%M:%S UTC"));
    
    if manifest.scripts().is_empty() {
        println!("   (no scripts in manifest)");
        return Ok(());
    }
    
    let mut modified_count = 0;
    let mut missing_count = 0;
    
    for (category, scripts) in manifest.scripts() {
        println!("\n📁 {}:", category);
        for (name, entry) in scripts {
            let status = if show_modified {
                // Actually check if script is modified
                let script_ref = ScriptReference::parse(&entry.command, project_dir);
                match script_ref.compute_hash_sync() {
                    Ok(current_hash) => {
                        if current_hash == entry.hash {
                            "✅"
                        } else {
                            modified_count += 1;
                            "❌"
                        }
                    }
                    Err(_) => {
                        // Script file doesn't exist
                        missing_count += 1;
                        "⚠️ "
                    }
                }
            } else {
                // Don't check, just show as present
                "  "
            };
            
            if show_hashes {
                let hash_display = if entry.hash().len() > 16 {
                    format!("{}...", &entry.hash()[..16])
                } else {
                    entry.hash().to_string()
                };
                println!("   {} {} [{}]", status, name, hash_display);
            } else {
                println!("   {} {}", status, name);
            }
        }
    }
    
    // Show summary if checking modifications
    if show_modified && (modified_count > 0 || missing_count > 0) {
        println!("\n📊 Status Summary:");
        if modified_count > 0 {
            println!("   ❌ {} modified scripts", modified_count);
        }
        if missing_count > 0 {
            println!("   ⚠️  {} missing scripts", missing_count);
        }
        println!("   Run 'cupcake trust update' to approve changes");
    }
    
    Ok(())
}

/// Temporarily disable trust verification
async fn trust_disable(project_dir: &Path) -> Result<()> {
    let trust_file = project_dir.join(".cupcake/.trust");
    
    if !trust_file.exists() {
        println!("ℹ️  Trust is not initialized");
        return Ok(());
    }
    
    let mut manifest = TrustManifest::load(&trust_file)
        .context("Failed to load trust manifest")?;
    
    if !manifest.is_enabled() {
        println!("ℹ️  Trust is already disabled");
        return Ok(());
    }
    
    manifest.set_mode(cupcake_core::trust::TrustMode::Disabled)?;
    manifest.save(&trust_file)
        .context("Failed to save trust manifest")?;
    
    println!("⚠️  Trust verification DISABLED");
    println!("   Scripts will execute without integrity checks");
    println!("   Run 'cupcake trust enable' to re-enable");
    
    Ok(())
}

/// Re-enable trust verification
async fn trust_enable(project_dir: &Path, verify: bool) -> Result<()> {
    let trust_file = project_dir.join(".cupcake/.trust");
    
    if !trust_file.exists() {
        println!("❌ No trust manifest found");
        println!("   Run 'cupcake trust init' first");
        return Ok(());
    }
    
    let mut manifest = TrustManifest::load(&trust_file)
        .context("Failed to load trust manifest")?;
    
    if manifest.is_enabled() {
        println!("ℹ️  Trust is already enabled");
        return Ok(());
    }
    
    if verify {
        println!("🔍 Verifying all scripts before enabling...");
        
        let mut all_valid = true;
        let mut modified_scripts = Vec::new();
        let mut missing_scripts = Vec::new();
        
        // Check each script in the manifest
        for (category, scripts) in manifest.scripts() {
            for (name, entry) in scripts {
                let script_ref = ScriptReference::parse(&entry.command, project_dir);
                match script_ref.compute_hash_sync() {
                    Ok(current_hash) => {
                        if current_hash != entry.hash {
                            modified_scripts.push(format!("{}/{}", category, name));
                            all_valid = false;
                        }
                    }
                    Err(_) => {
                        missing_scripts.push(format!("{}/{}", category, name));
                        all_valid = false;
                    }
                }
            }
        }
        
        // Report results
        if !all_valid {
            println!("\n❌ Cannot enable trust - scripts have been modified:");
            
            if !modified_scripts.is_empty() {
                println!("\n   Modified scripts:");
                for script in &modified_scripts {
                    println!("   - {}", script);
                }
            }
            
            if !missing_scripts.is_empty() {
                println!("\n   Missing scripts:");
                for script in &missing_scripts {
                    println!("   - {}", script);
                }
            }
            
            println!("\n   Resolution options:");
            println!("   1. Run 'cupcake trust update' to approve the changes");
            println!("   2. Restore the original scripts");
            println!("   3. Use 'cupcake trust enable' without --verify to enable anyway");
            
            return Ok(());
        }
        
        println!("✅ All scripts verified successfully");
    }
    
    manifest.set_mode(cupcake_core::trust::TrustMode::Enabled)?;
    manifest.save(&trust_file)
        .context("Failed to save trust manifest")?;
    
    println!("✅ Trust verification ENABLED");
    println!("   Scripts will be verified before execution");
    
    Ok(())
}

/// Remove trust manifest and disable trust mode
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
    
    std::fs::remove_file(&trust_file)
        .context("Failed to remove trust manifest")?;
    
    println!("🗑️  Trust manifest removed");
    println!("   Run 'cupcake trust init' to re-initialize");
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[test]
    fn test_command_availability() {
        // Test that all 7 commands are available in the enum
        let commands = vec![
            "init", "update", "verify", "list", 
            "disable", "enable", "reset"
        ];
        
        // This test ensures all documented commands exist
        // If this fails, we're missing a command implementation
        for cmd_name in commands {
            // Parse would fail if command doesn't exist
            let project_dir = std::path::PathBuf::from(".");
            let cmd = match cmd_name {
                "init" => TrustCommand::Init { project_dir, empty: false },
                "update" => TrustCommand::Update { project_dir, dry_run: false, yes: false },
                "verify" => TrustCommand::Verify { project_dir, verbose: false },
                "list" => TrustCommand::List { project_dir, hashes: false, modified: false },
                "disable" => TrustCommand::Disable { project_dir },
                "enable" => TrustCommand::Enable { project_dir, verify: false },
                "reset" => TrustCommand::Reset { project_dir, force: false },
                _ => panic!("Unknown command in test"),
            };
            
            // Verify we can match on it
            match cmd {
                TrustCommand::Init { .. } => {},
                TrustCommand::Update { .. } => {},
                TrustCommand::Verify { .. } => {},
                TrustCommand::List { .. } => {},
                TrustCommand::Disable { .. } => {},
                TrustCommand::Enable { .. } => {},
                TrustCommand::Reset { .. } => {},
            }
        }
    }
    
    #[tokio::test]
    #[cfg(feature = "deterministic-tests")]
    async fn test_trust_mode_toggle() {
        // Test disable/enable cycle
        let temp_dir = TempDir::new().unwrap();
        let trust_file = temp_dir.path().join(".cupcake").join(".trust");
        
        // Create directory structure
        std::fs::create_dir_all(trust_file.parent().unwrap()).unwrap();
        
        // Create a manifest with enabled mode
        let mut manifest = cupcake_core::trust::TrustManifest::new();
        manifest.save(&trust_file).unwrap();
        assert!(manifest.is_enabled());
        
        // Test disable
        trust_disable(temp_dir.path()).await.unwrap();
        let manifest = cupcake_core::trust::TrustManifest::load(&trust_file).unwrap();
        assert!(!manifest.is_enabled());
        
        // Test enable
        trust_enable(temp_dir.path(), false).await.unwrap();
        let manifest = cupcake_core::trust::TrustManifest::load(&trust_file).unwrap();
        assert!(manifest.is_enabled());
    }
    
    #[tokio::test]
    #[cfg(feature = "deterministic-tests")]
    async fn test_reset_removes_manifest() {
        // Test that reset actually removes the manifest file
        let temp_dir = TempDir::new().unwrap();
        let trust_file = temp_dir.path().join(".cupcake").join(".trust");
        
        // Create directory structure
        std::fs::create_dir_all(trust_file.parent().unwrap()).unwrap();
        
        // Create a manifest
        let manifest = cupcake_core::trust::TrustManifest::new();
        manifest.save(&trust_file).unwrap();
        assert!(trust_file.exists());
        
        // Reset with force flag
        trust_reset(temp_dir.path(), true).await.unwrap();
        assert!(!trust_file.exists());
    }
    
    #[test]
    fn test_error_messages_no_missing_commands() {
        // Ensure error messages don't reference non-existent commands
        use cupcake_core::trust::TrustError;
        
        let tampered_err = TrustError::ManifestTampered;
        let err_msg = format!("{}", tampered_err);
        
        // Should now reference the implemented reset command properly
        assert!(err_msg.contains("cupcake trust reset --force"));
        
        // Should provide clear recovery path
        assert!(err_msg.contains("cupcake trust init"));
    }
}


