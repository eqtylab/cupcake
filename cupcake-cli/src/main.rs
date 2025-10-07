//! Cupcake - A performant policy engine for coding agents
//!
//! Main entry point implementing the CRITICAL_GUIDING_STAR architecture

use anyhow::{anyhow, Context, Result};
use clap::{Parser, ValueEnum};
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use tabled::{
    settings::{object::Rows, Alignment, Modify, Style},
    Table, Tabled,
};
use tracing::{debug, error, info};
use tracing_subscriber::EnvFilter;

use cupcake_core::{debug::DebugCapture, engine, harness, validator};

mod harness_config;
mod trust_cli;

/// Trace modules for evaluation tracing
#[derive(Debug, Clone, ValueEnum)]
enum TraceModule {
    Eval,
    Signals,
    Wasm,
    Synthesis,
    Routing,
    All,
}

/// Log levels
#[derive(Debug, Clone, ValueEnum)]
enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl LogLevel {
    fn to_filter_directive(&self) -> &'static str {
        match self {
            LogLevel::Error => "error",
            LogLevel::Warn => "warn",
            LogLevel::Info => "info",
            LogLevel::Debug => "debug",
            LogLevel::Trace => "trace",
        }
    }
}

/// Memory size with validation
#[derive(Debug, Clone)]
struct MemorySize {
    bytes: usize,
}

impl FromStr for MemorySize {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        const MIN_MEMORY: usize = 1024 * 1024; // 1MB
        const MAX_MEMORY: usize = 100 * 1024 * 1024; // 100MB

        let parsed = if s.ends_with("MB") || s.ends_with("mb") {
            s.trim_end_matches("MB")
                .trim_end_matches("mb")
                .parse::<usize>()
                .map(|n| n * 1024 * 1024)
        } else if s.ends_with("KB") || s.ends_with("kb") {
            s.trim_end_matches("KB")
                .trim_end_matches("kb")
                .parse::<usize>()
                .map(|n| n * 1024)
        } else {
            s.parse::<usize>()
        };

        let bytes = parsed.map_err(|_| format!("Invalid memory size: {}", s))?;

        if bytes < MIN_MEMORY {
            return Err(format!(
                "Memory size too small: {}. Minimum is 1MB (1048576 bytes)",
                s
            ));
        }
        if bytes > MAX_MEMORY {
            return Err(format!(
                "Memory size too large: {}. Maximum is 100MB (104857600 bytes)",
                s
            ));
        }

        Ok(MemorySize { bytes })
    }
}

impl Default for MemorySize {
    fn default() -> Self {
        MemorySize {
            bytes: 10 * 1024 * 1024, // 10MB default
        }
    }
}

#[derive(Parser, Debug)]
#[clap(
    name = "cupcake",
    about = "Governance and augmentation orchestrator for agentic AI systems",
    version
)]
struct Cli {
    #[clap(subcommand)]
    command: Command,

    /// Enable evaluation tracing (comma-separated: eval,signals,wasm,synthesis,routing,all)
    #[clap(long, value_delimiter = ',', global = true)]
    trace: Vec<TraceModule>,

    /// Set log level
    #[clap(long, default_value = "info", global = true)]
    log_level: LogLevel,

    /// Override global configuration file path
    #[clap(long, global = true)]
    global_config: Option<PathBuf>,

    /// Maximum WASM memory allocation (e.g., "10MB", "50MB")
    #[clap(long, default_value = "10MB", global = true)]
    wasm_max_memory: MemorySize,

    /// Enable debug file output to .cupcake/debug/
    #[clap(long, global = true)]
    debug_files: bool,

    /// Enable routing debug output to .cupcake/debug/routing/
    #[clap(long, global = true)]
    debug_routing: bool,

    /// Override OPA binary path
    #[clap(long, global = true)]
    opa_path: Option<PathBuf>,
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

        /// Configure integration with an agent harness (e.g., 'claude')
        #[clap(long, value_enum)]
        harness: Option<HarnessType>,

        /// Enable specific builtins (comma-separated list)
        #[clap(long, value_delimiter = ',')]
        builtins: Option<Vec<String>>,
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

