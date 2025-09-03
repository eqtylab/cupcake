//! The Cupcake Engine - Core orchestration module
//! 
//! Implements the NEW_GUIDING_FINAL.md Hybrid Model architecture.
//! This intelligent engine provides:
//! - Metadata-driven policy discovery and routing
//! - Single aggregation entrypoint compilation
//! - Hybrid Model evaluation (Rego aggregation + Rust synthesis)
//! - O(1) routing performance via host-side indexing

use anyhow::{anyhow, Context, Result};
// use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Instant;
use tracing::{debug, error, info, instrument, trace, warn};

/// Project path resolution following .cupcake/ convention with optional global config
#[derive(Debug, Clone)]
pub struct ProjectPaths {
    /// Root project directory (contains .cupcake/)
    pub root: PathBuf,
    /// .cupcake/ directory
    pub cupcake_dir: PathBuf,
    /// Policies directory (.cupcake/policies/)
    pub policies: PathBuf,
    /// Signals directory (.cupcake/signals/)
    pub signals: PathBuf,
    /// Actions directory (.cupcake/actions/)
    pub actions: PathBuf,
    /// Guidebook file (.cupcake/guidebook.yml)
    pub guidebook: PathBuf,
    
    // Global configuration paths (optional - may not exist)
    /// Global config root directory
    pub global_root: Option<PathBuf>,
    /// Global policies directory
    pub global_policies: Option<PathBuf>,
    /// Global signals directory
    pub global_signals: Option<PathBuf>,
    /// Global actions directory
    pub global_actions: Option<PathBuf>,
    /// Global guidebook file
    pub global_guidebook: Option<PathBuf>,
}

impl ProjectPaths {
    /// Resolve project paths using convention over configuration
    /// Accepts either project root OR .cupcake/ directory for flexibility
    /// Also discovers global configuration if present
    pub fn resolve(input_path: impl AsRef<Path>) -> Result<Self> {
        let input = input_path.as_ref().to_path_buf();
        
        let (root, cupcake_dir) = if input.file_name() == Some(std::ffi::OsStr::new(".cupcake")) {
            // Input is .cupcake/ directory
            let root = input.parent()
                .ok_or_else(|| anyhow!(".cupcake directory has no parent"))?
                .to_path_buf();
            (root, input)
        } else if input.join(".cupcake").exists() {
            // Input is project root with .cupcake/ subdirectory
            let cupcake_dir = input.join(".cupcake");
            (input, cupcake_dir)
        } else {
            // Legacy: treat input as direct policies directory
            let root = input.parent().unwrap_or(&input).to_path_buf();
            let cupcake_dir = root.join(".cupcake");
            (root, cupcake_dir)
        };
        
        // Discover global configuration (optional - may not exist)
        let global_config = global_config::GlobalPaths::discover()
            .unwrap_or_else(|e| {
                debug!("Failed to discover global config: {}", e);
                None
            });
        
        // Extract global paths if config exists
        let (global_root, global_policies, global_signals, global_actions, global_guidebook) = 
            if let Some(global) = global_config {
                info!("Global configuration discovered at {:?}", global.root);
                (
                    Some(global.root),
                    Some(global.policies),
                    Some(global.signals),
                    Some(global.actions),
                    Some(global.guidebook),
                )
            } else {
                debug!("No global configuration found - using project config only");
                (None, None, None, None, None)
            };
        
        Ok(ProjectPaths {
            root,
            cupcake_dir: cupcake_dir.clone(),
            policies: cupcake_dir.join("policies"),
            signals: cupcake_dir.join("signals"),
            actions: cupcake_dir.join("actions"),
            guidebook: cupcake_dir.join("guidebook.yml"),
            global_root,
            global_policies,
            global_signals,
            global_actions,
            global_guidebook,
        })
    }
}

pub mod metadata;
pub mod scanner;
pub mod compiler;
pub mod routing;
pub mod decision;
pub mod synthesis;
pub mod wasm_runtime;
pub mod guidebook;
pub mod builtins;
pub mod trace;
pub mod global_config;

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
    /// Project paths following .cupcake/ convention
    paths: ProjectPaths,
    
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
    
    /// Optional trust verifier for script integrity
    trust_verifier: Option<crate::trust::TrustVerifier>,
    
    // Global configuration support (optional - may not exist)
    /// Global policies routing map
    global_routing_map: HashMap<String, Vec<PolicyUnit>>,
    
    /// Global WASM module (compiled separately)
    global_wasm_module: Option<Vec<u8>>,
    
    /// Global WASM runtime instance
    global_wasm_runtime: Option<wasm_runtime::WasmRuntime>,
    
    /// List of global policies
    global_policies: Vec<PolicyUnit>,
    
    /// Optional global guidebook
    global_guidebook: Option<guidebook::Guidebook>,
}

impl Engine {
    /// Create a new engine instance with the given project path
    /// Accepts either project root or .cupcake/ directory
    /// This is the primary public API - simple and clean
    pub async fn new(project_path: impl AsRef<Path>) -> Result<Self> {
        let paths = ProjectPaths::resolve(project_path)?;
        
        info!("Initializing Cupcake Engine");
        info!("Project root: {:?}", paths.root);
        info!("Policies directory: {:?}", paths.policies);
        
        // Create engine instance with both project and global support
        let mut engine = Self {
            paths,
            routing_map: HashMap::new(),
            wasm_module: None,
            wasm_runtime: None,
            policies: Vec::new(),
            guidebook: None,
            trust_verifier: None,
            // Initialize global fields (will be populated if global config exists)
            global_routing_map: HashMap::new(),
            global_wasm_module: None,
            global_wasm_runtime: None,
            global_policies: Vec::new(),
            global_guidebook: None,
        };
        
        // Initialize the engine (scan, parse, compile)
        engine.initialize().await?;
        
        Ok(engine)
    }
    
