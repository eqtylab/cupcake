//! The Cupcake Engine - Core orchestration module
//! 
//! Implements the NEW_GUIDING_FINAL.md Hybrid Model architecture.
//! This intelligent engine provides:
//! - Metadata-driven policy discovery and routing
//! - Single aggregation entrypoint compilation
//! - Hybrid Model evaluation (Rego aggregation + Rust synthesis)
//! - O(1) routing performance via host-side indexing

use anyhow::{Context, Result};
// use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::{debug, error, info, warn};

pub mod metadata;
pub mod scanner;
pub mod compiler;
pub mod routing;
pub mod decision;
pub mod synthesis;
pub mod wasm_runtime;
pub mod guidebook;

// Re-export metadata types for public API
pub use metadata::{RoutingDirective, PolicyMetadata};

/// Represents a discovered policy unit with its metadata
/// Updated for NEW_GUIDING_FINAL.md metadata-driven routing
#[derive(Debug, Clone)]
pub struct PolicyUnit {
    /// Path to the .rego file
    pub path: PathBuf,
    
    /// The package name extracted from the policy
    pub package_name: String,
    
    /// The routing directive from OPA metadata
    pub routing: RoutingDirective,
    
    /// Complete metadata for this policy (optional)
    pub metadata: Option<PolicyMetadata>,
}

/// The main Engine struct - a black box with simple public API
pub struct Engine {
    /// Directory containing policies
    policy_dir: PathBuf,
    
    /// In-memory routing map: event criteria -> policy packages
    routing_map: HashMap<String, Vec<PolicyUnit>>,
    
    /// Compiled WASM module (stored as bytes)
    wasm_module: Option<Vec<u8>>,
    
    /// WASM runtime instance
    wasm_runtime: Option<wasm_runtime::WasmRuntime>,
    
    /// List of all discovered policies
    policies: Vec<PolicyUnit>,
    
    /// Optional guidebook for signals and actions
    guidebook: Option<guidebook::Guidebook>,
    
    // Removed: No longer need entrypoint mapping in Hybrid Model
}

impl Engine {
    /// Create a new engine instance with the given policy directory
    /// This is the primary public API - simple and clean
    pub async fn new(policy_dir: impl AsRef<Path>) -> Result<Self> {
        let policy_dir = policy_dir.as_ref().to_path_buf();
        
        info!("Initializing Cupcake Engine with policy directory: {:?}", policy_dir);
        
        // Create engine instance
        let mut engine = Self {
            policy_dir: policy_dir.clone(),
            routing_map: HashMap::new(),
            wasm_module: None,
            wasm_runtime: None,
            policies: Vec::new(),
            guidebook: None,
        };
        
        // Initialize the engine (scan, parse, compile)
        engine.initialize().await?;
        
        Ok(engine)
    }
    
    /// Initialize the engine by scanning, parsing, and compiling policies
    async fn initialize(&mut self) -> Result<()> {
        info!("Starting engine initialization...");
        
        // Step 1: Scan for .rego files
        let policy_files = scanner::scan_policies(&self.policy_dir).await?;
        info!("Found {} policy files", policy_files.len());
        
        // Step 2: Parse selectors and build policy units
        for path in policy_files {
            match self.parse_policy(&path).await {
                Ok(unit) => {
                    info!("Successfully parsed policy: {} from {:?}", unit.package_name, path);
                    self.policies.push(unit);
                }
                Err(e) => {
                    // Fail loudly but don't crash - log and skip bad policies
                    error!("Failed to parse policy at {:?}: {}", path, e);
                }
            }
        }
        
        if self.policies.is_empty() {
            warn!("No valid policies found in directory");
            return Ok(());
        }
        
        // Step 3: Build routing map
        self.build_routing_map();
        info!("Built routing map with {} entries", self.routing_map.len());
        
        // No entrypoint mapping needed - Hybrid Model uses single aggregation entrypoint
        
        // Step 4: Compile unified WASM module - MANDATORY
        let wasm_bytes = compiler::compile_policies(&self.policies).await?;
        info!("Successfully compiled unified WASM module ({} bytes)", wasm_bytes.len());
        self.wasm_module = Some(wasm_bytes.clone());
        
        // Step 5: Initialize WASM runtime
        self.wasm_runtime = Some(wasm_runtime::WasmRuntime::new(&wasm_bytes)?);
        info!("WASM runtime initialized");
        
        // Step 6: Load guidebook if it exists
        let guidebook_path = self.policy_dir.join("guidebook.yml");
        if guidebook_path.exists() {
            self.guidebook = Some(guidebook::Guidebook::load(&guidebook_path).await?);
            info!("Loaded guidebook from {:?}", guidebook_path);
        } else {
            info!("No guidebook.yml found - signals and actions disabled");
        }
        
        info!("Engine initialization complete");
        Ok(())
    }
    
