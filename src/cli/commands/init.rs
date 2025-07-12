use super::CommandHandler;
use crate::Result;
use std::fs;
use std::io;
use std::path::Path;

/// Handler for the `init` command
pub struct InitCommand {
    pub output: String,
    pub yes: bool,
    pub verbose: bool,
}

impl CommandHandler for InitCommand {
    fn execute(&self) -> Result<()> {
        let guardrails_dir = Path::new(&self.output);
        let policies_dir = guardrails_dir.join("policies");
        let cupcake_yaml = guardrails_dir.join("cupcake.yaml");
        let base_policy = policies_dir.join("00-base.yaml");

        if self.verbose {
            println!("🚀 Initializing Cupcake guardrails structure");
            println!("📁 Output directory: {}", guardrails_dir.display());
        }

        // Check if guardrails directory already exists
        if guardrails_dir.exists() && !self.yes {
            println!("⚠️  Directory '{}' already exists.", guardrails_dir.display());
            println!("This will overwrite existing files. Continue? (y/N)");
            
            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            
            if !matches!(input.trim().to_lowercase().as_str(), "y" | "yes") {
                println!("❌ Initialization cancelled.");
                return Ok(());
            }
        }

        // Create directory structure
        if self.verbose {
            println!("📁 Creating directory structure...");
        }
        
        fs::create_dir_all(&policies_dir).map_err(|e| {
            crate::CupcakeError::Config(format!(
                "Failed to create policies directory {}: {}", 
                policies_dir.display(), 
                e
            ))
        })?;

        // Create root cupcake.yaml
        if self.verbose {
            println!("📄 Writing root configuration...");
        }
        
        let root_config_content = r#"# Cupcake YAML Configuration
# This file configures global settings and imports policy fragments

settings:
  # Enable structured audit logging for policy decisions
  audit_logging: true
  
  # Disable debug mode for production use
  debug_mode: false

# Import patterns for policy fragment files
# Files are processed in alphabetical order for deterministic behavior
imports:
  - "policies/*.yaml"
"#;

        fs::write(&cupcake_yaml, root_config_content).map_err(|e| {
            crate::CupcakeError::Config(format!(
                "Failed to write root config {}: {}", 
                cupcake_yaml.display(), 
                e
            ))
        })?;

        // Create base policy file with examples
        if self.verbose {
            println!("📄 Writing example policies...");
        }
        
        let base_policy_content = r#"# Base Policy Examples
# This file demonstrates the YAML policy format structure

PreToolUse:
  "Bash":
    - name: "Git Commit Reminder"
      description: "Reminds to run tests before committing"
      conditions:
        - type: "pattern"
          field: "tool_input.command"
          regex: "git\\s+commit"
      action:
        type: "provide_feedback"
        message: "💡 Remember to run tests before committing!"
        include_context: false

    - name: "Dangerous Command Warning"
      description: "Warns about potentially destructive commands"
      conditions:
        - type: "pattern"
          field: "tool_input.command"
          regex: "^(rm|dd|format)\\s.*(-rf|--force)"
      action:
        type: "provide_feedback"
        message: "⚠️  Potentially destructive command detected. Please review carefully."
        include_context: true

  "Edit|Write":
    - name: "Rust File Formatting Reminder"
      description: "Suggests running cargo fmt on Rust files"
      conditions:
        - type: "pattern"
          field: "tool_input.file_path"
          regex: "\\.rs$"
      action:
        type: "provide_feedback"
        message: "📝 Consider running 'cargo fmt' after editing Rust files"
        include_context: false

PostToolUse:
  "Write":
    - name: "File Creation Confirmation"
      description: "Confirms successful file creation"
      conditions:
        - type: "match"
          field: "tool_name"
          value: "Write"
      action:
        type: "provide_feedback"
        message: "✅ File successfully created"
        include_context: false
"#;

        fs::write(&base_policy, base_policy_content).map_err(|e| {
            crate::CupcakeError::Config(format!(
                "Failed to write base policy {}: {}", 
                base_policy.display(), 
                e
            ))
        })?;

        // Success message
        println!("✅ Cupcake guardrails initialized successfully!");
        println!("📁 Created structure:");
        println!("  {} (root configuration)", cupcake_yaml.display());
        println!("  {} (example policies)", base_policy.display());
        println!();
        println!("🎯 Next steps:");
        println!("  1. Customize policies in {}", policies_dir.display());
        println!("  2. Run 'cupcake validate' to check your configuration");
        println!("  3. Run 'cupcake sync' to integrate with Claude Code");
        println!();
        println!("📚 Learn more about the YAML format in the documentation.");

        Ok(())
    }

    fn name(&self) -> &'static str {
        "init"
    }
}

impl InitCommand {
    /// Create new init command
    pub fn new(output: String, yes: bool, verbose: bool) -> Self {
        Self {
            output,
            yes,
            verbose,
        }
    }
}