    /// Initialize the engine by scanning, parsing, and compiling policies
    async fn initialize(&mut self) -> Result<()> {
        info!("Starting engine initialization...");
        
        // Step 0A: Initialize global configuration first (if it exists)
        if self.paths.global_root.is_some() {
            info!("Global configuration detected - initializing global policies first");
            self.initialize_global().await?;
        }
        
        // Step 0B: Load project guidebook to get builtin configuration
        self.guidebook = Some(guidebook::Guidebook::load_with_conventions(
            &self.paths.guidebook,
            &self.paths.signals,
            &self.paths.actions,
        ).await?);
        info!("Project guidebook loaded with convention-based discovery");
        
        // Get list of enabled builtins for filtering
        let enabled_builtins = self.guidebook
            .as_ref()
            .map(|g| g.builtins.enabled_builtins())
            .unwrap_or_default();
        
        if !enabled_builtins.is_empty() {
            info!("Enabled builtins: {:?}", enabled_builtins);
        }
        
        // Step 1: Scan for .rego files in policies directory with builtin filtering
        let policy_files = scanner::scan_policies_with_filter(
            &self.paths.policies,
            &enabled_builtins
        ).await?;
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
        
        // Step 6: Try to initialize trust verifier (optional - don't fail if not enabled)
        self.initialize_trust_system().await;
        
        info!("Engine initialization complete");
        Ok(())
    }
    
    /// Initialize global configuration (policies, guidebook, WASM)
    async fn initialize_global(&mut self) -> Result<()> {
        info!("Initializing global configuration...");
        
        // Verify we have global paths
        let global_policies_path = self.paths.global_policies.as_ref()
            .context("Global policies path not set")?;
        let global_guidebook_path = self.paths.global_guidebook.as_ref()
            .context("Global guidebook path not set")?;
        
        // Load global guidebook
        if global_guidebook_path.exists() {
            self.global_guidebook = Some(guidebook::Guidebook::load_with_conventions(
                global_guidebook_path,
                self.paths.global_signals.as_ref().unwrap(),
                self.paths.global_actions.as_ref().unwrap(),
            ).await?);
            info!("Global guidebook loaded");
        }
        
        // Get global enabled builtins
        let global_enabled_builtins = self.global_guidebook
            .as_ref()
            .map(|g| g.builtins.enabled_builtins())
            .unwrap_or_default();
        
        // Scan for global policies
        if global_policies_path.exists() {
            let global_policy_files = scanner::scan_policies_with_filter(
                global_policies_path,
                &global_enabled_builtins
            ).await?;
            info!("Found {} global policy files", global_policy_files.len());
            
            // Parse global policies
            for path in global_policy_files {
                match self.parse_policy(&path).await {
                    Ok(mut unit) => {
                        // Transform package name to global namespace
                        if !unit.package_name.starts_with("cupcake.global.") {
                            // Replace "cupcake.policies" with "cupcake.global.policies"
                            unit.package_name = unit.package_name
                                .replace("cupcake.policies", "cupcake.global.policies")
                                .replace("cupcake.system", "cupcake.global.system");
                        }
                        info!("Successfully parsed global policy: {} from {:?}", unit.package_name, path);
                        self.global_policies.push(unit);
                    }
                    Err(e) => {
                        error!("Failed to parse global policy at {:?}: {}", path, e);
                    }
                }
            }
            
            if !self.global_policies.is_empty() {
                // Check if we have non-system policies (OPA panics with only system policies)
                info!("Global policies found: {:?}", self.global_policies.iter().map(|p| &p.package_name).collect::<Vec<_>>());
                let non_system_count = self.global_policies.iter()
                    .filter(|p| !p.package_name.ends_with(".system"))
                    .count();
                    
                if non_system_count > 0 {
                    // Build global routing map
                    self.build_global_routing_map();
                    info!("Built global routing map with {} entries", self.global_routing_map.len());
                    
                    info!("Found {} global policies ({} non-system) - compiling global WASM module", 
                        self.global_policies.len(), non_system_count);
                    
                    // Compile global policies to WASM
                    let global_wasm_bytes = compiler::compile_policies_with_namespace(
                        &self.global_policies,
                        "cupcake.global.system"
                    ).await?;
                    info!("Successfully compiled global WASM module ({} bytes)", global_wasm_bytes.len());
                    self.global_wasm_module = Some(global_wasm_bytes.clone());
                    
                    // Initialize global WASM runtime with global namespace
                    self.global_wasm_runtime = Some(
                        wasm_runtime::WasmRuntime::new_with_namespace(&global_wasm_bytes, "cupcake.global.system")?
                    );
                    info!("Global WASM runtime initialized with namespace: cupcake.global.system");
                } else {
                    info!("Only system policies found in global config - skipping global WASM compilation");
                }
            }
        }
        
        info!("Global configuration initialization complete");
        Ok(())
    }
    
