//! The Cupcake Engine - Core orchestration module.
//!
//! Provides metadata-driven policy discovery, O(1) routing, WASM evaluation,
//! and decision synthesis.

use anyhow::{Context, Result};
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;
use std::time::Instant;
use tracing::{debug, error, info, instrument, trace, warn};

use crate::debug::SignalTelemetry;
use crate::telemetry::TelemetryContext;

// Core engine modules - discovery and compilation
pub mod compiler;
pub mod config;
pub mod executor;
pub mod metadata;
pub mod scanner;

// Catalog overlay support
#[cfg(feature = "catalog")]
pub mod catalog_overlay;

// Routing system
pub mod routing;
pub mod routing_debug;

// Policy evaluation
pub mod decision;
pub mod synthesis;
pub mod wasm_runtime;

// Configuration and extensions
pub mod builtins;
pub mod global_config;
pub mod rulebook;

// Diagnostics and debugging
pub mod trace;

// Re-export types for public API
pub use config::{EngineConfig, ProjectPaths, SHELL_COMMAND};
pub use metadata::{PolicyMetadata, PolicyUnit, RoutingDirective};
pub use rulebook::{TelemetryConfig, TelemetryFormat};

/// The main Engine struct - a black box with simple public API
pub struct Engine {
    /// Project paths following .cupcake/ convention
    paths: ProjectPaths,

    /// Engine configuration from CLI flags
    config: EngineConfig,

    /// In-memory routing map: event criteria -> policy packages
    routing_map: HashMap<String, Vec<PolicyUnit>>,

    /// Compiled WASM module (stored as bytes)
    wasm_module: Option<Vec<u8>>,

    /// WASM runtime instance
    wasm_runtime: Option<wasm_runtime::WasmRuntime>,

    /// List of all discovered policies
    policies: Vec<PolicyUnit>,

    /// Optional rulebook for signals and actions
    rulebook: Option<rulebook::Rulebook>,

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

    /// Optional global rulebook
    global_rulebook: Option<rulebook::Rulebook>,

    /// Watchdog LLM-as-judge instance (optional)
    watchdog: Option<crate::watchdog::Watchdog>,

    // Catalog overlay support (optional - may not have any installed)
    #[cfg(feature = "catalog")]
    /// Catalog overlays discovered in .cupcake/catalog/
    catalog_overlays: Vec<catalog_overlay::CatalogOverlay>,

    #[cfg(feature = "catalog")]
    /// Catalog overlay routing maps (one per overlay)
    catalog_routing_maps: Vec<HashMap<String, Vec<PolicyUnit>>>,

    #[cfg(feature = "catalog")]
    /// Catalog WASM runtimes (one per overlay)
    catalog_wasm_runtimes: Vec<wasm_runtime::WasmRuntime>,
}

impl Engine {
    /// Create a new engine instance with the given project path and harness type
    /// Accepts either project root or .cupcake/ directory
    /// For CLI flag overrides, use `new_with_config()`
    pub async fn new(
        project_path: impl AsRef<Path>,
        harness: crate::harness::types::HarnessType,
    ) -> Result<Self> {
        Self::new_with_config(project_path, EngineConfig::new(harness)).await
    }

