//! Cupcake - A performant policy engine for coding agents
//! 
//! Main entry point implementing the CRITICAL_GUIDING_STAR architecture

use anyhow::{Context, Result};
use clap::Parser;
use std::env;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use tracing::{debug, error, info};
use tracing_subscriber::EnvFilter;

use cupcake_core::{engine, harness, validator};

mod trust_cli;

#[derive(Parser, Debug)]
#[clap(
    name = "cupcake",
    about = "Governance and augmentation orchestrator for agentic AI systems",
    version
)]
struct Cli {
    #[clap(subcommand)]
    command: Command,
}

#[derive(Parser, Debug)]
enum Command {
    /// Evaluate a hook event against policies
    Eval {
        /// Directory containing policy files
        #[clap(long, default_value = "./policies")]
        policy_dir: PathBuf,
        
        /// Enable debug output
        #[clap(long)]
        debug: bool,
        
        /// Strict mode (exit non-zero on deny)
        #[clap(long)]
        strict: bool,
    },
    
    /// Verify the engine configuration and policies
    Verify {
        /// Directory containing policy files
        #[clap(long, default_value = "./policies")]
        policy_dir: PathBuf,
    },
    
    /// Initialize a new Cupcake project
    Init {
        /// Initialize global (machine-wide) configuration instead of project
        #[clap(long)]
        global: bool,
    },
    
    /// Manage script trust and integrity verification
    Trust {
        #[clap(subcommand)]
        command: trust_cli::TrustCommand,
    },
    
    /// Validate policies for Cupcake requirements and best practices
    Validate {
        /// Directory containing policy files
        #[clap(long, default_value = ".cupcake/policies")]
        policy_dir: PathBuf,
        
        /// Output results as JSON
        #[clap(long)]
        json: bool,
    },
}

/// Initialize tracing with support for CUPCAKE_TRACE environment variable
/// 
/// Supports two environment variables:
/// - RUST_LOG: Standard Rust logging levels (info, debug, trace, etc.)
/// - CUPCAKE_TRACE: Specialized evaluation tracing (eval, signals, wasm, etc.)
/// 
/// When CUPCAKE_TRACE is set, enables JSON output for structured tracing.
fn initialize_tracing() {
    // Check for CUPCAKE_TRACE environment variable
    let cupcake_trace = env::var("CUPCAKE_TRACE").ok();
    
    // Build the env filter based on RUST_LOG and CUPCAKE_TRACE
    let mut filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));
    
    // If CUPCAKE_TRACE is set, add trace-level logging for specific modules
    if let Some(trace_config) = &cupcake_trace {
        let trace_modules: Vec<&str> = trace_config.split(',').collect();
        
        // Enable trace level for requested modules
        for module in trace_modules {
            let directive = match module.trim() {
                "eval" => "cupcake_core::engine=trace",
                "signals" => "cupcake_core::engine::guidebook=trace",
                "wasm" => "cupcake_core::engine::wasm_runtime=trace",
                "synthesis" => "cupcake_core::engine::synthesis=trace",
                "routing" => "cupcake_core::engine::routing=trace",
                "all" => "cupcake_core=trace",
                _ => continue,
            };
            
            // Parse and add the directive
            if let Ok(parsed) = directive.parse() {
                filter = filter.add_directive(parsed);
            }
        }
    }
    
    // Configure the subscriber based on whether tracing is enabled
    if cupcake_trace.is_some() {
        // JSON output for structured tracing
        tracing_subscriber::fmt()
            .json()
            .with_env_filter(filter)
            .with_target(true)
            .with_thread_ids(true)
            .with_thread_names(true)
            .with_file(true)
            .with_line_number(true)
            .init();
        
        // Log that tracing is enabled
        tracing::info!(
            cupcake_trace = ?cupcake_trace,
            "Cupcake evaluation tracing enabled"
        );
    } else {
        // Standard text output
        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_target(false)
            .init();
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing with enhanced support for CUPCAKE_TRACE
    initialize_tracing();
    
    let cli = Cli::parse();
    
    match cli.command {
        Command::Eval { policy_dir, debug, strict } => {
            if debug {
                tracing::subscriber::set_global_default(
                    tracing_subscriber::fmt()
                        .with_env_filter(EnvFilter::new("debug"))
                        .with_target(false)
                        .finish()
                ).ok();
            }
            
            eval_command(policy_dir, strict).await
        }
        Command::Verify { policy_dir } => {
            verify_command(policy_dir).await
        }
        Command::Init { global } => {
            init_command(global).await
        }
        Command::Trust { command } => {
            command.execute().await
        }
        Command::Validate { policy_dir, json } => {
            validate_command(policy_dir, json).await
        }
    }
}