    /// Parse a single policy file to extract selector and metadata
    async fn parse_policy(&self, path: &Path) -> Result<PolicyUnit> {
        let content = tokio::fs::read_to_string(path)
            .await
            .context("Failed to read policy file")?;
        
        // Extract package name
        let package_name = metadata::extract_package_name(&content)
            .context("Failed to extract package name")?;
        
        // Parse OPA metadata
        let policy_metadata = metadata::parse_metadata(&content)
            .context("Failed to parse OPA metadata")?;
        
        // Extract routing directive - allow system policies with empty routing
        let routing = if let Some(ref meta) = policy_metadata {
            if let Some(ref routing_directive) = meta.custom.routing {
                // Validate the routing directive
                metadata::validate_routing_directive(routing_directive)
                    .with_context(|| format!("Invalid routing directive in policy {}", package_name))?;
                routing_directive.clone()
            } else {
                warn!("Policy {} has no routing directive in metadata - will not be routed", package_name);
                return Err(anyhow::anyhow!("Policy missing routing directive"));
            }
        } else {
            warn!("Policy {} has no metadata block - will not be routed", package_name);
            return Err(anyhow::anyhow!("Policy missing metadata"));
        };
        
        Ok(PolicyUnit {
            path: path.to_path_buf(),
            package_name,
            routing,
            metadata: policy_metadata,
        })
    }
    