    /// Build routing map for global policies
    fn build_global_routing_map(&mut self) {
        Self::build_routing_map_generic(&self.global_policies, &mut self.global_routing_map, "global");
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
        
        // Extract routing directive - system policies don't need routing
        let routing = if let Some(ref meta) = policy_metadata {
            if let Some(ref routing_directive) = meta.custom.routing {
                // Validate the routing directive
                metadata::validate_routing_directive(routing_directive, &package_name)
                    .with_context(|| format!("Invalid routing directive in policy {}", package_name))?;
                routing_directive.clone()
            } else if package_name.ends_with(".system") {
                // System policies don't need routing - they're aggregation endpoints
                // This covers both cupcake.system and cupcake.global.system
                debug!("System policy {} has no routing directive (this is expected)", package_name);
                RoutingDirective::default()
            } else {
                warn!("Policy {} has no routing directive in metadata - will not be routed", package_name);
                return Err(anyhow::anyhow!("Policy missing routing directive"));
            }
        } else {
            // System policies are allowed to have no metadata
            if package_name.ends_with(".system") {
                debug!("System policy {} has no metadata block (this is allowed)", package_name);
                RoutingDirective::default()
            } else {
                warn!("Policy {} has no metadata block - will not be routed", package_name);
                return Err(anyhow::anyhow!("Policy missing metadata"));
            }
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
        Self::build_routing_map_generic(&self.policies, &mut self.routing_map, "project");
    }
    
    /// Generic method to build routing maps for both global and project policies
    fn build_routing_map_generic(
        policies: &[PolicyUnit],
        routing_map: &mut HashMap<String, Vec<PolicyUnit>>,
        map_type: &str
    ) {
        routing_map.clear();
        
        for policy in policies {
            // Create all routing keys for this policy from metadata
            let routing_keys = routing::create_all_routing_keys_from_metadata(&policy.routing);
            
            // Add policy to the routing map for each key
            for key in routing_keys {
                routing_map
                    .entry(key.clone())
                    .or_insert_with(Vec::new)
                    .push(policy.clone());
                debug!("Added {} policy {} to routing key: {}", map_type, policy.package_name, key);
            }
        }
        
        // Handle wildcard routes - also add them to specific tool lookups
        // This allows "PreToolUse:Bash" to find both specific and wildcard policies
        let wildcard_keys: Vec<String> = routing_map.keys()
            .filter(|k| k.ends_with(":*"))
            .cloned()
            .collect();
            
        for wildcard_key in wildcard_keys {
            if let Some(wildcard_policies) = routing_map.get(&wildcard_key).cloned() {
                let event_prefix = wildcard_key.strip_suffix(":*").unwrap();
                
                // Find all specific tool keys for this event
                let specific_keys: Vec<String> = routing_map.keys()
                    .filter(|k| k.starts_with(&format!("{}:", event_prefix)) && !k.ends_with(":*"))
                    .cloned()
                    .collect();
                
                // Add wildcard policies to each specific tool key
                for specific_key in specific_keys {
                    routing_map
                        .entry(specific_key)
                        .or_insert_with(Vec::new)
                        .extend(wildcard_policies.clone());
                }
            }
        }
        
        // Log the routing map for verification
        for (key, policies) in routing_map {
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
    
    /// Get the global routing map (for verification/testing)
    pub fn global_routing_map(&self) -> &HashMap<String, Vec<PolicyUnit>> {
        &self.global_routing_map
    }
    
    /// Get the compiled global WASM module (for verification/testing)
    pub fn global_wasm_module(&self) -> Option<&[u8]> {
        self.global_wasm_module.as_deref()
    }
    
    /// Find policies that match the given event criteria
    #[instrument(
        name = "route_event",
        skip(self),
        fields(
            routing_key = %routing::create_event_key(event_name, tool_name),
            matched_count = tracing::field::Empty,
            policy_names = tracing::field::Empty
        )
    )]
    pub fn route_event(&self, event_name: &str, tool_name: Option<&str>) -> Vec<&PolicyUnit> {
        let start = Instant::now();
        let key = routing::create_event_key(event_name, tool_name);
        
        // First try the specific key
        let mut result: Vec<&PolicyUnit> = self.routing_map
            .get(&key)
            .map(|policies| policies.iter().collect())
            .unwrap_or_default();
            
        // If we have a tool name, also check for policies that match the event without tool constraints
        // (these are treated as wildcards that match any tool)
        if tool_name.is_some() {
            let wildcard_key = event_name.to_string();
            if let Some(wildcard_policies) = self.routing_map.get(&wildcard_key) {
                result.extend(wildcard_policies.iter());
            }
        }
        
        // Record matched policies
        let current_span = tracing::Span::current();
        current_span.record("matched_count", &result.len());
        if !result.is_empty() {
            let policy_names: Vec<&str> = result.iter()
                .map(|p| p.package_name.as_str())
                .collect();
            current_span.record("policy_names", &format!("{:?}", policy_names).as_str());
        }
        
        trace!(
            duration_us = start.elapsed().as_micros(),
            matched = result.len(),
            "Policy routing complete"
        );
        
        result
    }
    
    /// Evaluate policies for a hook event - THE MAIN PUBLIC API
    /// Implements the NEW_GUIDING_FINAL.md Hybrid Model evaluation flow
    #[instrument(
        name = "evaluate",
        skip(self, input),
        fields(
            trace_id = %trace::generate_trace_id(),
            event_name = tracing::field::Empty,
            tool_name = tracing::field::Empty,
            session_id = tracing::field::Empty,
            matched_policy_count = tracing::field::Empty,
            final_decision = tracing::field::Empty,
            duration_ms = tracing::field::Empty
        )
    )]
    pub async fn evaluate(&self, input: &Value) -> Result<decision::FinalDecision> {
        let eval_start = Instant::now();
        
        // Extract event info from input for routing
        // Try both camelCase and snake_case for compatibility
        let event_name = input.get("hookEventName")
            .or_else(|| input.get("hook_event_name"))
            .and_then(|v| v.as_str())
            .context("Missing hookEventName/hook_event_name in input")?;
            
        let tool_name = input.get("tool_name")
            .and_then(|v| v.as_str());
            
        // Set span fields
        let current_span = tracing::Span::current();
        current_span.record("event_name", &event_name);
        current_span.record("tool_name", &tool_name);
        if let Some(session_id) = trace::extract_session_id(input) {
            current_span.record("session_id", &session_id.as_str());
        }
            
        info!("Evaluating event: {} tool: {:?}", event_name, tool_name);
        
        // PHASE 1: Evaluate global policies first (if they exist)
        if self.global_wasm_runtime.is_some() {
            debug!("Phase 1: Evaluating global policies");
            let (global_decision, global_decision_set) = self.evaluate_global(input, event_name, tool_name).await?;
            
            // Early termination on global blocking decisions
            match &global_decision {
                decision::FinalDecision::Halt { reason } => {
                    info!("Global policy HALT - immediate termination: {}", reason);
                    // Execute global actions before returning
                    if let Some(ref global_guidebook) = self.global_guidebook {
                        info!("Global guidebook found - executing global actions");
                        self.execute_actions_with_guidebook(&global_decision, &global_decision_set, global_guidebook).await;
                    } else {
                        warn!("No global guidebook - cannot execute global actions!");
                    }
                    current_span.record("final_decision", "GlobalHalt");
                    current_span.record("duration_ms", &eval_start.elapsed().as_millis());
                    return Ok(global_decision);
                }
                decision::FinalDecision::Deny { reason } => {
                    info!("Global policy DENY - immediate termination: {}", reason);
                    // Execute global actions before returning
                    if let Some(ref global_guidebook) = self.global_guidebook {
                        self.execute_actions_with_guidebook(&global_decision, &global_decision_set, global_guidebook).await;
                    }
                    current_span.record("final_decision", "GlobalDeny");
                    current_span.record("duration_ms", &eval_start.elapsed().as_millis());
                    return Ok(global_decision);
                }
                decision::FinalDecision::Block { reason } => {
                    info!("Global policy BLOCK - immediate termination: {}", reason);
                    // Execute global actions before returning
                    if let Some(ref global_guidebook) = self.global_guidebook {
                        self.execute_actions_with_guidebook(&global_decision, &global_decision_set, global_guidebook).await;
                    }
                    current_span.record("final_decision", "GlobalBlock");
                    current_span.record("duration_ms", &eval_start.elapsed().as_millis());
                    return Ok(global_decision);
                }
                _ => {
                    debug!("Global policies did not halt/deny/block - proceeding to project evaluation");
                    // TODO: In the future, preserve Ask/Allow/Context for merging with project decisions
                }
            }
        }
        
        // PHASE 2: Evaluate project policies
        debug!("Phase 2: Evaluating project policies");
        
        // Step 1: Route - find relevant policies (collect owned PolicyUnits)
        let matched_policies: Vec<PolicyUnit> = self.route_event(event_name, tool_name)
            .into_iter()
            .cloned()
            .collect();
            
        if matched_policies.is_empty() {
            info!("No policies matched for this event - allowing");
            current_span.record("matched_policy_count", &0);
            current_span.record("final_decision", "Allow");
            current_span.record("duration_ms", &eval_start.elapsed().as_millis());
            return Ok(decision::FinalDecision::Allow { context: vec![] });
        }
        
        current_span.record("matched_policy_count", &matched_policies.len());
        info!("Found {} matching policies", matched_policies.len());
        
        // Step 2: Gather Signals - collect all required signals from matched policies
        let enriched_input = self.gather_signals(input, &matched_policies).await?;
        
        // Debug output for testing
        if let Some(params) = input.get("params") {
            if let Some(file_path) = params.get("file_path") {
                if let Some(path_str) = file_path.as_str() {
                    if path_str.ends_with(".fail") {
                        eprintln!("DEBUG: Enriched input for .fail file: {}", 
                            serde_json::to_string_pretty(&enriched_input).unwrap_or_else(|_| "failed to serialize".to_string()));
                    }
                }
            }
        }
        
        // Step 3: Evaluate using single aggregation entrypoint with enriched input
        debug!("About to evaluate decision set with enriched input");
        let decision_set = self.evaluate_decision_set(&enriched_input).await?;
        
        // Step 4: Apply Intelligence Layer synthesis
        let final_decision = synthesis::SynthesisEngine::synthesize(&decision_set)?;
        
        info!("Synthesized final decision: {:?}", final_decision);
        
        // Step 5: Execute actions based on decision (async, non-blocking)
        self.execute_actions(&final_decision, &decision_set).await;
        
        // Record final decision type and duration
        let duration = eval_start.elapsed();
        let current_span = tracing::Span::current();
        current_span.record("final_decision", &format!("{:?}", final_decision).as_str());
        current_span.record("duration_ms", &duration.as_millis());
        
        trace!(
            decision = ?final_decision,
            duration_ms = duration.as_millis(),
            "Evaluation complete"
        );
        
        Ok(final_decision)
    }
    
    /// Evaluate global policies
    async fn evaluate_global(
        &self,
        input: &Value,
        event_name: &str,
        tool_name: Option<&str>,
    ) -> Result<(decision::FinalDecision, decision::DecisionSet)> {
        // Route through global policies
        let global_matched: Vec<PolicyUnit> = self.route_global_event(event_name, tool_name)
            .into_iter()
            .cloned()
            .collect();
        
        if global_matched.is_empty() {
            debug!("No global policies matched for this event");
            return Ok((
                decision::FinalDecision::Allow { context: vec![] },
                decision::DecisionSet::default()
            ));
        }
        
        info!("Found {} matching global policies", global_matched.len());
        
        // Gather signals for global policies (using global guidebook if available)
        let enriched_input = if let Some(ref global_guidebook) = self.global_guidebook {
            self.gather_signals_with_guidebook(input, &global_matched, global_guidebook).await?
        } else {
            input.clone()
        };
        
        // Evaluate using global WASM runtime
        let global_runtime = self.global_wasm_runtime.as_ref()
            .context("Global WASM runtime not initialized")?;
        
        let global_decision_set = global_runtime.query_decision_set(&enriched_input)?;
        debug!("Global DecisionSet: {} total decisions", global_decision_set.decision_count());
        
        // Synthesize global decision
        let global_decision = synthesis::SynthesisEngine::synthesize(&global_decision_set)?;
        info!("Global policy decision: {:?}", global_decision);
        
        Ok((global_decision, global_decision_set))
    }
    
    /// Route event through global policies
    fn route_global_event(&self, event_name: &str, tool_name: Option<&str>) -> Vec<&PolicyUnit> {
        let key = routing::create_event_key(event_name, tool_name);
        
        // First try the specific key
        let mut result: Vec<&PolicyUnit> = self.global_routing_map
            .get(&key)
            .map(|policies| policies.iter().collect())
            .unwrap_or_default();
            
        // If we have a tool name, also check for wildcards
        if tool_name.is_some() {
            let wildcard_key = event_name.to_string();
            if let Some(wildcard_policies) = self.global_routing_map.get(&wildcard_key) {
                result.extend(wildcard_policies.iter());
            }
        }
        
        debug!("Routed to {} global policies", result.len());
        result
    }
    
    /// Gather signals with a specific guidebook
    async fn gather_signals_with_guidebook(
        &self,
        input: &Value,
        matched_policies: &[PolicyUnit],
        guidebook: &guidebook::Guidebook,
    ) -> Result<Value> {
        // Collect required signals
        let mut required_signals = std::collections::HashSet::new();
        for policy in matched_policies {
            for signal_name in &policy.routing.required_signals {
                required_signals.insert(signal_name.clone());
            }
        }
        
        // ALSO: For builtin policies, automatically include their generated signals
        // This mirrors the logic in gather_signals() for project builtins
        for policy in matched_policies {
            // Check for both global and project builtin namespaces
            let is_global_builtin = policy.package_name.starts_with("cupcake.global.policies.builtins.");
            let is_project_builtin = policy.package_name.starts_with("cupcake.policies.builtins.");
            
            if is_global_builtin || is_project_builtin {
                // Extract the builtin name from the package
                let prefix = if is_global_builtin {
                    "cupcake.global.policies.builtins."
                } else {
                    "cupcake.policies.builtins."
                };
                
                let builtin_name = policy.package_name
                    .strip_prefix(prefix)
                    .unwrap_or("");
                
                // Add all signals that match this builtin's pattern
                let signal_prefix = format!("__builtin_{}_", builtin_name);
                for signal_name in guidebook.signals.keys() {
                    if signal_name.starts_with(&signal_prefix) {
                        debug!("Auto-adding signal '{}' for global builtin '{}'", signal_name, builtin_name);
                        required_signals.insert(signal_name.clone());
                    }
                }
                
                // Also check for signals without the trailing underscore (like __builtin_system_protection_paths)
                let signal_prefix_no_underscore = format!("__builtin_{}", builtin_name);
                for signal_name in guidebook.signals.keys() {
                    if signal_name.starts_with(&signal_prefix_no_underscore) && !signal_name.starts_with(&signal_prefix) {
                        debug!("Auto-adding signal '{}' for global builtin '{}'", signal_name, builtin_name);
                        required_signals.insert(signal_name.clone());
                    }
                }
            }
        }
        
        if required_signals.is_empty() {
            return Ok(input.clone());
        }
        
        let signal_names: Vec<String> = required_signals.into_iter().collect();
        info!("Gathering {} signals from guidebook", signal_names.len());
        
        // Execute signals using the provided guidebook
        let signal_data = self.execute_signals_from_guidebook(&signal_names, guidebook).await
            .unwrap_or_else(|e| {
                warn!("Signal execution failed: {}", e);
                std::collections::HashMap::new()
            });
        
        // Merge signal data into input
        let mut enriched_input = input.clone();
        if let Some(obj) = enriched_input.as_object_mut() {
            obj.insert("signals".to_string(), serde_json::json!(signal_data));
        }
        
        Ok(enriched_input)
    }
    
    /// Execute signals from a specific guidebook
    async fn execute_signals_from_guidebook(
        &self,
        signal_names: &[String],
        guidebook: &guidebook::Guidebook,
    ) -> Result<std::collections::HashMap<String, Value>> {
        // Use the guidebook's execute_signals method directly
        guidebook.execute_signals(signal_names).await
    }
    
    /// Evaluate using the Hybrid Model single aggregation entrypoint
    async fn evaluate_decision_set(&self, input: &Value) -> Result<decision::DecisionSet> {
        let runtime = self.wasm_runtime.as_ref()
            .context("WASM runtime not initialized")?;
        
        debug!("Evaluating using single cupcake.system.evaluate entrypoint");
        
        // Query the single aggregation entrypoint
        let decision_set = runtime.query_decision_set(input)?;
        
        debug!("Raw DecisionSet from WASM: {} total decisions", decision_set.decision_count());
        debug!("DecisionSet summary: {}", synthesis::SynthesisEngine::summarize_decision_set(&decision_set));
        
        Ok(decision_set)
    }
    
    /// Gather signals required by the matched policies and enrich the input
    #[instrument(
        name = "gather_signals",
        skip(self, input, matched_policies),
        fields(
            signal_count = tracing::field::Empty,
            signals_executed = tracing::field::Empty,
            duration_ms = tracing::field::Empty
        )
    )]
    async fn gather_signals(&self, input: &Value, matched_policies: &[PolicyUnit]) -> Result<Value> {
        let start = Instant::now();
        // Collect all unique required signals from matched policies
        let mut required_signals = std::collections::HashSet::new();
        for policy in matched_policies {
            for signal_name in &policy.routing.required_signals {
                required_signals.insert(signal_name.clone());
            }
        }
        
        // ALSO: For builtin policies, automatically include their generated signals
        // This is needed because builtin policies can't statically declare dynamic signals
        if let Some(guidebook) = &self.guidebook {
            for policy in matched_policies {
                if policy.package_name.starts_with("cupcake.policies.builtins.") {
                    // This is a builtin policy - add signals that match its pattern
                    let builtin_name = policy.package_name
                        .strip_prefix("cupcake.policies.builtins.")
                        .unwrap_or("");
                    
                    // Special handling for post_edit_check - only add the signal for the actual file extension
                    if builtin_name == "post_edit_check" {
                        // Extract file extension from input if available
                        if let Some(params) = input.get("params") {
                            if let Some(file_path) = params.get("file_path") {
                                if let Some(path_str) = file_path.as_str() {
                                    if let Some(extension) = std::path::Path::new(path_str)
                                        .extension()
                                        .and_then(|e| e.to_str()) {
                                        // Only add the signal for this specific extension
                                        let signal_name = format!("__builtin_post_edit_{}", extension);
                                        if guidebook.signals.contains_key(&signal_name) {
                                            debug!("Auto-adding signal '{}' for file extension '{}'", signal_name, extension);
                                            required_signals.insert(signal_name);
                                        }
                                    }
                                }
                            }
                        }
                    } else {
                        // For other builtins, add all matching signals
                        let signal_prefix = format!("__builtin_{}_", builtin_name);
                        for signal_name in guidebook.signals.keys() {
                            if signal_name.starts_with(&signal_prefix) {
                                debug!("Auto-adding signal '{}' for builtin '{}'", signal_name, builtin_name);
                                required_signals.insert(signal_name.clone());
                            }
                        }
                    }
                }
            }
        }
        
        if required_signals.is_empty() {
            debug!("No signals required - using original input");
            return Ok(input.clone());
        }
        
        let signal_names: Vec<String> = required_signals.into_iter().collect();
        info!("Gathering {} signals: {:?}", signal_names.len(), signal_names);
        
        // Execute signals if we have a guidebook
        let signal_data = if let Some(guidebook) = &self.guidebook {
            self.execute_signals_with_trust(&signal_names, guidebook).await.unwrap_or_else(|e| {
                warn!("Signal execution failed: {}", e);
                std::collections::HashMap::<String, serde_json::Value>::new()
            })
        } else {
            debug!("No guidebook available - no signals collected");
            std::collections::HashMap::<String, serde_json::Value>::new()
        };
        
        // Merge signal data into input
        let signal_count = signal_data.len();
        let mut enriched_input = input.clone();
        if let Some(input_obj) = enriched_input.as_object_mut() {
            input_obj.insert("signals".to_string(), serde_json::to_value(signal_data)?);
        }
        
        
        // Record span fields
        let duration = start.elapsed();
        let current_span = tracing::Span::current();
        current_span.record("signal_count", &signal_count);
        current_span.record("signals_executed", &signal_names.join(",").as_str());
        current_span.record("duration_ms", &duration.as_millis());
        
        debug!("Input enriched with {} signal values", signal_count);
        trace!(
            signal_count = signal_count,
            duration_ms = duration.as_millis(),
            "Signal gathering complete"
        );
        
        Ok(enriched_input)
    }
    
    /// Execute signals with trust verification
    async fn execute_signals_with_trust(
        &self,
        signal_names: &[String],
        guidebook: &guidebook::Guidebook,
    ) -> Result<HashMap<String, serde_json::Value>> {
        use futures::future::join_all;
        
        if signal_names.is_empty() {
            return Ok(HashMap::new());
        }
        
        debug!("Executing {} signals with trust verification", signal_names.len());
        
        let futures: Vec<_> = signal_names.iter()
            .map(|name| {
                let name = name.clone();
                let trust_verifier = self.trust_verifier.clone();
                let signal_config = guidebook.get_signal(&name).cloned();
                
                async move {
                    // Get the signal config
                    let signal = match signal_config {
                        Some(s) => s,
                        None => {
                            return (name.clone(), Err(anyhow::anyhow!("Signal '{}' not found", name)));
                        }
                    };
                    
                    // Verify trust if enabled
                    if let Some(verifier) = &trust_verifier {
                        if let Err(e) = verifier.verify_script(&signal.command).await {
                            // Log trust violation specially (TrustError logs itself on creation)
                            return (name.clone(), Err(anyhow::anyhow!("Trust verification failed: {}", e)));
                        }
                    }
                    
                    // Execute the signal (using the existing guidebook method)
                    let result = guidebook.execute_signal(&name).await;
                    (name, result)
                }
            })
            .collect();
        
        let results = join_all(futures).await;
        
        let mut signal_data = HashMap::new();
        for (name, result) in results {
            match result {
                Ok(value) => {
                    debug!("Signal '{}' executed successfully", name);
                    signal_data.insert(name, value);
                }
                Err(e) => {
                    // Log error but don't fail the whole evaluation
                    error!("Signal '{}' failed: {}", name, e);
                }
            }
        }
        
        Ok(signal_data)
    }
    
    /// Execute actions based on the final decision and decision set
    /// Execute actions with a specific guidebook
    async fn execute_actions_with_guidebook(
        &self, 
        final_decision: &decision::FinalDecision, 
        decision_set: &decision::DecisionSet,
        guidebook: &guidebook::Guidebook
    ) {
        info!("execute_actions_with_guidebook called with decision: {:?}", final_decision);
        
        // Determine working directory based on whether this is a global guidebook
        // Check if we're using the global guidebook by comparing pointers
        let is_global = self.global_guidebook.as_ref()
            .map(|gb| std::ptr::eq(gb as *const _, guidebook as *const _))
            .unwrap_or(false);
        
            
        let working_dir = if is_global {
            // For global actions, use global config root if available
            if let Ok(Some(global_paths)) = global_config::GlobalPaths::discover() {
                global_paths.root
            } else {
                self.paths.root.clone()
            }
        } else {
            self.paths.root.clone()
        };
        
        // Execute actions based on decision type
        match final_decision {
            decision::FinalDecision::Halt { reason } => {
                info!("Executing actions for HALT decision: {}", reason);
                info!("Number of halt decisions in set: {}", decision_set.halts.len());
                self.execute_rule_specific_actions(&decision_set.halts, guidebook, &working_dir).await;
            },
            decision::FinalDecision::Deny { reason } => {
                info!("Executing actions for DENY decision: {}", reason);
                
                // Execute general denial actions
                for action in &guidebook.actions.on_any_denial {
                    self.execute_single_action(action, &working_dir).await;
                }
                
                // Execute rule-specific actions for denials
                self.execute_rule_specific_actions(&decision_set.denials, guidebook, &working_dir).await;
            },
            decision::FinalDecision::Block { reason } => {
                info!("Executing actions for BLOCK decision: {}", reason);
                self.execute_rule_specific_actions(&decision_set.blocks, guidebook, &working_dir).await;
            },
            decision::FinalDecision::Ask { .. } => {
                debug!("ASK decision - no automatic actions");
            },
            decision::FinalDecision::Allow { .. } => {
                debug!("ALLOW decision - no actions needed");
            },
            decision::FinalDecision::AllowOverride { .. } => {
                debug!("ALLOW_OVERRIDE decision - no actions needed");
            },
        }
    }

    /// Execute actions using project guidebook (for backward compatibility)
    async fn execute_actions(&self, final_decision: &decision::FinalDecision, decision_set: &decision::DecisionSet) {
        let Some(guidebook) = &self.guidebook else {
            debug!("No guidebook available - no actions to execute");
            return;
        };
        
        self.execute_actions_with_guidebook(final_decision, decision_set, guidebook).await;
    }
    
    /// Execute actions for a specific set of decision objects (by rule ID)
    async fn execute_rule_specific_actions(&self, decisions: &[decision::DecisionObject], guidebook: &guidebook::Guidebook, working_dir: &std::path::PathBuf) {
        info!("execute_rule_specific_actions: Checking actions for {} decision objects", decisions.len());
        info!("Available action rules: {:?}", guidebook.actions.by_rule_id.keys().collect::<Vec<_>>());
        
        for decision_obj in decisions {
            let rule_id = &decision_obj.rule_id;
            info!("Looking for actions for rule ID: {}", rule_id);
            
            if let Some(actions) = guidebook.actions.by_rule_id.get(rule_id) {
                info!("Found {} actions for rule {}", actions.len(), rule_id);
                for action in actions {
                    info!("About to execute action: {}", action.command);
                    self.execute_single_action(action, working_dir).await;
                }
            } else {
                info!("No actions found for rule ID: {}", rule_id);
            }
        }
    }
    
    /// Execute a single action command
    async fn execute_single_action(&self, action: &guidebook::ActionConfig, working_dir: &std::path::PathBuf) {
        debug!("Executing action: {} in directory: {:?}", action.command, working_dir);
        
        // Verify trust if enabled
        if let Some(verifier) = &self.trust_verifier {
            if let Err(e) = verifier.verify_script(&action.command).await {
                // Log trust violation specially (TrustError logs itself on creation)
                error!("Action blocked by trust verification: {}", e);
                return; // Don't execute untrusted action
            }
        }
        
        // Use the provided working directory
        let working_dir = working_dir.clone();
        
        // Execute action asynchronously without blocking
        let command = action.command.clone();
        tokio::spawn(async move {
            
            // Determine if this is a script file or a shell command
            // A script file starts with / or ./ and doesn't contain shell operators
            let is_script = (command.starts_with('/') || command.starts_with("./")) 
                && !command.contains("&&") 
                && !command.contains("||") 
                && !command.contains(';')
                && !command.contains('|')
                && !command.contains('>');
                
            let result = if is_script {
                // It's a script file, execute directly
                // Extract the directory from the script path to use as working directory
                let script_path = std::path::Path::new(&command);
                let script_working_dir = script_path.parent().and_then(|p| p.parent()).and_then(|p| p.parent())
                    .unwrap_or(&working_dir);
                
                tokio::process::Command::new(&command)
                    .current_dir(script_working_dir)
                    .output()
                    .await
            } else {
                // It's a shell command, use sh -c
                // Use the provided working directory
                let result = tokio::process::Command::new("sh")
                    .arg("-c")
                    .arg(&command)
                    .current_dir(&working_dir)
                    .output()
                    .await;
                result
            };
                
            match result {
                Ok(output) => {
                    if output.status.success() {
                        debug!("Action completed successfully: {}", command);
                    } else {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        let stdout = String::from_utf8_lossy(&output.stdout);
                        warn!("Action failed: {} - stderr: {} stdout: {}", command, stderr, stdout);
                    }
                },
                Err(e) => {
                    error!("Failed to execute action: {} - {}", command, e);
                }
            }
        });
        
        // Give the runtime a chance to start the spawned task
        tokio::task::yield_now().await;
    }
    
    /// Initialize the trust system, respecting the mode setting
    async fn initialize_trust_system(&mut self) {
        let trust_path = self.paths.root.join(".cupcake").join(".trust");
        
        // First check if trust manifest exists
        if !trust_path.exists() {
            info!("Trust mode not initialized (optional) - run 'cupcake trust init' to enable");
            self.show_trust_startup_notification();
            return;
        }
        
        // Load manifest to check mode
        match crate::trust::TrustManifest::load(&trust_path) {
            Ok(manifest) => {
                // Check if trust is enabled or disabled
                if manifest.is_enabled() {
                    // Trust is enabled, create verifier
                    match crate::trust::TrustVerifier::new(&self.paths.root).await {
                        Ok(verifier) => {
                            info!("Trust mode ENABLED - script integrity verification active");
                            self.trust_verifier = Some(verifier);
                        }
                        Err(e) => {
                            warn!("Failed to initialize trust verifier: {}", e);
                            warn!("Continuing without trust verification");
                            self.show_trust_startup_notification();
                        }
                    }
                } else {
                    // Trust exists but is disabled
                    info!("Trust mode DISABLED by user - scripts will execute without verification");
                    self.show_trust_disabled_notification();
                    // Explicitly set verifier to None
                    self.trust_verifier = None;
                }
            }
            Err(crate::trust::TrustError::NotInitialized) => {
                // This shouldn't happen since we checked file exists, but handle it
                info!("Trust mode not initialized (optional) - run 'cupcake trust init' to enable");
                self.show_trust_startup_notification();
            }
            Err(e) => {
                // Manifest exists but can't be loaded (corruption, tampering, etc.)
                warn!("Failed to load trust manifest: {}", e);
                warn!("Continuing without trust verification for safety");
                self.show_trust_startup_notification();
            }
        }
    }
    
    /// Show notification when trust is disabled by user
    fn show_trust_disabled_notification(&self) {
        eprintln!("┌─────────────────────────────────────────────────────────┐");
        eprintln!("│ Trust Mode: DISABLED                                    │");
        eprintln!("│                                                         │");
        eprintln!("│ ⚠️  Script integrity verification is OFF                │");
        eprintln!("│ Scripts will execute without safety checks.            │");
        eprintln!("│                                                         │");
        eprintln!("│ To re-enable: cupcake trust enable                     │");
        eprintln!("└─────────────────────────────────────────────────────────┘");
        eprintln!();
    }
    
    /// Show startup notification about trust mode when it's not enabled
    fn show_trust_startup_notification(&self) {
        eprintln!("┌─────────────────────────────────────────────────────────┐");
        eprintln!("│ Cupcake is running in STANDARD mode                    │");
        eprintln!("│                                                         │");
        eprintln!("│ Script integrity verification is DISABLED.             │");
        eprintln!("│ Enable trust mode for enhanced security:               │");
        eprintln!("│   $ cupcake trust init                                  │");
        eprintln!("│                                                         │");
        eprintln!("│ Learn more: cupcake trust --help                       │");
        eprintln!("└─────────────────────────────────────────────────────────┘");
    }
    
}

// Aligns with NEW_GUIDING_FINAL.md:
// - Hybrid Model: Rego aggregates decisions, Rust synthesizes outcomes
// - Metadata-driven routing replaces custom selector system
// - Single entrypoint (cupcake.system.evaluate) enables sub-millisecond performance
// - Intelligence Layer applies strict priority hierarchy in Rust
// - Foundation for "Simplicity for the User, Intelligence in the Engine"