async fn eval_command(policy_dir: PathBuf, strict: bool) -> Result<()> {
    debug!("Initializing Cupcake engine with policies from: {:?}", policy_dir);
    
    // Initialize the engine - MUST succeed or we exit
    let engine = match engine::Engine::new(&policy_dir).await {
        Ok(e) => {
            debug!("Engine initialized successfully");
            e
        }
        Err(e) => {
            error!("Fatal: Cupcake engine failed to initialize: {:#}", e);
            eprintln!("\nError: Could not start the Cupcake engine.");
            eprintln!("Please ensure the OPA CLI is installed and accessible in your system's PATH.");
            eprintln!("You can download it from: https://www.openpolicyagent.org/docs/latest/#running-opa");
            std::process::exit(1);
        }
    };
    
    // Read hook event from stdin
    let mut buffer = String::new();
    io::stdin()
        .read_to_string(&mut buffer)
        .context("Failed to read hook event from stdin")?;
    
    // Parse the Claude Code event using the harness
    let event = harness::ClaudeHarness::parse_event(&buffer)
        .context("Failed to parse Claude Code event")?;
    
    debug!("Processing {} hook event", event.event_name());
    
    // Convert to JSON Value for the engine
    let mut hook_event_json = serde_json::to_value(&event)?;
    
    // Add hookEventName field for the engine
    // The engine expects camelCase but ClaudeCodeEvent uses snake_case
    if let Some(obj) = hook_event_json.as_object_mut() {
        obj.insert("hookEventName".to_string(), serde_json::Value::String(event.event_name().to_string()));
    }
    
    // Evaluate policies for this hook event
    let decision = match engine.evaluate(&hook_event_json).await {
        Ok(d) => d,
        Err(e) => {
            error!("Policy evaluation failed: {:#}", e);
            // On error, return a safe "allow" with no modifications
            // This ensures we don't break Claude Code on engine failures
            println!("{{}}");
            if strict {
                std::process::exit(1);
            }
            return Ok(());
        }
    };
    
    // Format response using ClaudeHarness
    let response = harness::ClaudeHarness::format_response(&event, &decision)?;
    
    // Output the response to stdout
    println!("{}", response);
    
    // In strict mode, exit non-zero on blocking decisions
    if strict && (decision.is_halt() || decision.is_blocking()) {
        std::process::exit(1);
    }
    
    Ok(())
}