    /// Build the routing map from parsed policies
    fn build_routing_map(&mut self) {
        self.routing_map.clear();
        
        for policy in &self.policies {
            // Create all routing keys for this policy from metadata
            let routing_keys = routing::create_all_routing_keys_from_metadata(&policy.routing);
            
            // Add policy to the routing map for each key
            for key in routing_keys {
                self.routing_map
                    .entry(key)
                    .or_insert_with(Vec::new)
                    .push(policy.clone());
            }
        }
        
        // Handle wildcard routes - also add them to specific tool lookups
        // This allows "PreToolUse:Bash" to find both specific and wildcard policies
        let wildcard_keys: Vec<String> = self.routing_map.keys()
            .filter(|k| k.ends_with(":*"))
            .cloned()
            .collect();
            
        for wildcard_key in wildcard_keys {
            if let Some(wildcard_policies) = self.routing_map.get(&wildcard_key).cloned() {
                let event_prefix = wildcard_key.strip_suffix(":*").unwrap();
                
                // Find all specific tool keys for this event
                let specific_keys: Vec<String> = self.routing_map.keys()
                    .filter(|k| k.starts_with(&format!("{}:", event_prefix)) && !k.ends_with(":*"))
                    .cloned()
                    .collect();
                
                // Add wildcard policies to each specific tool key
                for specific_key in specific_keys {
                    self.routing_map
                        .entry(specific_key)
                        .or_insert_with(Vec::new)
                        .extend(wildcard_policies.clone());
                }
            }
        }
        
        // Log the routing map for verification
        for (key, policies) in &self.routing_map {
            debug!(
                "Route '{}' -> {} policies: [{}]",
                key,
                policies.len(),
                policies.iter()
                    .map(|p| p.package_name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
    }
    
    /// Get the routing map (for verification/testing)
    pub fn routing_map(&self) -> &HashMap<String, Vec<PolicyUnit>> {
        &self.routing_map
    }
    
    /// Get the compiled WASM module (for verification/testing)
    pub fn wasm_module(&self) -> Option<&[u8]> {
        self.wasm_module.as_deref()
    }
    
    /// Find policies that match the given event criteria
    pub fn route_event(&self, event_name: &str, tool_name: Option<&str>) -> Vec<&PolicyUnit> {
        let key = routing::create_event_key(event_name, tool_name);
        
        self.routing_map
            .get(&key)
            .map(|policies| policies.iter().collect())
            .unwrap_or_default()
    }
    
    /// Evaluate policies for a hook event - THE MAIN PUBLIC API
    /// Implements the NEW_GUIDING_FINAL.md Hybrid Model evaluation flow
    pub async fn evaluate(&mut self, input: &Value) -> Result<decision::FinalDecision> {
        // Extract event info from input for routing
        // Try both camelCase and snake_case for compatibility
        let event_name = input.get("hookEventName")
            .or_else(|| input.get("hook_event_name"))
            .and_then(|v| v.as_str())
            .context("Missing hookEventName/hook_event_name in input")?;
            
        let tool_name = input.get("tool_name")
            .and_then(|v| v.as_str());
            
        info!("Evaluating event: {} tool: {:?}", event_name, tool_name);
        
        // Step 1: Route - find relevant policies (collect owned PolicyUnits)
        let matched_policies: Vec<PolicyUnit> = self.route_event(event_name, tool_name)
            .into_iter()
            .cloned()
            .collect();
            
        if matched_policies.is_empty() {
            info!("No policies matched for this event - allowing");
            return Ok(decision::FinalDecision::Allow { context: vec![] });
        }
        
        info!("Found {} matching policies", matched_policies.len());
        
        // Step 2: Evaluate using single aggregation entrypoint
        let decision_set = self.evaluate_decision_set(input).await?;
        
        // Step 3: Apply Intelligence Layer synthesis
        let final_decision = synthesis::SynthesisEngine::synthesize(&decision_set)?;
        
        info!("Synthesized final decision: {:?}", final_decision);
        
        Ok(final_decision)
    }
    
    /// Evaluate using the Hybrid Model single aggregation entrypoint
    async fn evaluate_decision_set(&mut self, input: &Value) -> Result<decision::DecisionSet> {
        let runtime = self.wasm_runtime.as_mut()
            .context("WASM runtime not initialized")?;
        
        debug!("Evaluating using single cupcake.system.evaluate entrypoint");
        
        // Query the single aggregation entrypoint
        let decision_set = runtime.query_decision_set(input)?;
        
        debug!("Raw DecisionSet from WASM: {} total decisions", decision_set.decision_count());
        debug!("DecisionSet summary: {}", synthesis::SynthesisEngine::summarize_decision_set(&decision_set));
        
        Ok(decision_set)
    }
    
    /// Trigger actions for violations (runs in background)
    fn trigger_actions(&self, violations: &[decision::ViolationObject]) {
        let Some(guidebook) = &self.guidebook else {
            return;
        };
        
        for violation in violations {
            let actions = guidebook.get_actions_for_violation(&violation.id);
            
            for action in actions {
                // Clone data for the spawned task
                let command = action.command.clone();
                let violation_id = violation.id.clone();
                
                // Fire and forget - spawn background task
                tokio::spawn(async move {
                    debug!("Executing action for violation {}: {}", violation_id, command);
                    
                    match tokio::process::Command::new("sh")
                        .arg("-c")
                        .arg(&command)
                        .output()
                        .await
                    {
                        Ok(output) => {
                            if output.status.success() {
                                debug!("Action for {} completed successfully", violation_id);
                            } else {
                                let stderr = String::from_utf8_lossy(&output.stderr);
                                error!("Action for {} failed: {}", violation_id, stderr);
                            }
                        }
                        Err(e) => {
                            error!("Failed to execute action for {}: {}", violation_id, e);
                        }
                    }
                });
            }
        }
    }
}

// Aligns with NEW_GUIDING_FINAL.md:
// - Hybrid Model: Rego aggregates decisions, Rust synthesizes outcomes
// - Metadata-driven routing replaces custom selector system
// - Single entrypoint (cupcake.system.evaluate) enables sub-millisecond performance
// - Intelligence Layer applies strict priority hierarchy in Rust
// - Foundation for "Simplicity for the User, Intelligence in the Engine"