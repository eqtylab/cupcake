use super::CommandHandler;
use crate::config::loader::PolicyLoader;
use crate::Result;
use std::path::Path;

/// Handler for the `validate` command
pub struct ValidateCommand {
    pub policy_file: String,
    pub strict: bool,
    pub format: String,
}

impl CommandHandler for ValidateCommand {
    fn execute(&self) -> Result<()> {
        let mut loader = PolicyLoader::new();

        // Enable strict validation if requested
        if self.strict {
            loader = loader.with_strict_validation();
        }

        // Determine the starting directory for policy discovery
        let start_dir = if self.policy_file.is_empty() {
            // Auto-discovery mode - start from current directory
            std::env::current_dir().map_err(|e| {
                crate::CupcakeError::Config(format!("Failed to get current directory: {}", e))
            })?
        } else {
            // Specific path provided
            Path::new(&self.policy_file).to_path_buf()
        };

        // Load and validate policies using the YAML composition engine
        match loader.load_and_compose_policies(&start_dir) {
            Ok(policies) => {
                let policy_count = policies.len();

                if self.format == "json" {
                    // JSON output format
                    let result = serde_json::json!({
                        "valid": true,
                        "policy_count": policy_count,
                        "policies": policies.iter().map(|p| {
                            serde_json::json!({
                                "name": p.name,
                                "hook_event": p.hook_event.to_string(),
                                "matcher": p.matcher,
                                "description": p.description
                            })
                        }).collect::<Vec<_>>()
                    });
                    println!("{}", serde_json::to_string_pretty(&result)?);
                } else {
                    // Text output format
                    println!("✅ Policy validation successful!");
                    println!("📄 Found {} composed policies", policy_count);

                    if policy_count > 0 {
                        println!("\nPolicy summary:");
                        for (i, policy) in policies.iter().enumerate() {
                            println!(
                                "  {}. {} ({}:{})",
                                i + 1,
                                policy.name,
                                policy.hook_event,
                                policy.matcher
                            );
                        }
                    }

                    if self.strict {
                        println!("\n🔍 Strict validation mode: PASSED");
                    }
                }

                Ok(())
            }
            Err(e) => {
                if self.format == "json" {
                    // JSON error output
                    let result = serde_json::json!({
                        "valid": false,
                        "error": e.to_string()
                    });
                    println!("{}", serde_json::to_string_pretty(&result)?);
                } else {
                    // Text error output
                    println!("❌ Policy validation failed!");
                    println!("Error: {}", e);
                }

                // Return the original error for proper exit codes
                Err(e)
            }
        }
    }

    fn name(&self) -> &'static str {
        "validate"
    }
}

impl ValidateCommand {
    /// Create new validate command
    pub fn new(policy_file: String, strict: bool, format: String) -> Self {
        Self {
            policy_file,
            strict,
            format,
        }
    }
}