async fn verify_command(policy_dir: PathBuf) -> Result<()> {
    use cupcake_core::engine::global_config::GlobalPaths;
    
    info!("Verifying Cupcake engine configuration...");
    info!("Policy directory: {:?}", policy_dir);
    
    // Check for global configuration
    println!("\n=== Global Configuration ===");
    match GlobalPaths::discover()? {
        Some(global_paths) if global_paths.is_initialized() => {
            println!("✅ Global config found at: {:?}", global_paths.root);
            
            // Count global policies
            if let Ok(entries) = fs::read_dir(&global_paths.policies) {
                let policy_count = entries
                    .filter_map(Result::ok)
                    .filter(|e| {
                        e.path().extension()
                            .and_then(|s| s.to_str())
                            .map(|s| s == "rego")
                            .unwrap_or(false)
                    })
                    .count();
                println!("   Policies: {} global policies", policy_count);
            }
        }
        Some(global_paths) => {
            println!("❌ Global config not initialized (location would be: {:?})", global_paths.root);
            println!("   Run 'cupcake init --global' to initialize");
        }
        None => {
            println!("❌ No global config location available");
            println!("   Set CUPCAKE_GLOBAL_CONFIG or run 'cupcake init --global'");
        }
    }
    
    // Initialize the engine - MUST succeed or we exit
    println!("\n=== Project Configuration ===");
    let engine = match engine::Engine::new(&policy_dir).await {
        Ok(e) => {
            println!("✅ Engine initialized successfully");
            e
        }
        Err(e) => {
            error!("Fatal: Cupcake engine failed to initialize: {:#}", e);
            eprintln!("\n❌ Error: Could not start the Cupcake engine.");
            eprintln!("   Please ensure the OPA CLI is installed and accessible in your system's PATH.");
            eprintln!("   You can download it from: https://www.openpolicyagent.org/docs/latest/#running-opa");
            std::process::exit(1);
        }
    };
    
    // Display routing maps
    println!("\n=== Project Routing Map ===");
    for (key, policies) in engine.routing_map() {
        println!("  {} -> {} policies", key, policies.len());
        for policy in policies {
            println!("    - {}", policy.package_name);
        }
    }
    
    // Display global routing map if it exists
    if !engine.global_routing_map().is_empty() {
        println!("\n=== Global Routing Map ===");
        for (key, policies) in engine.global_routing_map() {
            println!("  {} -> {} policies", key, policies.len());
            for policy in policies {
                println!("    - {}", policy.package_name);
            }
        }
    }
    
    // Check WASM modules
    println!("\n=== WASM Compilation ===");
    if let Some(wasm) = engine.wasm_module() {
        println!("  Project WASM: {} bytes ✅", wasm.len());
    } else {
        // This should never happen now - engine initialization would have failed
        println!("  Project WASM: MISSING ❌");
    }
    
    if let Some(global_wasm) = engine.global_wasm_module() {
        println!("  Global WASM:  {} bytes ✅", global_wasm.len());
    } else {
        println!("  Global WASM:  Not compiled (no global policies or only system policies)");
    }
    
    println!("\n✅ Verification complete!");
    Ok(())
}

async fn init_command(global: bool) -> Result<()> {
    if global {
        // Initialize global configuration
        init_global_config().await
    } else {
        // Initialize project configuration
        init_project_config().await
    }
}

