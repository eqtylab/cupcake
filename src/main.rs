//! Cupcake - A performant policy engine for coding agents
//! 
//! Main entry point implementing the CRITICAL_GUIDING_STAR architecture

use anyhow::{Context, Result};
use clap::Parser;
use serde_json::Value;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use tracing::{debug, error, info, warn};
use tracing_subscriber::EnvFilter;

mod engine;
mod harness;
mod trust;

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
    
    /// Initialize a new Cupcake project in the current directory
    Init,
    
    /// Manage script trust and integrity verification
    Trust {
        #[clap(subcommand)]
        command: trust::TrustCommand,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));
    
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .init();
    
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
        Command::Init => {
            init_command().await
        }
        Command::Trust { command } => {
            command.execute().await
        }
    }
}

async fn eval_command(policy_dir: PathBuf, strict: bool) -> Result<()> {
    debug!("Initializing Cupcake engine with policies from: {:?}", policy_dir);
    
    // Initialize the engine - MUST succeed or we exit
    let mut engine = match engine::Engine::new(&policy_dir).await {
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
    info!("Verifying Cupcake engine configuration...");
    info!("Policy directory: {:?}", policy_dir);
    
    // Initialize the engine - MUST succeed or we exit
    let engine = match engine::Engine::new(&policy_dir).await {
        Ok(e) => {
            info!("Engine initialized successfully");
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
    
    // Display routing map
    info!("=== Routing Map ===");
    for (key, policies) in engine.routing_map() {
        info!("  {} -> {} policies", key, policies.len());
        for policy in policies {
            info!("    - {}", policy.package_name);
        }
    }
    
    // Check WASM module - it MUST exist
    if let Some(wasm) = engine.wasm_module() {
        info!("=== WASM Module ===");
        info!("  Size: {} bytes", wasm.len());
        info!("  Compilation: SUCCESS");
    } else {
        // This should never happen now - engine initialization would have failed
        error!("CRITICAL: Engine has no WASM module - this should be impossible");
    }
    
    info!("Verification complete!");
    Ok(())
}

async fn init_command() -> Result<()> {
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
    fs::create_dir_all(".cupcake/signals")
        .context("Failed to create .cupcake/signals directory")?;
    fs::create_dir_all(".cupcake/actions")
        .context("Failed to create .cupcake/actions directory")?;
    
    // Write the one required file
    fs::write(
        ".cupcake/policies/system/evaluate.rego",
        SYSTEM_EVALUATE_TEMPLATE
    )
    .context("Failed to create system evaluate.rego file")?;
    
    // Write a simple example policy to avoid OPA compilation issues
    fs::write(
        ".cupcake/policies/example.rego",
        EXAMPLE_POLICY_TEMPLATE
    )
    .context("Failed to create example policy file")?;
    
    println!("✅ Initialized Cupcake project in .cupcake/");
    println!("   Add your policies to .cupcake/policies/");
    println!("   Add signal scripts to .cupcake/signals/");
    println!("   Add action scripts to .cupcake/actions/");
    
    Ok(())
}

const SYSTEM_EVALUATE_TEMPLATE: &str = r#"package cupcake.system

import rego.v1

# METADATA
# scope: rule
# title: System Aggregation Policy
# authors: ["Cupcake Engine"]

# Collect all decision verbs from the policy hierarchy
# Uses walk() for automatic policy discovery

halts := collect_verbs("halt")
denials := collect_verbs("deny") 
blocks := collect_verbs("block")
asks := collect_verbs("ask")
allow_overrides := collect_verbs("allow_override")
add_context := collect_verbs("add_context")

# Single evaluation entrypoint for the engine
evaluate := {
    "halts": halts,
    "denials": denials,
    "blocks": blocks,
    "asks": asks,
    "allow_overrides": allow_overrides,
    "add_context": add_context
}

# Collect all instances of a decision verb across all policies
collect_verbs(verb_name) := result if {
    verb_sets := [value |
        walk(data.cupcake.policies, [path, value])
        path[count(path) - 1] == verb_name
    ]
    all_decisions := [decision |
        some verb_set in verb_sets
        some decision in verb_set
    ]
    result := all_decisions
}
"#;

const EXAMPLE_POLICY_TEMPLATE: &str = r#"package cupcake.policies.example

import rego.v1

# METADATA
# scope: rule
# title: Example Policy
# description: A minimal example policy that never fires
# custom:
#   routing:
#     required_events: ["PreToolUse"]
#     required_tools: ["Bash"]

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

// Aligns with CRITICAL_GUIDING_STAR.md:
// - Simple CLI interface: cupcake eval
// - Takes policy directory as argument (decoupled from examples)
// - Verify command for testing Phase 1 implementation
// - Foundation for reading hook events from stdin (Phase 2)