    /// Create a new engine instance with custom configuration
    /// Accepts either project root or .cupcake/ directory
    /// This is the primary API for CLI integration with flag overrides
    pub async fn new_with_config(
        project_path: impl AsRef<Path>,
        config: EngineConfig,
    ) -> Result<Self> {
        let paths = ProjectPaths::resolve_with_config(project_path, config.global_config.clone())?;

        info!("Initializing Cupcake Engine");
        info!("Project root: {:?}", paths.root);
        info!("Policies directory: {:?}", paths.policies);

        if config.wasm_max_memory.is_some() {
            info!(
                "WASM max memory override: {} bytes",
                config.wasm_max_memory.unwrap()
            );
        }
        if config.opa_path.is_some() {
            info!("OPA path override: {:?}", config.opa_path);
        }
        if config.global_config.is_some() {
            info!("Global config override: {:?}", config.global_config);
        }
        if config.debug_routing {
            info!("Routing debug output enabled");
        }

        // Create engine instance with both project and global support
        let mut engine = Self {
            paths,
            config,
            routing_map: HashMap::new(),
            wasm_module: None,
            wasm_runtime: None,
            policies: Vec::new(),
            rulebook: None,
            trust_verifier: None,
            // Initialize global fields (will be populated if global config exists)
            global_routing_map: HashMap::new(),
            global_wasm_module: None,
            global_wasm_runtime: None,
            global_policies: Vec::new(),
            global_rulebook: None,
            // Watchdog initialized later from rulebook config
            watchdog: None,
            // Initialize catalog fields (will be populated if overlays are installed)
            #[cfg(feature = "catalog")]
            catalog_overlays: Vec::new(),
            #[cfg(feature = "catalog")]
            catalog_routing_maps: Vec::new(),
            #[cfg(feature = "catalog")]
            catalog_wasm_runtimes: Vec::new(),
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

        // Step 0A.5: Initialize catalog overlays (between global and project)
        #[cfg(feature = "catalog")]
        {
            if let Err(e) = self.initialize_catalog_overlays().await {
                warn!("Failed to initialize catalog overlays: {}", e);
                // Continue without catalog overlays - non-fatal
            }
        }

        // Step 0B: Load project rulebook to get builtin configuration
        self.rulebook = Some(
            rulebook::Rulebook::load_with_conventions(
                &self.paths.rulebook,
                &self.paths.signals,
                &self.paths.actions,
            )
            .await?,
        );
        info!("Project rulebook loaded with convention-based discovery");

        // Step 0C: Initialize Watchdog if enabled in rulebook
        // Watchdog uses directory-based configuration from .cupcake/watchdog/
        // with fallback to ~/.config/cupcake/watchdog/ for global settings
        if let Some(ref rulebook) = self.rulebook {
            if rulebook.watchdog.enabled {
                // Get watchdog directories for config/prompt loading
                let project_watchdog_dir = self.paths.project_watchdog_dir();
                let global_watchdog_dir = self.paths.global_watchdog_dir();

                debug!(
                    "Watchdog directories: project={:?}, global={:?}",
                    project_watchdog_dir, global_watchdog_dir
                );

                // Use from_directories() for full directory-based config loading
                match crate::watchdog::Watchdog::from_directories(
                    project_watchdog_dir.as_deref(),
                    global_watchdog_dir.as_deref(),
                ) {
                    Ok(watchdog) => {
                        if watchdog.is_enabled() {
                            info!("Watchdog initialized and ready");
                            self.watchdog = Some(watchdog);
                        } else {
                            warn!("Watchdog enabled in config but failed to initialize backend");
                        }
                    }
                    Err(e) => {
                        warn!(
                            "Failed to initialize Watchdog: {e}. Continuing without it. \
                            Check that your API key environment variable is set and the watchdog configuration is valid."
                        );
                    }
                }
            }
        }

        // Get list of enabled builtins for filtering
        let enabled_builtins = self
            .rulebook
            .as_ref()
            .map(|g| g.builtins.enabled_builtins())
            .unwrap_or_default();

        if !enabled_builtins.is_empty() {
            info!("Enabled builtins: {:?}", enabled_builtins);
        }

        // Determine harness-specific policies subdirectory
        let harness_subdir = match self.config.harness {
            crate::harness::types::HarnessType::ClaudeCode => "claude",
            crate::harness::types::HarnessType::Cursor => "cursor",
            crate::harness::types::HarnessType::Factory => "factory",
            crate::harness::types::HarnessType::OpenCode => "opencode",
        };
        let harness_policies_dir = self.paths.policies.join(harness_subdir);
        info!(
            "Scanning harness-specific policies for: {:?} at {:?}",
            self.config.harness, harness_policies_dir
        );

        // Step 1: Scan for .rego files in harness-specific directory with builtin filtering
        let policy_files =
            scanner::scan_policies_with_filter(&harness_policies_dir, &enabled_builtins).await?;
        info!(
            "Found {} policy files in {} harness directory",
            policy_files.len(),
            harness_subdir
        );

        // Step 1b: Scan system directory at cupcake root for shared system entrypoint
        let system_dir = self.paths.cupcake_dir.join("system");
        let system_files = if system_dir.exists() && system_dir.is_dir() {
            info!("Scanning system directory: {:?}", system_dir);
            scanner::scan_policies(&system_dir)
                .await
                .unwrap_or_else(|e| {
                    warn!("Failed to scan system directory: {}", e);
                    Vec::new()
                })
        } else {
            debug!("No system directory found at {:?}", system_dir);
            Vec::new()
        };
        info!("Found {} system policy files", system_files.len());

        // Step 2: Parse selectors and build policy units from harness policies
        for path in policy_files {
            match self.parse_policy(&path).await {
                Ok(unit) => {
                    info!(
                        "Successfully parsed policy: {} from {:?}",
                        unit.package_name, path
                    );
                    self.policies.push(unit);
                }
                Err(e) => {
                    // Fail loudly but don't crash - log and skip bad policies
                    error!("Failed to parse policy at {:?}: {}", path, e);
                }
            }
        }

        // Step 2b: Parse system policies
        for path in system_files {
            match self.parse_policy(&path).await {
                Ok(unit) => {
                    info!(
                        "Successfully parsed system policy: {} from {:?}",
                        unit.package_name, path
                    );
                    self.policies.push(unit);
                }
                Err(e) => {
                    error!("Failed to parse system policy at {:?}: {}", path, e);
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

        // Step 4: Compile unified WASM module with OPA path from CLI
        // Pass cupcake_dir for helpers resolution at root level
        let wasm_bytes = compiler::compile_policies_with_namespace(
            &self.policies,
            "cupcake.system",
            self.config.opa_path.clone(),
            Some(&self.paths.cupcake_dir),
        )
        .await?;
        info!(
            "Successfully compiled unified WASM module ({} bytes)",
            wasm_bytes.len()
        );
        self.wasm_module = Some(wasm_bytes.clone());

        // Step 5: Initialize WASM runtime with memory config from CLI
        self.wasm_runtime = Some(wasm_runtime::WasmRuntime::new_with_config(
            &wasm_bytes,
            "cupcake.system",
            self.config.wasm_max_memory,
        )?);
        info!("WASM runtime initialized");

        // Step 6: Try to initialize trust verifier (optional - don't fail if not enabled)
        self.initialize_trust_system().await;

        // Step 7: Dump routing diagnostics if debug mode enabled via CLI flag
        // This happens after ALL initialization (including global) is complete
        if self.config.debug_routing {
            if let Err(e) = self.dump_routing_diagnostics() {
                // Don't fail initialization, just warn
                warn!("Failed to dump routing diagnostics: {}", e);
            }
        }

        info!("Engine initialization complete");
        Ok(())
    }

    /// Initialize global configuration (policies, rulebook, WASM)
    async fn initialize_global(&mut self) -> Result<()> {
        info!("Initializing global configuration...");

        // Verify we have global paths
        let global_policies_path = self
            .paths
            .global_policies
            .as_ref()
            .context("Global policies path not set")?;
        let global_rulebook_path = self
            .paths
            .global_rulebook
            .as_ref()
            .context("Global rulebook path not set")?;

        // Load global rulebook
        if global_rulebook_path.exists() {
            self.global_rulebook = Some(
                rulebook::Rulebook::load_with_conventions(
                    global_rulebook_path,
                    self.paths.global_signals.as_ref().unwrap(),
                    self.paths.global_actions.as_ref().unwrap(),
                )
                .await?,
            );
            info!("Global rulebook loaded");
        }

        // Get global enabled builtins
        let global_enabled_builtins = self
            .global_rulebook
            .as_ref()
            .map(|g| g.builtins.enabled_builtins())
            .unwrap_or_default();

        // Determine harness-specific global policies subdirectory
        let harness_subdir = match self.config.harness {
            crate::harness::types::HarnessType::ClaudeCode => "claude",
            crate::harness::types::HarnessType::Cursor => "cursor",
            crate::harness::types::HarnessType::Factory => "factory",
            crate::harness::types::HarnessType::OpenCode => "opencode",
        };
        let harness_global_policies_dir = global_policies_path.join(harness_subdir);

        // Scan for global policies in harness-specific directory
        if harness_global_policies_dir.exists() {
            info!(
                "Scanning global harness-specific policies for: {:?} at {:?}",
                self.config.harness, harness_global_policies_dir
            );
            let global_policy_files = scanner::scan_policies_with_filter(
                &harness_global_policies_dir,
                &global_enabled_builtins,
            )
            .await?;
            info!(
                "Found {} global policy files in {} harness directory",
                global_policy_files.len(),
                harness_subdir
            );

            // Parse global policies
            for path in global_policy_files {
                match self.parse_policy(&path).await {
                    Ok(mut unit) => {
                        // Transform package name to global namespace
                        if !unit.package_name.starts_with("cupcake.global.") {
                            // Replace "cupcake.policies" with "cupcake.global.policies"
                            unit.package_name = unit
                                .package_name
                                .replace("cupcake.policies", "cupcake.global.policies")
                                .replace("cupcake.system", "cupcake.global.system");
                        }
                        info!(
                            "Successfully parsed global policy: {} from {:?}",
                            unit.package_name, path
                        );
                        self.global_policies.push(unit);
                    }
                    Err(e) => {
                        error!("Failed to parse global policy at {:?}: {}", path, e);
                    }
                }
            }

            if !self.global_policies.is_empty() {
                // Check if we have non-system policies (OPA panics with only system policies)
                info!(
                    "Global policies found: {:?}",
                    self.global_policies
                        .iter()
                        .map(|p| &p.package_name)
                        .collect::<Vec<_>>()
                );
                let non_system_count = self
                    .global_policies
                    .iter()
                    .filter(|p| !p.package_name.ends_with(".system"))
                    .count();

                if non_system_count > 0 {
                    // Build global routing map
                    self.build_global_routing_map();
                    info!(
                        "Built global routing map with {} entries",
                        self.global_routing_map.len()
                    );

                    info!(
                        "Found {} global policies ({} non-system) - compiling global WASM module",
                        self.global_policies.len(),
                        non_system_count
                    );

                    // Compile global policies to WASM with OPA path from CLI
                    // Pass global_root for helpers resolution at root level
                    let global_wasm_bytes = compiler::compile_policies_with_namespace(
                        &self.global_policies,
                        "cupcake.global.system",
                        self.config.opa_path.clone(),
                        self.paths.global_root.as_deref(),
                    )
                    .await?;
                    info!(
                        "Successfully compiled global WASM module ({} bytes)",
                        global_wasm_bytes.len()
                    );
                    self.global_wasm_module = Some(global_wasm_bytes.clone());

                    // Initialize global WASM runtime with global namespace and memory config
                    self.global_wasm_runtime = Some(wasm_runtime::WasmRuntime::new_with_config(
                        &global_wasm_bytes,
                        "cupcake.global.system",
                        self.config.wasm_max_memory,
                    )?);
                    info!("Global WASM runtime initialized with namespace: cupcake.global.system");
                } else {
                    info!("Only system policies found in global config - skipping global WASM compilation");
                }
            }
        }

        info!("Global configuration initialization complete");
        Ok(())
    }

    /// Initialize catalog overlays from .cupcake/catalog/
    #[cfg(feature = "catalog")]
    async fn initialize_catalog_overlays(&mut self) -> Result<()> {
        info!("Initializing catalog overlays...");

        // Discover installed catalog overlays
        let mut overlays = catalog_overlay::discover_catalog_overlays(
            &self.paths.cupcake_dir,
            self.config.harness,
        )
        .await?;

        if overlays.is_empty() {
            debug!("No catalog overlays found");
            return Ok(());
        }

        info!("Found {} catalog overlays to initialize", overlays.len());

        // Scan and parse policies for each overlay
        catalog_overlay::scan_catalog_policies(&mut overlays, self.config.opa_path.clone()).await?;

        // Compile each overlay and create runtime
        for overlay in overlays {
            if overlay.policies.is_empty() {
                debug!(
                    "Catalog overlay {} has no policies for {} harness - skipping",
                    overlay.name,
                    match self.config.harness {
                        crate::harness::types::HarnessType::ClaudeCode => "claude",
                        crate::harness::types::HarnessType::Cursor => "cursor",
                        crate::harness::types::HarnessType::Factory => "factory",
                        crate::harness::types::HarnessType::OpenCode => "opencode",
                    }
                );
                continue;
            }

            // Check for non-system policies
            let non_system_count = overlay
                .policies
                .iter()
                .filter(|p| !p.package_name.ends_with(".system"))
                .count();

            if non_system_count == 0 {
                debug!(
                    "Catalog overlay {} has only system policies - skipping WASM compilation",
                    overlay.name
                );
                continue;
            }

            info!(
                "Compiling catalog overlay {} ({} policies, {} non-system)",
                overlay.name,
                overlay.policies.len(),
                non_system_count
            );

            // Build routing map for this overlay
            let mut routing_map = HashMap::new();
            Self::build_routing_map_generic(&overlay.policies, &mut routing_map, &overlay.name);
            info!(
                "Built routing map for catalog overlay {} with {} entries",
                overlay.name,
                routing_map.len()
            );

            // Compile to WASM
            match catalog_overlay::compile_catalog_overlay(&overlay, self.config.opa_path.clone())
                .await
            {
                Ok(wasm_bytes) => {
                    info!(
                        "Compiled catalog overlay {} to {} bytes",
                        overlay.name,
                        wasm_bytes.len()
                    );

                    // Create WASM runtime
                    let system_namespace = format!("{}.system", overlay.namespace);
                    match wasm_runtime::WasmRuntime::new_with_config(
                        &wasm_bytes,
                        &system_namespace,
                        self.config.wasm_max_memory,
                    ) {
                        Ok(runtime) => {
                            info!(
                                "Created WASM runtime for catalog overlay {} (namespace: {})",
                                overlay.name, system_namespace
                            );

                            // Store everything
                            self.catalog_overlays.push(overlay);
                            self.catalog_routing_maps.push(routing_map);
                            self.catalog_wasm_runtimes.push(runtime);
                        }
                        Err(e) => {
                            warn!(
                                "Failed to create WASM runtime for catalog overlay {}: {}",
                                overlay.name, e
                            );
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to compile catalog overlay {}: {}", overlay.name, e);
                }
            }
        }

        info!(
            "Catalog overlay initialization complete: {} overlays active",
            self.catalog_overlays.len()
        );
        Ok(())
    }

    /// Build routing map for global policies
    fn build_global_routing_map(&mut self) {
        Self::build_routing_map_generic(
            &self.global_policies,
            &mut self.global_routing_map,
            "global",
        );
    }

    /// Parse a single policy file to extract selector and metadata
    async fn parse_policy(&self, path: &Path) -> Result<PolicyUnit> {
        let content = tokio::fs::read_to_string(path)
            .await
            .context("Failed to read policy file")?;

        // Extract package name
        let package_name =
            metadata::extract_package_name(&content).context("Failed to extract package name")?;

        // Parse OPA metadata
        let policy_metadata =
            metadata::parse_metadata(&content).context("Failed to parse OPA metadata")?;

        // Extract routing directive - system policies don't need routing
        let routing = if let Some(ref meta) = policy_metadata {
            if let Some(ref routing_directive) = meta.custom.routing {
                // Validate the routing directive
                metadata::validate_routing_directive(routing_directive, &package_name)
                    .with_context(|| {
                        format!("Invalid routing directive in policy {package_name}")
                    })?;
                routing_directive.clone()
            } else if package_name.starts_with("cupcake.system")
                || package_name.starts_with("cupcake.global.system")
            {
                // System packages don't need routing - they're entrypoints or helpers
                // This covers cupcake.system, cupcake.system.commands, cupcake.global.system, etc.
                debug!(
                    "System package {} has no routing directive (this is expected)",
                    package_name
                );
                RoutingDirective::default()
            } else {
                warn!(
                    "Policy {} has no routing directive in metadata - will not be routed",
                    package_name
                );
                return Err(anyhow::anyhow!("Policy missing routing directive"));
            }
        } else {
            // System packages are allowed to have no metadata
            if package_name.starts_with("cupcake.system")
                || package_name.starts_with("cupcake.global.system")
            {
                debug!(
                    "System package {} has no metadata block (this is allowed)",
                    package_name
                );
                RoutingDirective::default()
            } else {
                warn!(
                    "Policy {} has no metadata block - will not be routed",
                    package_name
                );
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
        map_type: &str,
    ) {
        routing_map.clear();

        for policy in policies {
            // Create routing keys for this policy from metadata
            let routing_keys = routing::create_routing_key_from_metadata(&policy.routing);

            // Add policy to the routing map for each key
            for key in routing_keys {
                routing_map
                    .entry(key.clone())
                    .or_default()
                    .push(policy.clone());
                debug!(
                    "Added {} policy {} to routing key: {}",
                    map_type, policy.package_name, key
                );
            }
        }

        // Handle wildcard routes - also add them to specific tool lookups
        // This allows "PreToolUse:Bash" to find both specific and wildcard policies
        let wildcard_keys: Vec<String> = routing_map
            .keys()
            .filter(|k| k.ends_with(":*"))
            .cloned()
            .collect();

        for wildcard_key in wildcard_keys {
            if let Some(wildcard_policies) = routing_map.get(&wildcard_key).cloned() {
                let event_prefix = wildcard_key.strip_suffix(":*").unwrap();

                // Find all specific tool keys for this event
                let specific_keys: Vec<String> = routing_map
                    .keys()
                    .filter(|k| k.starts_with(&format!("{event_prefix}:")) && !k.ends_with(":*"))
                    .cloned()
                    .collect();

                // Add wildcard policies to each specific tool key
                for specific_key in specific_keys {
                    routing_map
                        .entry(specific_key)
                        .or_default()
                        .extend(wildcard_policies.clone());
                }
            }
        }

        // Handle event-only policies for tool events (PreToolUse, PostToolUse)
        // These events ALWAYS have tools, so event-only policies are effectively wildcards
        const TOOL_EVENTS: &[&str] = &["PreToolUse", "PostToolUse"];

        for tool_event in TOOL_EVENTS {
            if let Some(event_only_policies) = routing_map.get(*tool_event).cloned() {
                // Find all specific tool keys for this event
                let specific_keys: Vec<String> = routing_map
                    .keys()
                    .filter(|k| k.starts_with(&format!("{tool_event}:")) && !k.ends_with(":*"))
                    .cloned()
                    .collect();

                // Add event-only policies to each specific tool key (they act as wildcards)
                for specific_key in specific_keys {
                    routing_map
                        .entry(specific_key.clone())
                        .and_modify(|policies| {
                            // Add event-only policies if not already present
                            for event_policy in &event_only_policies {
                                if !policies
                                    .iter()
                                    .any(|p| p.package_name == event_policy.package_name)
                                {
                                    policies.push(event_policy.clone());
                                    debug!(
                                        "Added {} wildcard policy {} to specific key: {}",
                                        map_type, event_policy.package_name, specific_key
                                    );
                                }
                            }
                        });
                }
            }
        }

        // Log the routing map for verification
        for (key, policies) in routing_map {
            debug!(
                "Route '{}' -> {} policies: [{}]",
                key,
                policies.len(),
                policies
                    .iter()
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

    /// Get the telemetry configuration from the rulebook
    pub fn telemetry_config(&self) -> Option<&TelemetryConfig> {
        self.rulebook.as_ref().map(|rb| &rb.telemetry)
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
        let mut result: Vec<&PolicyUnit> = self
            .routing_map
            .get(&key)
            .map(|policies| policies.iter().collect())
            .unwrap_or_default();

        // ALSO check for event-only policies when there's a tool
        // This handles the case where ONLY wildcard policies exist (nothing to duplicate into)
        if tool_name.is_some() {
            let wildcard_key = event_name.to_string();
            if let Some(wildcard_policies) = self.routing_map.get(&wildcard_key) {
                // Only add if not already present (avoid duplicates from build-time merging)
                for policy in wildcard_policies {
                    if !result.iter().any(|p| p.package_name == policy.package_name) {
                        result.push(policy);
                    }
                }
            }
        }

        // Record matched policies
        let current_span = tracing::Span::current();
        current_span.record("matched_count", result.len());
        if !result.is_empty() {
            let policy_names: Vec<&str> = result.iter().map(|p| p.package_name.as_str()).collect();
            current_span.record("policy_names", format!("{policy_names:?}").as_str());
        }

        trace!(
            duration_us = start.elapsed().as_micros(),
            matched = result.len(),
            "Policy routing complete"
        );

        result
    }

    /// Route event using a specific routing map (used for catalog overlays)
    #[cfg(feature = "catalog")]
    fn route_with_map<'a>(
        routing_map: &'a HashMap<String, Vec<PolicyUnit>>,
        event_name: &str,
        tool_name: Option<&str>,
    ) -> Vec<&'a PolicyUnit> {
        let key = routing::create_event_key(event_name, tool_name);

        // First try the specific key
        let mut result: Vec<&PolicyUnit> = routing_map
            .get(&key)
            .map(|policies| policies.iter().collect())
            .unwrap_or_default();

        // ALSO check for event-only policies when there's a tool
        if tool_name.is_some() {
            let wildcard_key = event_name.to_string();
            if let Some(wildcard_policies) = routing_map.get(&wildcard_key) {
                for policy in wildcard_policies {
                    if !result.iter().any(|p| p.package_name == policy.package_name) {
                        result.push(policy);
                    }
                }
            }
        }

        result
    }

    /// Evaluate policies for a hook event.
    ///
    /// This is the main public API for policy evaluation.
    #[instrument(
        name = "evaluate",
        skip(self, input, telemetry),
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
    pub async fn evaluate(
        &self,
        input: &Value,
        mut telemetry: Option<&mut TelemetryContext>,
    ) -> Result<decision::FinalDecision> {
        let eval_start = Instant::now();

        // STEP 0: ALWAYS PREPROCESS - Self-Defending Engine Architecture
        // The Engine never accepts raw, unpreprocessed input. This provides
        // defense-in-depth by ensuring ALL paths (CLI, FFI, tests) are protected
        // from TOB-3 (spacing bypass) and TOB-4 (symlink bypass) attacks.
        //
        // This preprocessing is:
        // - Automatic: No caller action required
        // - Universal: Protects all policies (builtin and custom)
        // - Idempotent: Safe to call multiple times (e.g., if CLI also preprocesses)
        // - Fast: ~30-100μs overhead (<0.1% of total evaluation time)
        //
        // See TOB4_IMPLEMENTATION_LOG.md Phase 4 for architectural rationale.
        //
        // IMPORTANT: This clone is intentional and required for security.
        // DO NOT OPTIMIZE: The preprocessing defends against adversarial input attacks
        // (TOB findings) and must never modify the original input. The overhead is
        // <0.1% of evaluation time. With default config, preprocessing modifies:
        // - Bash commands: if whitespace normalization needed
        // - File operations: always (adds resolved_file_path, is_symlink fields)
        // - Other events: no-op but clone still required for uniform security model
        //
        // NOTE: Copy-on-write (CoW) optimization considered and rejected:
        // - Clone cost: ~10-50μs even for large files, <0.1% of 10-100ms eval time
        // - CoW complexity: requires custom types, mutation tracking, architectural changes
        // - Security value: immutable input pattern prevents accidental modifications
        // - Tradeoff: Security and simplicity take priority over micro-optimization
        let mut safe_input = input.clone();
        let preprocess_config = crate::preprocessing::PreprocessConfig::default();
        crate::preprocessing::preprocess_input(
            &mut safe_input,
            &preprocess_config,
            self.config.harness,
        );
        trace!("Input preprocessing completed (self-defending engine)");

        // STEP 1: Extract event info from SAFE input for routing
        // Try both camelCase and snake_case for compatibility
        let event_name = safe_input
            .get("hookEventName")
            .or_else(|| safe_input.get("hook_event_name"))
            .and_then(|v| v.as_str())
            .context("Missing hookEventName/hook_event_name in input")?;

        let tool_name = safe_input.get("tool_name").and_then(|v| v.as_str());

        // Set span fields
        let current_span = tracing::Span::current();
        current_span.record("event_name", event_name);
        current_span.record("tool_name", tool_name);
        if let Some(session_id) = trace::extract_session_id(&safe_input) {
            current_span.record("session_id", session_id.as_str());
        }

        info!("Evaluating event: {} tool: {:?}", event_name, tool_name);

        // Create Executor early - used for both global and project evaluation
        // The Executor handles all OS/IO interactions (signals, actions)
        let exec = executor::Executor {
            rulebook: self.rulebook.as_ref(),
            global_rulebook: self.global_rulebook.as_ref(),
            trust_verifier: self.trust_verifier.as_ref(),
            watchdog: self.watchdog.as_ref(),
            working_dir: &self.paths.root,
        };

        // PHASE 1: Evaluate global policies first (if they exist)
        if self.global_wasm_runtime.is_some() {
            debug!("Phase 1: Evaluating global policies");
            let capture_telemetry = telemetry.is_some();
            let (global_decision, global_decision_set, global_signal_executions) = self
                .evaluate_global(&safe_input, event_name, tool_name, &exec, capture_telemetry)
                .await?;

            // Record global evaluation in telemetry
            if let Some(ref mut ctx) = telemetry {
                let phase = ctx.start_phase("global");
                phase.evaluation_mut().record_routing(
                    true,
                    &self.global_routing_map.keys().cloned().collect::<Vec<_>>(),
                );
                // Record signal executions from global evaluation
                for signal in global_signal_executions {
                    phase.record_signal(signal);
                }
                phase
                    .evaluation_mut()
                    .record_wasm_result(&global_decision_set);
                phase
                    .evaluation_mut()
                    .record_final_decision(&global_decision);
            }

            // Early termination on global blocking decisions
            match &global_decision {
                decision::FinalDecision::Halt { reason, .. } => {
                    info!("Global policy HALT - immediate termination: {}", reason);
                    // Record exit reason in telemetry
                    if let Some(ref mut ctx) = telemetry {
                        if let Some(phase) = ctx.current_phase_mut() {
                            phase
                                .evaluation_mut()
                                .record_exit(format!("Global halt: {reason}"));
                            phase.finalize();
                        }
                    }
                    // Execute global actions before returning
                    exec.execute_global_actions(&global_decision, &global_decision_set)
                        .await;
                    current_span.record("final_decision", "GlobalHalt");
                    current_span.record("duration_ms", eval_start.elapsed().as_millis());
                    return Ok(global_decision);
                }
                decision::FinalDecision::Deny { reason, .. } => {
                    info!("Global policy DENY - immediate termination: {}", reason);
                    // Record exit reason in telemetry
                    if let Some(ref mut ctx) = telemetry {
                        if let Some(phase) = ctx.current_phase_mut() {
                            phase
                                .evaluation_mut()
                                .record_exit(format!("Global deny: {reason}"));
                            phase.finalize();
                        }
                    }
                    // Execute global actions before returning
                    exec.execute_global_actions(&global_decision, &global_decision_set)
                        .await;
                    current_span.record("final_decision", "GlobalDeny");
                    current_span.record("duration_ms", eval_start.elapsed().as_millis());
                    return Ok(global_decision);
                }
                decision::FinalDecision::Block { reason, .. } => {
                    info!("Global policy BLOCK - immediate termination: {}", reason);
                    // Record exit reason in telemetry
                    if let Some(ref mut ctx) = telemetry {
                        if let Some(phase) = ctx.current_phase_mut() {
                            phase
                                .evaluation_mut()
                                .record_exit(format!("Global block: {reason}"));
                            phase.finalize();
                        }
                    }
                    // Execute global actions before returning
                    exec.execute_global_actions(&global_decision, &global_decision_set)
                        .await;
                    current_span.record("final_decision", "GlobalBlock");
                    current_span.record("duration_ms", eval_start.elapsed().as_millis());
                    return Ok(global_decision);
                }
                _ => {
                    debug!("Global policies did not halt/deny/block - proceeding to catalog evaluation");
                    // Finalize global phase before continuing
                    if let Some(ref mut ctx) = telemetry {
                        if let Some(phase) = ctx.current_phase_mut() {
                            phase.finalize();
                        }
                    }
                    // TODO: In the future, preserve Ask/Allow/Context for merging with project decisions
                }
            }
        }

        // PHASE 1.5: Evaluate catalog overlays (between global and project)
        #[cfg(feature = "catalog")]
        if !self.catalog_wasm_runtimes.is_empty() {
            debug!(
                "Phase 1.5: Evaluating {} catalog overlays",
                self.catalog_wasm_runtimes.len()
            );

            for (idx, (overlay, runtime)) in self
                .catalog_overlays
                .iter()
                .zip(self.catalog_wasm_runtimes.iter())
                .enumerate()
            {
                debug!(
                    "Evaluating catalog overlay {}/{}: {}",
                    idx + 1,
                    self.catalog_overlays.len(),
                    overlay.name
                );

                // Route through this overlay's routing map
                let routing_map = &self.catalog_routing_maps[idx];
                let catalog_matched = Self::route_with_map(routing_map, event_name, tool_name);

                if catalog_matched.is_empty() {
                    debug!("No policies matched in catalog overlay {}", overlay.name);
                    continue;
                }

                info!(
                    "Found {} matching policies in catalog overlay {}",
                    catalog_matched.len(),
                    overlay.name
                );

                // Evaluate using this overlay's WASM runtime
                let catalog_decision_set = runtime.query_decision_set(&safe_input)?;
                let catalog_decision =
                    synthesis::SynthesisEngine::synthesize(&catalog_decision_set)?;

                info!(
                    "Catalog overlay {} decision: {:?}",
                    overlay.name, catalog_decision
                );

                // Early termination on blocking decisions
                match &catalog_decision {
                    decision::FinalDecision::Halt { reason, .. } => {
                        info!(
                            "Catalog overlay {} HALT - immediate termination: {}",
                            overlay.name, reason
                        );
                        current_span.record("final_decision", "CatalogHalt");
                        current_span.record("duration_ms", eval_start.elapsed().as_millis());
                        return Ok(catalog_decision);
                    }
                    decision::FinalDecision::Deny { reason, .. } => {
                        info!(
                            "Catalog overlay {} DENY - immediate termination: {}",
                            overlay.name, reason
                        );
                        current_span.record("final_decision", "CatalogDeny");
                        current_span.record("duration_ms", eval_start.elapsed().as_millis());
                        return Ok(catalog_decision);
                    }
                    decision::FinalDecision::Block { reason, .. } => {
                        info!(
                            "Catalog overlay {} BLOCK - immediate termination: {}",
                            overlay.name, reason
                        );
                        current_span.record("final_decision", "CatalogBlock");
                        current_span.record("duration_ms", eval_start.elapsed().as_millis());
                        return Ok(catalog_decision);
                    }
                    _ => {
                        debug!(
                            "Catalog overlay {} did not halt/deny/block - continuing",
                            overlay.name
                        );
                    }
                }
            }
        }

        // PHASE 2: Evaluate project policies
        debug!("Phase 2: Evaluating project policies");

        // Start project evaluation phase
        if let Some(ref mut ctx) = telemetry {
            ctx.start_phase("project");
        }

        // Step 1: Route - find relevant policies (collect owned PolicyUnits)
        let matched_policies: Vec<PolicyUnit> = self
            .route_event(event_name, tool_name)
            .into_iter()
            .cloned()
            .collect();

        let policy_names: Vec<String> = matched_policies
            .iter()
            .map(|p| p.package_name.clone())
            .collect();

        // Record routing in telemetry
        if let Some(ref mut ctx) = telemetry {
            if let Some(phase) = ctx.current_phase_mut() {
                phase
                    .evaluation_mut()
                    .record_routing(!matched_policies.is_empty(), &policy_names);
            }
        }

        if matched_policies.is_empty() {
            info!("No policies matched for this event - allowing");
            // Record early exit in telemetry
            if let Some(ref mut ctx) = telemetry {
                if let Some(phase) = ctx.current_phase_mut() {
                    phase
                        .evaluation_mut()
                        .record_exit("No policies matched - implicit allow");
                    phase
                        .evaluation_mut()
                        .record_final_decision(&decision::FinalDecision::Allow { context: vec![] });
                    phase.finalize();
                }
            }
            current_span.record("matched_policy_count", 0);
            current_span.record("final_decision", "Allow");
            current_span.record("duration_ms", eval_start.elapsed().as_millis());
            return Ok(decision::FinalDecision::Allow { context: vec![] });
        }

        current_span.record("matched_policy_count", matched_policies.len());
        info!("Found {} matching policies", matched_policies.len());

        // Step 2: Gather signals using the Executor (created earlier for global evaluation)
        // Gather signals - collect telemetry if enabled
        let (enriched_input, signal_executions) = if telemetry.is_some() {
            let mut signal_telemetry = SignalTelemetry::new();
            let result = exec
                .gather_signals(&safe_input, &matched_policies, Some(&mut signal_telemetry))
                .await?;
            (result, signal_telemetry.signals)
        } else {
            let result = exec
                .gather_signals(&safe_input, &matched_policies, None)
                .await?;
            (result, Vec::new())
        };

        // Record signal executions in telemetry
        if let Some(ref mut ctx) = telemetry {
            if let Some(phase) = ctx.current_phase_mut() {
                for signal in signal_executions {
                    phase.record_signal(signal);
                }
            }
        }

        // Step 3: Evaluate using single aggregation entrypoint with enriched input
        debug!("About to evaluate decision set with enriched input");
        let decision_set = self.evaluate_decision_set(&enriched_input).await?;

        // Record WASM results in telemetry
        if let Some(ref mut ctx) = telemetry {
            if let Some(phase) = ctx.current_phase_mut() {
                phase.evaluation_mut().record_wasm_result(&decision_set);
            }
        }

        // Step 4: Apply Intelligence Layer synthesis
        let final_decision = synthesis::SynthesisEngine::synthesize(&decision_set)?;

        info!("Synthesized final decision: {:?}", final_decision);

        // Record final decision in telemetry
        if let Some(ref mut ctx) = telemetry {
            if let Some(phase) = ctx.current_phase_mut() {
                phase
                    .evaluation_mut()
                    .record_final_decision(&final_decision);
                phase.finalize();
            }
        }

        // Step 5: Execute actions based on decision (async, non-blocking)
        exec.execute_actions(&final_decision, &decision_set).await;

        // Record final decision type and duration
        let duration = eval_start.elapsed();
        let current_span = tracing::Span::current();
        current_span.record("final_decision", format!("{final_decision:?}").as_str());
        current_span.record("duration_ms", duration.as_millis());

        trace!(
            decision = ?final_decision,
            duration_ms = duration.as_millis(),
            "Evaluation complete"
        );

        Ok(final_decision)
    }

    /// Evaluate global policies
    ///
    /// Returns (decision, decision_set, signal_executions) where signal_executions
    /// is populated only when capture_telemetry is true.
    async fn evaluate_global(
        &self,
        input: &Value,
        event_name: &str,
        tool_name: Option<&str>,
        exec: &executor::Executor<'_>,
        capture_telemetry: bool,
    ) -> Result<(
        decision::FinalDecision,
        decision::DecisionSet,
        Vec<crate::telemetry::span::SignalExecution>,
    )> {
        // Route through global policies
        let global_matched: Vec<PolicyUnit> = self
            .route_global_event(event_name, tool_name)
            .into_iter()
            .cloned()
            .collect();

        if global_matched.is_empty() {
            debug!("No global policies matched for this event");
            return Ok((
                decision::FinalDecision::Allow { context: vec![] },
                decision::DecisionSet::default(),
                Vec::new(),
            ));
        }

        info!("Found {} matching global policies", global_matched.len());

        // Gather signals for global policies using Executor - collect telemetry if enabled
        let (enriched_input, signal_executions) = if capture_telemetry {
            let mut signal_telemetry = SignalTelemetry::new();
            let result = exec
                .gather_global_signals(input, &global_matched, Some(&mut signal_telemetry))
                .await?;
            (result, signal_telemetry.signals)
        } else {
            let result = exec
                .gather_global_signals(input, &global_matched, None)
                .await?;
            (result, Vec::new())
        };

        // Evaluate using global WASM runtime
        let global_runtime = self
            .global_wasm_runtime
            .as_ref()
            .context("Global WASM runtime not initialized")?;

        let global_decision_set = global_runtime.query_decision_set(&enriched_input)?;
        debug!(
            "Global DecisionSet: {} total decisions",
            global_decision_set.decision_count()
        );

        // Synthesize global decision
        let global_decision = synthesis::SynthesisEngine::synthesize(&global_decision_set)?;
        info!("Global policy decision: {:?}", global_decision);

        Ok((global_decision, global_decision_set, signal_executions))
    }

    /// Route event through global policies
    fn route_global_event(&self, event_name: &str, tool_name: Option<&str>) -> Vec<&PolicyUnit> {
        let key = routing::create_event_key(event_name, tool_name);

        // First try the specific key
        let mut result: Vec<&PolicyUnit> = self
            .global_routing_map
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

    /// Evaluate using the Hybrid Model single aggregation entrypoint
    async fn evaluate_decision_set(&self, input: &Value) -> Result<decision::DecisionSet> {
        let runtime = self
            .wasm_runtime
            .as_ref()
            .context("WASM runtime not initialized")?;

        debug!("Evaluating using single cupcake.system.evaluate entrypoint");

        // Query the single aggregation entrypoint
        let decision_set = runtime.query_decision_set(input)?;

        debug!(
            "Raw DecisionSet from WASM: {} total decisions",
            decision_set.decision_count()
        );
        debug!(
            "DecisionSet summary: {}",
            synthesis::SynthesisEngine::summarize_decision_set(&decision_set)
        );

        Ok(decision_set)
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
                    info!(
                        "Trust mode DISABLED by user - scripts will execute without verification"
                    );
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