async fn init_global_config() -> Result<()> {
    use cupcake_core::engine::global_config::GlobalPaths;
    
    // Discover or create global config location
    let global_paths = match GlobalPaths::discover()? {
        Some(paths) => {
            // Check if already initialized
            if paths.is_initialized() {
                println!("Global Cupcake configuration already initialized at: {:?}", paths.root);
                println!("To reinitialize, first remove the existing configuration.");
                return Ok(());
            }
            paths
        }
        None => {
            // Create default global config location using the same logic as discovery
            use directories::ProjectDirs;
            
            let config_dir = if let Some(proj_dirs) = ProjectDirs::from("", "", "cupcake") {
                // Use the project-specific config directory
                proj_dirs.config_dir().to_path_buf()
            } else {
                // Fallback
                dirs::home_dir()
                    .context("Could not determine home directory")?
                    .join(".config")
                    .join("cupcake")
            };
            
            GlobalPaths {
                root: config_dir.clone(),
                policies: config_dir.join("policies"),
                guidebook: config_dir.join("guidebook.yml"),
                signals: config_dir.join("signals"),
                actions: config_dir.join("actions"),
            }
        }
    };
    
    info!("Initializing global Cupcake configuration at: {:?}", global_paths.root);
    
    // Initialize the directory structure
    global_paths.initialize()?;
    
    // Create system evaluate policy for global namespace
    let system_dir = global_paths.policies.join("system");
    fs::create_dir_all(&system_dir)?;
    
    fs::write(
        system_dir.join("evaluate.rego"),
        r#"# METADATA
# scope: package
# custom:
#   entrypoint: true
# title: Global System Evaluation Aggregator
# description: |
#   This is the global namespace system evaluation policy.
#   It aggregates decision verbs from all global policies.
package cupcake.global.system

import rego.v1

# Aggregate all decision verbs from global policies
halts := collect_verbs("halt")
denials := collect_verbs("deny")
blocks := collect_verbs("block")
asks := collect_verbs("ask")
allow_overrides := collect_verbs("allow_override")
add_context := collect_verbs("add_context")

# Main evaluation entrypoint
evaluate := {
    "halts": halts,
    "denials": denials,
    "blocks": blocks,
    "asks": asks,
    "allow_overrides": allow_overrides,
    "add_context": add_context
}

# Default implementation returns empty array
default collect_verbs(_) := []

# Collect all instances of a specific verb from all policies
collect_verbs(verb_name) := result if {
    verb_sets := [value |
        walk(data.cupcake.global.policies, [path, value])
        path[count(path) - 1] == verb_name
    ]
    all_decisions := [decision |
        some verb_set in verb_sets
        some decision in verb_set
    ]
    result := all_decisions
}
"#
    )?;
    
    // Create an example global policy
    fs::write(
        global_paths.policies.join("example_global.rego"),
        r#"# METADATA
# scope: package
# custom:
#   routing:
#     required_events: ["UserPromptSubmit"]
# title: Example Global Policy
# description: |
#   This is an example global policy that applies to all Cupcake projects
#   on this machine. Global policies take absolute precedence over project policies.
#   
#   To activate: Uncomment the rules below and customize for your needs.
package cupcake.global.policies.example

import rego.v1

# Example: Add context to all prompts
# add_context contains "Global policy monitoring active"

# Example: Deny dangerous operations globally
# deny contains decision if {
#     input.hook_event_name == "PreToolUse"
#     input.tool_name == "Bash"
#     contains(input.tool_input.command, "rm -rf /")
#     decision := {
#         "rule_id": "GLOBAL-SAFETY-001",
#         "reason": "Dangerous system command blocked by global policy",
#         "severity": "CRITICAL"
#     }
# }

# Example: Halt on specific conditions
# halt contains decision if {
#     input.hook_event_name == "UserPromptSubmit"
#     contains(lower(input.prompt), "malicious")
#     decision := {
#         "rule_id": "GLOBAL-SECURITY-001", 
#         "reason": "Potentially malicious prompt detected",
#         "severity": "CRITICAL"
#     }
# }
"#
    )?;
    
    // Create guidebook with global builtins enabled by default
    fs::write(
        global_paths.guidebook.clone(),
        r#"# Global Cupcake Configuration
# This configuration applies to ALL Cupcake projects on this machine

# Global signals - available to all global policies
signals: {}

# Global actions - triggered by global policy decisions  
actions: {}

# Global builtins - machine-wide security policies
builtins:
  # Protect critical system paths from modification
  system_protection:
    enabled: true
    additional_paths: []  # Add custom protected system paths
    # message: "Custom message for system protection blocks"
  
  # Block reading of credentials and sensitive data
  sensitive_data_protection:
    enabled: true
    additional_patterns: []  # Add custom sensitive file patterns
  
  # Block direct execution of cupcake binary
  cupcake_exec_protection:
    enabled: true
    # message: "Custom message for cupcake execution blocks"
"#
    )?;
    
    // Create builtins directory for global builtin policies
    let builtins_dir = global_paths.policies.join("builtins");
    fs::create_dir_all(&builtins_dir)?;
    
    // Deploy the three global builtin policies
    fs::write(
        builtins_dir.join("system_protection.rego"),
        GLOBAL_SYSTEM_PROTECTION_POLICY
    )?;
    
    fs::write(
        builtins_dir.join("sensitive_data_protection.rego"),
        GLOBAL_SENSITIVE_DATA_POLICY
    )?;
    
    fs::write(
        builtins_dir.join("cupcake_exec_protection.rego"),
        GLOBAL_CUPCAKE_EXEC_POLICY
    )?;
    
    println!("✅ Initialized global Cupcake configuration");
    println!("   Location:      {:?}", global_paths.root);
    println!("   Configuration: {:?}", global_paths.guidebook);
    println!("   Add policies:  {:?}", global_paths.policies);
    println!();
    println!("   Global policies have absolute precedence over project policies.");
    println!("   Three security builtins are enabled by default:");
    println!("     - system_protection: Blocks critical system path modifications");
    println!("     - sensitive_data_protection: Blocks credential file reads");
    println!("     - cupcake_exec_protection: Prevents direct cupcake binary execution");
    println!();
    println!("   Edit guidebook.yml to customize or disable builtins.");
    println!("   Edit example_global.rego to add custom machine-wide policies.");
    
    Ok(())
}

