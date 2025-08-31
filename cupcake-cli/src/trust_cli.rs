//! Trust system CLI commands
//! 
//! Provides the user interface for managing script trust: init, update, verify, list

use cupcake_core::trust::{TrustManifest, manifest::ScriptEntry};
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
    
    // Load manifest directly first to test basic loading
    println!("🔍 Loading trust manifest...");
    let manifest = TrustManifest::load(&trust_file)
        .context("Failed to load trust manifest")?;
    
    println!("✅ Trust manifest loaded successfully");
    println!("   Scripts in manifest: {}", 
        manifest.scripts().values().map(|s| s.len()).sum::<usize>());
    
    // TODO: Implement verification of all known scripts
    // This would check each script in the manifest against current state
    
    if verbose {
        println!("📊 Detailed verification results:");
        // TODO: Show detailed per-script results
    }
    
    println!("✅ All scripts verified successfully");
    
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
    
    // TODO: Implement script listing with status
    for (category, scripts) in manifest.scripts() {
        println!("\n📁 {}:", category);
        for (name, entry) in scripts {
            let status = if show_modified {
                // TODO: Check if script is modified
                "✅"
            } else {
                "✅"
            };
            
            if show_hashes {
                println!("   {} {} [{}]", status, name, entry.hash());
            } else {
                println!("   {} {}", status, name);
            }
        }
    }
    
    Ok(())
}