    /// Inspect policies to show metadata and routing information
    Inspect {
        /// Directory containing policy files
        #[clap(long, default_value = ".cupcake/policies")]
        policy_dir: PathBuf,

        /// Output results as JSON
        #[clap(long, conflicts_with = "table")]
        json: bool,

        /// Display results in a compact table format
        #[clap(short, long)]
        table: bool,
    },
}

/// Supported agent harness types for integration
#[derive(Debug, Clone, ValueEnum)]
enum HarnessType {
    /// Claude Code (claude.ai/code)
    Claude,
    // Future harnesses can be added here
}

/// Initialize tracing with CLI flags
///
/// Configures logging based on --log-level and --trace flags.
/// When --trace is set, enables JSON output for structured tracing.
fn initialize_tracing(log_level: &LogLevel, trace_modules: &[TraceModule]) {
    // Build the env filter based on log level
    let mut filter = EnvFilter::new(log_level.to_filter_directive());

    // If trace modules are specified, add trace-level logging for specific modules
    for module in trace_modules {
        let directive = match module {
            TraceModule::Eval => "cupcake_core::engine=trace",
            TraceModule::Signals => "cupcake_core::engine::guidebook=trace",
            TraceModule::Wasm => "cupcake_core::engine::wasm_runtime=trace",
            TraceModule::Synthesis => "cupcake_core::engine::synthesis=trace",
            TraceModule::Routing => "cupcake_core::engine::routing=trace",
            TraceModule::All => "cupcake_core=trace",
        };

        // Parse and add the directive
        if let Ok(parsed) = directive.parse() {
            filter = filter.add_directive(parsed);
        }
    }

    // Configure the subscriber based on whether tracing is enabled
    if !trace_modules.is_empty() {
        // JSON output for structured tracing - MUST go to stderr
        tracing_subscriber::fmt()
            .json()
            .with_env_filter(filter)
            .with_target(true)
            .with_thread_ids(true)
            .with_thread_names(true)
            .with_file(true)
            .with_line_number(true)
            .with_writer(std::io::stderr) // Critical: logs to stderr, not stdout
            .init();

        // Log that tracing is enabled
        tracing::info!(
            trace_modules = ?trace_modules,
            "Cupcake evaluation tracing enabled"
        );
    } else {
        // Standard text output - MUST go to stderr
        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_target(false)
            .with_writer(std::io::stderr) // Critical: logs to stderr, not stdout
            .init();
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize tracing with CLI flags
    initialize_tracing(&cli.log_level, &cli.trace);

    match cli.command {
        Command::Eval {
            policy_dir,
            debug,
            strict,
        } => {
            if debug {
                tracing::subscriber::set_global_default(
                    tracing_subscriber::fmt()
                        .with_env_filter(EnvFilter::new("debug"))
                        .with_target(false)
                        .with_writer(std::io::stderr) // Critical: logs to stderr, not stdout
                        .finish(),
                )
                .ok();
            }

            eval_command(policy_dir, strict, cli.debug_files).await
        }
        Command::Verify { policy_dir } => verify_command(policy_dir).await,
        Command::Init {
            global,
            harness,
            builtins,
        } => init_command(global, harness, builtins).await,
        Command::Trust { command } => command.execute().await,
        Command::Validate { policy_dir, json } => validate_command(policy_dir, json).await,
        Command::Inspect {
            policy_dir,
            json,
            table,
        } => inspect_command(policy_dir, json, table).await,
    }
}

async fn eval_command(policy_dir: PathBuf, strict: bool, debug_files_enabled: bool) -> Result<()> {
    debug!(
        "Initializing Cupcake engine with policies from: {:?}",
        policy_dir
    );

    // Initialize the engine - MUST succeed or we exit
    let engine = match engine::Engine::new(&policy_dir).await {
        Ok(e) => {
            debug!("Engine initialized successfully");
            e
        }
        Err(e) => {
            error!("Fatal: Cupcake engine failed to initialize: {:#}", e);
            eprintln!("\nError: Could not start the Cupcake engine.");
            eprintln!(
                "Please ensure the OPA CLI is installed and accessible in your system's PATH."
            );
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
        obj.insert(
            "hookEventName".to_string(),
            serde_json::Value::String(event.event_name().to_string()),
        );
    }

    // Create debug capture if enabled via CLI flag
    let mut debug_capture = if debug_files_enabled {
        // Generate a trace ID for this evaluation
        let trace_id = cupcake_core::engine::trace::generate_trace_id();
        Some(DebugCapture::new(hook_event_json.clone(), trace_id))
    } else {
        None
    };

    // Evaluate policies for this hook event
    let decision = match engine
        .evaluate(&hook_event_json, debug_capture.as_mut())
        .await
    {
        Ok(d) => d,
        Err(e) => {
            error!("Policy evaluation failed: {:#}", e);

            // Capture the error in debug if enabled
            if let Some(ref mut debug) = debug_capture {
                debug.add_error(format!("Policy evaluation failed: {e:#}"));

                // Write the debug file even on error to help troubleshooting
                if let Err(write_err) = debug.write_if_enabled() {
                    debug!("Failed to write debug file: {}", write_err);
                }
            }

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

    // Capture the response in debug if enabled
    if let Some(ref mut debug) = debug_capture {
        // The response is already a JSON Value
        debug.response_to_claude = Some(response.clone());

        // Write the debug file
        if let Err(e) = debug.write_if_enabled() {
            // Log but don't fail on debug write errors
            debug!("Failed to write debug file: {}", e);
        }
    }

    // Output the response to stdout as JSON string
    println!("{}", serde_json::to_string(&response)?);

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
                        e.path()
                            .extension()
                            .and_then(|s| s.to_str())
                            .map(|s| s == "rego")
                            .unwrap_or(false)
                    })
                    .count();
                println!("   Policies: {policy_count} global policies");
            }
        }
        Some(global_paths) => {
            println!(
                "❌ Global config not initialized (location would be: {:?})",
                global_paths.root
            );
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
            eprintln!(
                "   Please ensure the OPA CLI is installed and accessible in your system's PATH."
            );
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

/// List of valid project-level builtin names
const VALID_PROJECT_BUILTINS: &[&str] = &[
    "always_inject_on_prompt",
    "global_file_lock",
    "git_pre_check",
    "post_edit_check",
    "rulebook_security_guardrails",
    "protected_paths",
    "git_block_no_verify",
    "enforce_full_file_read",
];

/// List of valid global-level builtin names
const VALID_GLOBAL_BUILTINS: &[&str] = &[
    "system_protection",
    "sensitive_data_protection",
    "cupcake_exec_protection",
];

/// Validate that provided builtin names are valid
fn validate_builtin_names(names: &[String], global: bool) -> Result<()> {
    let valid_list = if global {
        VALID_GLOBAL_BUILTINS
    } else {
        VALID_PROJECT_BUILTINS
    };

    for name in names {
        if !valid_list.contains(&name.as_str()) {
            let valid_names = valid_list.join(", ");
            return Err(anyhow!(
                "Unknown builtin '{}'. Valid {} builtins are: {}",
                name,
                if global { "global" } else { "project" },
                valid_names
            ));
        }
    }
    Ok(())
}

/// Generate guidebook content with specified builtins enabled
fn generate_guidebook_with_builtins(enabled_builtins: &[String]) -> Result<String> {
    // Start with the base template
    let template = GUIDEBOOK_TEMPLATE;

    // If no builtins specified, return the template as-is
    if enabled_builtins.is_empty() {
        return Ok(template.to_string());
    }

    let mut lines: Vec<String> = template.lines().map(|s| s.to_string()).collect();
    let mut in_builtin_section = false;
    let mut current_builtin: Option<String> = None;
    let mut indent_level = 0;

    for (i, line) in lines.clone().iter().enumerate() {
        let line = line.clone();
        let trimmed = line.trim();

        // Check if we're entering the builtins section
        if trimmed == "builtins:" {
            in_builtin_section = true;
            continue;
        }

        if !in_builtin_section {
            continue;
        }

        // Check for builtin names in comments
        for builtin_name in enabled_builtins {
            let inline_pattern = format!("{builtin_name}:");

            // Look for the builtin in a comment
            if trimmed.starts_with('#') && trimmed.contains(&inline_pattern) {
                current_builtin = Some(builtin_name.clone());
                indent_level = line.len() - line.trim_start().len();

                // Uncomment this line
                if let Some(pos) = line.find("# ") {
                    lines[i] = format!("{}{}", &line[..pos], &line[pos + 2..]);
                }
            }

            // If we're processing lines for the current builtin, uncomment them
            if let Some(ref current) = current_builtin {
                if current == builtin_name {
                    // Check if we've moved to a different builtin or ended the section
                    if trimmed.starts_with("# -----")
                        || (trimmed.starts_with('#')
                            && trimmed.ends_with(':')
                            && !trimmed.contains(builtin_name))
                    {
                        current_builtin = None;
                        continue;
                    }

                    // Uncomment lines that belong to this builtin
                    let line_copy = lines[i].clone();
                    if line_copy.trim_start().starts_with('#') {
                        let current_indent = line_copy.len() - line_copy.trim_start().len();
                        if current_indent >= indent_level && current_indent <= indent_level + 4 {
                            // Remove the comment marker
                            if let Some(pos) = line_copy.find("# ") {
                                lines[i] =
                                    format!("{}{}", &line_copy[..pos], &line_copy[pos + 2..]);
                            } else if let Some(pos) = line_copy.find("#") {
                                lines[i] =
                                    format!("{}{}", &line_copy[..pos], &line_copy[pos + 1..]);
                            }
                        }
                    }
                }
            }
        }
    }

    // Now add default configurations for certain builtins that need them
    let result = add_default_builtin_configs(lines.join("\n"), enabled_builtins)?;

    Ok(result)
}

/// Add sensible default configurations for builtins that need them
fn add_default_builtin_configs(mut content: String, enabled_builtins: &[String]) -> Result<String> {
    // For git_pre_check, ensure there's at least one check
    if enabled_builtins.contains(&"git_pre_check".to_string())
        && (!content.contains("checks:") || content.contains("checks: []"))
    {
        // Replace empty checks with a default
        content = content.replace(
                "git_pre_check:\n    enabled: true",
                "git_pre_check:\n    enabled: true\n    checks:\n      - command: \"echo 'Validation passed'\"\n        message: \"Basic validation check\""
            );
    }

    // For protected_paths, add default paths if none exist
    if enabled_builtins.contains(&"protected_paths".to_string())
        && (!content.contains("paths:") || content.contains("paths: []"))
    {
        content = content.replace(
                "protected_paths:\n    enabled: true",
                "protected_paths:\n    enabled: true\n    message: \"System path modification blocked by policy\"\n    paths:\n      - \"/etc/\"\n      - \"/System/\"\n      - \"~/.ssh/\""
            );
    }

    // For post_edit_check, add basic extension checks if none exist
    if enabled_builtins.contains(&"post_edit_check".to_string())
        && (!content.contains("by_extension:") || content.contains("by_extension: {}"))
    {
        content = content.replace(
                "post_edit_check:",
                "post_edit_check:\n    by_extension:\n      \"py\":\n        command: \"python -m py_compile\"\n        message: \"Python syntax validation\"\n      \"rs\":\n        command: \"cargo check --message-format short 2>/dev/null || echo 'Rust check not available'\"\n        message: \"Rust compilation check\""
            );
    }

    Ok(content)
}

/// Generate global guidebook with specified builtins enabled
fn generate_global_guidebook(enabled_builtins: Option<&[String]>) -> String {
    // Default builtins for global config if none specified
    let default_builtins = vec![
        "system_protection".to_string(),
        "sensitive_data_protection".to_string(),
        "cupcake_exec_protection".to_string(),
    ];

    let builtins_to_enable = enabled_builtins
        .map(|b| b.to_vec())
        .unwrap_or(default_builtins);

    let mut guidebook = String::from(
        r#"# Global Cupcake Configuration
# This configuration applies to ALL Cupcake projects on this machine

# Global signals - available to all global policies
signals: {}

# Global actions - triggered by global policy decisions
actions: {}

# Global builtins - machine-wide security policies
builtins:"#,
    );

    // Add each enabled builtin
    for builtin in &builtins_to_enable {
        match builtin.as_str() {
            "system_protection" => {
                guidebook.push_str(
                    r#"
  # Protect critical system paths from modification
  system_protection:
    enabled: true
    additional_paths: []  # Add custom protected system paths
    # message: "Custom message for system protection blocks""#,
                );
            }
            "sensitive_data_protection" => {
                guidebook.push_str(
                    r#"

  # Block reading of credentials and sensitive data
  sensitive_data_protection:
    enabled: true
    additional_patterns: []  # Add custom sensitive file patterns"#,
                );
            }
            "cupcake_exec_protection" => {
                guidebook.push_str(
                    r#"

  # Block direct execution of cupcake binary
  cupcake_exec_protection:
    enabled: true
    # message: "Custom message for cupcake execution blocks""#,
                );
            }
            _ => {
                // This shouldn't happen due to validation, but handle gracefully
                eprintln!("Warning: Unknown global builtin '{builtin}' - skipping");
            }
        }
    }

    guidebook.push('\n');
    guidebook
}

async fn init_command(
    global: bool,
    harness: Option<HarnessType>,
    builtins: Option<Vec<String>>,
) -> Result<()> {
    // Validate builtin names if provided
    if let Some(ref builtin_list) = builtins {
        validate_builtin_names(builtin_list, global)?;
    }

    if global {
        // Initialize global configuration
        init_global_config(harness, builtins).await
    } else {
        // Initialize project configuration
        init_project_config(harness, builtins).await
    }
}

async fn init_global_config(
    harness: Option<HarnessType>,
    builtins: Option<Vec<String>>,
) -> Result<()> {
    use cupcake_core::engine::global_config::GlobalPaths;

    // Discover or create global config location
    let global_paths = match GlobalPaths::discover()? {
        Some(paths) => {
            // Check if already initialized
            if paths.is_initialized() {
                println!(
                    "Global Cupcake configuration already initialized at: {:?}",
                    paths.root
                );
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

    info!(
        "Initializing global Cupcake configuration at: {:?}",
        global_paths.root
    );

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
"#,
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
"#,
    )?;

    // Create guidebook with specified or default global builtins
    let guidebook_content = generate_global_guidebook(builtins.as_deref());
    fs::write(global_paths.guidebook.clone(), guidebook_content)?;

    // Create builtins directory for global builtin policies
    let builtins_dir = global_paths.policies.join("builtins");
    fs::create_dir_all(&builtins_dir)?;

    // Deploy the three global builtin policies
    fs::write(
        builtins_dir.join("system_protection.rego"),
        GLOBAL_SYSTEM_PROTECTION_POLICY,
    )?;

    fs::write(
        builtins_dir.join("sensitive_data_protection.rego"),
        GLOBAL_SENSITIVE_DATA_POLICY,
    )?;

    fs::write(
        builtins_dir.join("cupcake_exec_protection.rego"),
        GLOBAL_CUPCAKE_EXEC_POLICY,
    )?;

    println!("✅ Initialized global Cupcake configuration");
    println!("   Location:      {:?}", global_paths.root);
    println!("   Configuration: {:?}", global_paths.guidebook);
    println!("   Add policies:  {:?}", global_paths.policies);
    println!();
    println!("   Global policies have absolute precedence over project policies.");

    // Show which builtins were enabled
    if let Some(ref builtin_list) = builtins {
        println!("   Enabled builtins:");
        for builtin in builtin_list {
            println!("     - {builtin}");
        }
    } else {
        // Default global builtins were enabled
        println!("   Three security builtins are enabled by default:");
        println!("     - system_protection: Blocks critical system path modifications");
        println!("     - sensitive_data_protection: Blocks credential file reads");
        println!("     - cupcake_exec_protection: Prevents direct cupcake binary execution");
    }

    println!();
    println!("   Edit guidebook.yml to customize or disable builtins.");
    println!("   Edit example_global.rego to add custom machine-wide policies.");

    // Configure harness if specified
    if let Some(harness_type) = harness {
        println!();
        harness_config::configure_harness(harness_type, &global_paths.root, true).await?;
    }

    Ok(())
}

async fn init_project_config(
    harness: Option<HarnessType>,
    builtins: Option<Vec<String>>,
) -> Result<()> {
    let cupcake_dir = Path::new(".cupcake");

    // Check if cupcake directory exists
    let cupcake_exists = cupcake_dir.exists();

    if cupcake_exists {
        println!("Cupcake project already initialized (.cupcake/ exists)");
        // Continue to configure harness if specified
    } else {
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

        // Write guidebook.yml - either with enabled builtins or commented template
        let guidebook_content = if let Some(ref builtin_list) = builtins {
            generate_guidebook_with_builtins(builtin_list)?
        } else {
            GUIDEBOOK_TEMPLATE.to_string()
        };

        fs::write(".cupcake/guidebook.yml", guidebook_content)
            .context("Failed to create guidebook.yml file")?;

        // Write the authoritative system evaluate policy
        fs::write(
            ".cupcake/policies/system/evaluate.rego",
            SYSTEM_EVALUATE_TEMPLATE,
        )
        .context("Failed to create system evaluate.rego file")?;

        // Write all builtin policies
        fs::write(
            ".cupcake/policies/builtins/always_inject_on_prompt.rego",
            ALWAYS_INJECT_POLICY,
        )
        .context("Failed to create always_inject_on_prompt.rego")?;

        fs::write(
            ".cupcake/policies/builtins/global_file_lock.rego",
            GLOBAL_FILE_LOCK_POLICY,
        )
        .context("Failed to create global_file_lock.rego")?;

        fs::write(
            ".cupcake/policies/builtins/git_pre_check.rego",
            GIT_PRE_CHECK_POLICY,
        )
        .context("Failed to create git_pre_check.rego")?;

        fs::write(
            ".cupcake/policies/builtins/post_edit_check.rego",
            POST_EDIT_CHECK_POLICY,
        )
        .context("Failed to create post_edit_check.rego")?;

        fs::write(
            ".cupcake/policies/builtins/rulebook_security_guardrails.rego",
            RULEBOOK_SECURITY_POLICY,
        )
        .context("Failed to create rulebook_security_guardrails.rego")?;

        fs::write(
            ".cupcake/policies/builtins/protected_paths.rego",
            PROTECTED_PATHS_POLICY,
        )
        .context("Failed to create protected_paths.rego")?;

        fs::write(
            ".cupcake/policies/builtins/git_block_no_verify.rego",
            GIT_BLOCK_NO_VERIFY_POLICY,
        )
        .context("Failed to create git_block_no_verify.rego")?;

        fs::write(
            ".cupcake/policies/builtins/enforce_full_file_read.rego",
            ENFORCE_FULL_FILE_READ_POLICY,
        )
        .context("Failed to create enforce_full_file_read.rego")?;

        // Write a simple example policy
        fs::write(".cupcake/policies/example.rego", EXAMPLE_POLICY_TEMPLATE)
            .context("Failed to create example policy file")?;

        println!("✅ Initialized Cupcake project in .cupcake/");
        println!("   Configuration: .cupcake/guidebook.yml (with examples)");
        println!("   Add policies:  .cupcake/policies/");
        println!("   Add signals:   .cupcake/signals/");
        println!("   Add actions:   .cupcake/actions/");
        println!();

        // Show which builtins were enabled
        if let Some(ref builtin_list) = builtins {
            if !builtin_list.is_empty() {
                println!("   Enabled builtins:");
                for builtin in builtin_list {
                    println!("     - {builtin}");
                }
                println!();
            }
        }

        println!("   Edit guidebook.yml to enable builtins and configure your project.");
    }

    // Configure harness if specified (whether cupcake exists or not)
    if let Some(harness_type) = harness {
        if cupcake_exists {
            println!();
        }
        harness_config::configure_harness(harness_type, Path::new(".cupcake"), false).await?;
    }

    Ok(())
}

async fn validate_command(policy_dir: PathBuf, json: bool) -> Result<()> {
    info!("Validating policies in directory: {:?}", policy_dir);

    if !policy_dir.exists() {
        eprintln!("Error: Policy directory does not exist: {policy_dir:?}");
        std::process::exit(1);
    }

    // Find all .rego files recursively
    let mut policy_files = Vec::new();
    find_rego_files(&policy_dir, &mut policy_files)?;

    if policy_files.is_empty() {
        eprintln!("No .rego files found in {policy_dir:?}");
        return Ok(());
    }

    info!("Found {} policy files", policy_files.len());

    // Load all policies
    let mut policies = Vec::new();
    for path in policy_files {
        match validator::PolicyContent::from_file(&path) {
            Ok(policy) => policies.push(policy),
            Err(e) => {
                eprintln!("Warning: Failed to load policy {path:?}: {e}");
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
            let errors: Vec<_> = policy_result
                .issues
                .iter()
                .filter(|i| i.severity == Severity::Error)
                .collect();
            let warnings: Vec<_> = policy_result
                .issues
                .iter()
                .filter(|i| i.severity == Severity::Warning)
                .collect();

            if !errors.is_empty() {
                println!("✗ {}", policy_result.path.display());
                for issue in errors {
                    let line_info = if let Some(line) = issue.line {
                        format!(" (line {line})")
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
                    format!(" (line {line})")
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
        println!(
            "{} errors, {} warnings in {} files",
            result.total_errors,
            result.total_warnings,
            result.policies.len()
        );
    }
}

// Table row structure for policy display
#[derive(Tabled)]
struct PolicyTableRow {
    #[tabled(rename = "Package")]
    package: String,
    #[tabled(rename = "Events")]
    events: String,
    #[tabled(rename = "Tools")]
    tools: String,
    #[tabled(rename = "Title")]
    title: String,
    #[tabled(rename = "Type")]
    policy_type: String,
}

async fn inspect_command(policy_dir: PathBuf, json: bool, table: bool) -> Result<()> {
    info!("Inspecting policies in directory: {:?}", policy_dir);

    if !policy_dir.exists() {
        eprintln!("Error: Policy directory does not exist: {policy_dir:?}");
        std::process::exit(1);
    }

    // Find all .rego files recursively
    let mut policy_files = Vec::new();
    find_rego_files(&policy_dir, &mut policy_files)?;

    if policy_files.is_empty() {
        eprintln!("No .rego files found in {policy_dir:?}");
        return Ok(());
    }

    info!("Found {} policy files", policy_files.len());

    // Collect policy metadata
    let mut policies_metadata = Vec::new();

    for path in &policy_files {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read policy file: {path:?}"))?;

        // Parse metadata using the engine's metadata parser
        let metadata = match engine::metadata::parse_metadata(&content) {
            Ok(Some(meta)) => meta,
            Ok(None) | Err(_) => {
                // No metadata or parse error - create empty metadata
                engine::metadata::PolicyMetadata {
                    scope: None,
                    title: None,
                    authors: vec![],
                    organizations: vec![],
                    custom: engine::metadata::CustomMetadata::default(),
                }
            }
        };

        // Extract package name
        let package_name = content
            .lines()
            .find(|line| line.trim().starts_with("package "))
            .map(|line| line.trim_start_matches("package ").trim().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        // Check if it's a builtin
        let is_builtin = path.components().any(|c| c.as_os_str() == "builtins");

        let routing = metadata.custom.routing.as_ref();

        policies_metadata.push(serde_json::json!({
            "path": path.display().to_string(),
            "package": package_name,
            "is_builtin": is_builtin,
            "routing": {
                "required_events": routing.map(|r| r.required_events.clone()).unwrap_or_default(),
                "required_tools": routing.map(|r| r.required_tools.clone()).unwrap_or_default(),
                "required_signals": routing.map(|r| r.required_signals.clone()).unwrap_or_default(),
            },
            "metadata": {
                "title": metadata.title,
                "authors": metadata.authors,
                "organizations": metadata.organizations,
                "scope": metadata.scope,
            }
        }));
    }

    if json {
        // Output as JSON
        let output = serde_json::json!({
            "total_policies": policies_metadata.len(),
            "policies": policies_metadata,
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else if table {
        // Output as table
        let mut table_rows = Vec::new();

        for policy in &policies_metadata {
            let package = policy
                .get("package")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");

            // Shorten package name if too long
            let package_short = if package.starts_with("cupcake.") {
                package.trim_start_matches("cupcake.").to_string()
            } else {
                package.to_string()
            };

            let is_builtin = policy
                .get("is_builtin")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            let policy_type = if is_builtin { "builtin" } else { "custom" }.to_string();

            // Get routing info
            let events = policy
                .get("routing")
                .and_then(|r| r.get("required_events"))
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|e| e.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .unwrap_or_else(|| "-".to_string());

            let tools = policy
                .get("routing")
                .and_then(|r| r.get("required_tools"))
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|t| t.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .unwrap_or_else(|| "-".to_string());

            // Get title, truncate if too long
            let title = policy
                .get("metadata")
                .and_then(|m| m.get("title"))
                .and_then(|v| v.as_str())
                .unwrap_or("-");

            let title_truncated = if title.len() > 40 {
                format!("{}...", &title[..37])
            } else {
                title.to_string()
            };

            table_rows.push(PolicyTableRow {
                package: package_short,
                events,
                tools,
                title: title_truncated,
                policy_type,
            });
        }

        if !table_rows.is_empty() {
            let table = Table::new(&table_rows)
                .with(Style::rounded())
                .with(Modify::new(Rows::first()).with(Alignment::center()))
                .to_string();

            println!("Found {} policies\n", policies_metadata.len());
            println!("{table}");
        } else {
            println!("No policies found.");
        }
    } else {
        // Output as human-readable format
        println!("Found {} policies\n", policies_metadata.len());

        for policy in &policies_metadata {
            let path = policy
                .get("path")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            let package = policy
                .get("package")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            let is_builtin = policy
                .get("is_builtin")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            println!("Policy: {path}");
            println!("  Package: {package}");
            if is_builtin {
                println!("  Type: Builtin");
            }

            if let Some(routing) = policy.get("routing") {
                if let Some(events) = routing.get("required_events").and_then(|v| v.as_array()) {
                    if !events.is_empty() {
                        let events_str: Vec<String> = events
                            .iter()
                            .filter_map(|e| e.as_str().map(String::from))
                            .collect();
                        println!("  Required Events: {}", events_str.join(", "));
                    }
                }

                if let Some(tools) = routing.get("required_tools").and_then(|v| v.as_array()) {
                    if !tools.is_empty() {
                        let tools_str: Vec<String> = tools
                            .iter()
                            .filter_map(|t| t.as_str().map(String::from))
                            .collect();
                        println!("  Required Tools: {}", tools_str.join(", "));
                    }
                }
            }

            if let Some(metadata) = policy.get("metadata") {
                if let Some(title) = metadata.get("title").and_then(|v| v.as_str()) {
                    println!("  Title: {title}");
                }
                if let Some(authors) = metadata.get("authors").and_then(|v| v.as_array()) {
                    if !authors.is_empty() {
                        let authors_str: Vec<String> = authors
                            .iter()
                            .filter_map(|a| a.as_str().map(String::from))
                            .collect();
                        println!("  Authors: {}", authors_str.join(", "));
                    }
                }
            }

            println!();
        }
    }

    Ok(())
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
const GUIDEBOOK_TEMPLATE: &str = include_str!("../../fixtures/init/base-config.yml");

// Include authoritative builtin policies from fixtures
const ALWAYS_INJECT_POLICY: &str =
    include_str!("../../fixtures/builtins/always_inject_on_prompt.rego");
const GLOBAL_FILE_LOCK_POLICY: &str = include_str!("../../fixtures/builtins/global_file_lock.rego");
const GIT_PRE_CHECK_POLICY: &str = include_str!("../../fixtures/builtins/git_pre_check.rego");
const POST_EDIT_CHECK_POLICY: &str = include_str!("../../fixtures/builtins/post_edit_check.rego");
const RULEBOOK_SECURITY_POLICY: &str =
    include_str!("../../fixtures/builtins/rulebook_security_guardrails.rego");
const PROTECTED_PATHS_POLICY: &str = include_str!("../../fixtures/builtins/protected_paths.rego");
const GIT_BLOCK_NO_VERIFY_POLICY: &str =
    include_str!("../../fixtures/builtins/git_block_no_verify.rego");
const ENFORCE_FULL_FILE_READ_POLICY: &str =
    include_str!("../../fixtures/builtins/enforce_full_file_read.rego");

// Global builtin policies embedded in the binary
const GLOBAL_SYSTEM_PROTECTION_POLICY: &str =
    include_str!("../../fixtures/global_builtins/system_protection.rego");
const GLOBAL_SENSITIVE_DATA_POLICY: &str =
    include_str!("../../fixtures/global_builtins/sensitive_data_protection.rego");
const GLOBAL_CUPCAKE_EXEC_POLICY: &str =
    include_str!("../../fixtures/global_builtins/cupcake_exec_protection.rego");

// Aligns with CRITICAL_GUIDING_STAR.md:
// - Simple CLI interface: cupcake eval
// - Takes policy directory as argument (decoupled from examples)
// - Verify command for testing Phase 1 implementation
// - Foundation for reading hook events from stdin (Phase 2)