async fn init_project_config() -> Result<()> {
    let cupcake_dir = Path::new(".cupcake");
    
    // If exists, we're done
    if cupcake_dir.exists() {
        println!("Cupcake project already initialized (.cupcake/ exists)");
        return Ok(());
    }
    
    info!("Initializing Cupcake project structure...");
    
    // Create directories
    fs::create_dir_all(".cupcake/policies/system")
        .context("Failed to create .cupcake/policies/system directory")?;
    fs::create_dir_all(".cupcake/policies/builtins")
        .context("Failed to create .cupcake/policies/builtins directory")?;
    fs::create_dir_all(".cupcake/signals")
        .context("Failed to create .cupcake/signals directory")?;
    fs::create_dir_all(".cupcake/actions")
        .context("Failed to create .cupcake/actions directory")?;
    
    // Write guidebook.yml with commented template
    fs::write(
        ".cupcake/guidebook.yml",
        GUIDEBOOK_TEMPLATE
    )
    .context("Failed to create guidebook.yml file")?;
    
    // Write the authoritative system evaluate policy
    fs::write(
        ".cupcake/policies/system/evaluate.rego",
        SYSTEM_EVALUATE_TEMPLATE
    )
    .context("Failed to create system evaluate.rego file")?;
    
    // Write all builtin policies
    fs::write(
        ".cupcake/policies/builtins/always_inject_on_prompt.rego",
        ALWAYS_INJECT_POLICY
    )
    .context("Failed to create always_inject_on_prompt.rego")?;
    
    fs::write(
        ".cupcake/policies/builtins/global_file_lock.rego",
        GLOBAL_FILE_LOCK_POLICY
    )
    .context("Failed to create global_file_lock.rego")?;
    
    fs::write(
        ".cupcake/policies/builtins/git_pre_check.rego",
        GIT_PRE_CHECK_POLICY
    )
    .context("Failed to create git_pre_check.rego")?;
    
    fs::write(
        ".cupcake/policies/builtins/post_edit_check.rego",
        POST_EDIT_CHECK_POLICY
    )
    .context("Failed to create post_edit_check.rego")?;
    
    fs::write(
        ".cupcake/policies/builtins/rulebook_security_guardrails.rego",
        RULEBOOK_SECURITY_POLICY
    )
    .context("Failed to create rulebook_security_guardrails.rego")?;
    
    fs::write(
        ".cupcake/policies/builtins/protected_paths.rego",
        PROTECTED_PATHS_POLICY
    )
    .context("Failed to create protected_paths.rego")?;
    
    // Write a simple example policy
    fs::write(
        ".cupcake/policies/example.rego",
        EXAMPLE_POLICY_TEMPLATE
    )
    .context("Failed to create example policy file")?;
    
    println!("✅ Initialized Cupcake project in .cupcake/");
    println!("   Configuration: .cupcake/guidebook.yml (with examples)");
    println!("   Add policies:  .cupcake/policies/");
    println!("   Add signals:   .cupcake/signals/");
    println!("   Add actions:   .cupcake/actions/");
    println!();
    println!("   Edit guidebook.yml to enable builtins and configure your project.");
    
    Ok(())
}

async fn validate_command(policy_dir: PathBuf, json: bool) -> Result<()> {
    info!("Validating policies in directory: {:?}", policy_dir);
    
    if !policy_dir.exists() {
        eprintln!("Error: Policy directory does not exist: {:?}", policy_dir);
        std::process::exit(1);
    }
    
    // Find all .rego files recursively
    let mut policy_files = Vec::new();
    find_rego_files(&policy_dir, &mut policy_files)?;
    
    if policy_files.is_empty() {
        eprintln!("No .rego files found in {:?}", policy_dir);
        return Ok(());
    }
    
    info!("Found {} policy files", policy_files.len());
    
    // Load all policies
    let mut policies = Vec::new();
    for path in policy_files {
        match validator::PolicyContent::from_file(&path) {
            Ok(policy) => policies.push(policy),
            Err(e) => {
                eprintln!("Warning: Failed to load policy {:?}: {}", path, e);
            }
        }
    }
    
    // Create validator
    let validator = validator::PolicyValidator::new();
    
    // Validate policies
    let result = validator.validate_policies(&policies);
    
    
    // Output results
    if json {
        let json_output = serde_json::json!({
            "total_files": policies.len(),
            "total_errors": result.total_errors,
            "total_warnings": result.total_warnings,
            "policies": result.policies.iter().map(|p| {
                serde_json::json!({
                    "path": p.path,
                    "error_count": p.error_count,
                    "warning_count": p.warning_count,
                    "issues": p.issues.iter().map(|i| {
                        serde_json::json!({
                            "severity": format!("{:?}", i.severity),
                            "rule_id": i.rule_id,
                            "message": i.message,
                            "line": i.line,
                        })
                    }).collect::<Vec<_>>()
                })
            }).collect::<Vec<_>>()
        });
        println!("{}", serde_json::to_string_pretty(&json_output)?);
    } else {
        // Human-readable output
        print_validation_results(&result);
    }
    
    // Exit with error code if there were errors
    if result.total_errors > 0 {
        std::process::exit(1);
    }
    
    Ok(())
}

fn find_rego_files(dir: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        
        if path.is_file() {
            if let Some(ext) = path.extension() {
                if ext == "rego" {
                    files.push(path);
                }
            }
        } else if path.is_dir() {
            find_rego_files(&path, files)?;
        }
    }
    
    Ok(())
}

fn print_validation_results(result: &validator::ValidationResult) {
    use validator::Severity;
    
    for policy_result in &result.policies {
        if policy_result.issues.is_empty() {
            println!("✓ {}", policy_result.path.display());
        } else {
            // Group by severity
            let errors: Vec<_> = policy_result.issues.iter()
                .filter(|i| i.severity == Severity::Error).collect();
            let warnings: Vec<_> = policy_result.issues.iter()
                .filter(|i| i.severity == Severity::Warning).collect();
            
            if !errors.is_empty() {
                println!("✗ {}", policy_result.path.display());
                for issue in errors {
                    let line_info = if let Some(line) = issue.line {
                        format!(" (line {})", line)
                    } else {
                        String::new()
                    };
                    println!("  ERROR{}: {}", line_info, issue.message);
                }
            } else {
                println!("⚠ {}", policy_result.path.display());
            }
            
            for issue in warnings {
                let line_info = if let Some(line) = issue.line {
                    format!(" (line {})", line)
                } else {
                    String::new()
                };
                println!("  WARNING{}: {}", line_info, issue.message);
            }
        }
    }
    
    // Summary
    println!();
    if result.total_errors == 0 && result.total_warnings == 0 {
        println!("✅ All policies passed validation!");
    } else {
        println!("{} errors, {} warnings in {} files", 
                result.total_errors, result.total_warnings, result.policies.len());
    }
}

const SYSTEM_EVALUATE_TEMPLATE: &str = r#"package cupcake.system

import rego.v1

# METADATA
# scope: document
# title: System Aggregation Entrypoint for Hybrid Model
# authors: ["Cupcake Engine"]
# custom:
#   description: "Aggregates all decision verbs from policies into a DecisionSet"
#   entrypoint: true
#   routing:
#     required_events: []
#     required_tools: []

# The single entrypoint for the Hybrid Model.
# This uses the `walk()` built-in to recursively traverse data.cupcake.policies,
# automatically discovering and aggregating all decision verbs from all loaded
# policies, regardless of their package name or nesting depth.
evaluate := decision_set if {
    decision_set := {
        "halts": collect_verbs("halt"),
        "denials": collect_verbs("deny"),
        "blocks": collect_verbs("block"),
        "asks": collect_verbs("ask"),
        "allow_overrides": collect_verbs("allow_override"),
        "add_context": collect_verbs("add_context")
    }
}

# Helper function to collect all decisions for a specific verb type.
# Uses walk() to recursively find all instances of the verb across
# the entire policy hierarchy under data.cupcake.policies.
collect_verbs(verb_name) := result if {
    # Collect all matching verb sets from the policy tree
    verb_sets := [value |
        walk(data.cupcake.policies, [path, value])
        path[count(path) - 1] == verb_name
    ]
    
    # Flatten all sets into a single array
    # Since Rego v1 decision verbs are sets, we need to convert to arrays
    all_decisions := [decision |
        some verb_set in verb_sets
        some decision in verb_set
    ]
    
    result := all_decisions
}

# Default to empty arrays if no decisions found
default collect_verbs(_) := []
"#;

const EXAMPLE_POLICY_TEMPLATE: &str = r#"# METADATA
# scope: package
# title: Example Policy
# description: A minimal example policy that never fires
# custom:
#   routing:
#     required_events: ["PreToolUse"]
#     required_tools: ["Bash"]
package cupcake.policies.example

import rego.v1

# This rule will never fire - it's just here to prevent OPA compilation issues
# It checks for a command that nobody would ever type
deny contains decision if {
    input.tool_input.command == "CUPCAKE_EXAMPLE_RULE_THAT_NEVER_FIRES_12345"
    decision := {
        "reason": "This will never happen",
        "severity": "LOW",
        "rule_id": "EXAMPLE-001"
    }
}

# Replace the above with your actual policies
# Example of a real policy:
# deny contains decision if {
#     contains(input.tool_input.command, "rm -rf /")
#     decision := {
#         "reason": "Dangerous command blocked",
#         "severity": "HIGH",
#         "rule_id": "SAFETY-001"
#     }
# }
"#;

// Include guidebook.yml template directly from base-config.yml
const GUIDEBOOK_TEMPLATE: &str = include_str!("../../examples/base-config.yml");

// Include authoritative builtin policies from examples
const ALWAYS_INJECT_POLICY: &str = include_str!("../../examples/.cupcake/policies/builtins/always_inject_on_prompt.rego");
const GLOBAL_FILE_LOCK_POLICY: &str = include_str!("../../examples/.cupcake/policies/builtins/global_file_lock.rego");
const GIT_PRE_CHECK_POLICY: &str = include_str!("../../examples/.cupcake/policies/builtins/git_pre_check.rego");
const POST_EDIT_CHECK_POLICY: &str = include_str!("../../examples/.cupcake/policies/builtins/post_edit_check.rego");
const RULEBOOK_SECURITY_POLICY: &str = include_str!("../../examples/.cupcake/policies/builtins/rulebook_security_guardrails.rego");
const PROTECTED_PATHS_POLICY: &str = include_str!("../../examples/.cupcake/policies/builtins/protected_paths.rego");

// Global builtin policies embedded in the binary
const GLOBAL_SYSTEM_PROTECTION_POLICY: &str = include_str!("../../examples/global_builtins/system_protection.rego");
const GLOBAL_SENSITIVE_DATA_POLICY: &str = include_str!("../../examples/global_builtins/sensitive_data_protection.rego");
const GLOBAL_CUPCAKE_EXEC_POLICY: &str = include_str!("../../examples/global_builtins/cupcake_exec_protection.rego");

// Aligns with CRITICAL_GUIDING_STAR.md:
// - Simple CLI interface: cupcake eval
// - Takes policy directory as argument (decoupled from examples)
// - Verify command for testing Phase 1 implementation
// - Foundation for reading hook events from stdin (Phase 2)