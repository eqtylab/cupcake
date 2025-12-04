//! The Cupcake Engine - Core orchestration module.
//!
//! Provides metadata-driven policy discovery, O(1) routing, WASM evaluation,
//! and decision synthesis.

use anyhow::{anyhow, Context, Result};
use once_cell::sync::Lazy;
// use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Instant;
use tracing::{debug, error, info, instrument, trace, warn};

use crate::debug::DebugCapture;

/// Detects the appropriate shell command for the current platform
///
/// On Windows, attempts to find Git Bash for shell script compatibility:
/// 1. Check standard Git for Windows installation path (C:\Program Files\Git\bin\bash.exe)
/// 2. Check alternative 32-bit path (C:\Program Files (x86)\Git\bin\bash.exe)
/// 3. Try bash.exe in PATH (may be available from other installations)
///
/// On Unix systems, uses 'sh' which is always available.
fn find_shell_command() -> &'static str {
    if cfg!(windows) {
        // Git Bash on GitHub Actions and most Windows dev machines
        if std::path::Path::new(r"C:\Program Files\Git\bin\bash.exe").exists() {
            debug!("Found Git Bash at standard 64-bit location");
            return r"C:\Program Files\Git\bin\bash.exe";
        }

        // Check 32-bit Program Files location
        if std::path::Path::new(r"C:\Program Files (x86)\Git\bin\bash.exe").exists() {
            debug!("Found Git Bash at 32-bit location");
            return r"C:\Program Files (x86)\Git\bin\bash.exe";
        }

        // Try bash.exe from PATH (will work if Git Bash is in PATH)
        debug!("No Git Bash found at standard locations, trying bash.exe from PATH");
        "bash.exe"
    } else {
        "sh"
    }
}

/// Cached shell command determined at first use
static SHELL_COMMAND: Lazy<&'static str> = Lazy::new(find_shell_command);

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
    /// Rulebook file (.cupcake/rulebook.yml)
    pub rulebook: PathBuf,

    // Global configuration paths (optional - may not exist)
    /// Global config root directory
    pub global_root: Option<PathBuf>,
    /// Global policies directory
    pub global_policies: Option<PathBuf>,
    /// Global signals directory
    pub global_signals: Option<PathBuf>,
    /// Global actions directory
    pub global_actions: Option<PathBuf>,
    /// Global rulebook file
    pub global_rulebook: Option<PathBuf>,
}

impl ProjectPaths {
    /// Resolve project paths using convention over configuration
    /// Accepts either project root OR .cupcake/ directory for flexibility
    /// Also discovers global configuration if present
    pub fn resolve(input_path: impl AsRef<Path>) -> Result<Self> {
        Self::resolve_with_config(input_path, None)
    }

    /// Resolve project paths with optional global config override
    /// Used by Engine::new_with_config() to apply CLI --global-config flag
    pub fn resolve_with_config(
        input_path: impl AsRef<Path>,
        global_config_override: Option<PathBuf>,
    ) -> Result<Self> {
        let input = input_path.as_ref().to_path_buf();

        let (root, cupcake_dir) = if input.file_name() == Some(std::ffi::OsStr::new(".cupcake")) {
            // Input is .cupcake/ directory
            let root = input
                .parent()
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

        // Discover global configuration with optional CLI override
        let global_config =
            global_config::GlobalPaths::discover_with_override(global_config_override)
                .unwrap_or_else(|e| {
                    debug!("Failed to discover global config: {}", e);
                    None
                });

        // Extract global paths if config exists
        let (global_root, global_policies, global_signals, global_actions, global_rulebook) =
            if let Some(global) = global_config {
                info!("Global configuration discovered at {:?}", global.root);
                (
                    Some(global.root),
                    Some(global.policies),
                    Some(global.signals),
                    Some(global.actions),
                    Some(global.rulebook),
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
            rulebook: cupcake_dir.join("rulebook.yml"),
            global_root,
            global_policies,
            global_signals,
            global_actions,
            global_rulebook,
        })
    }

    /// Get project-level watchdog directory path (.cupcake/watchdog/)
    ///
    /// Returns Some if the directory exists, None otherwise.
    pub fn project_watchdog_dir(&self) -> Option<PathBuf> {
        let watchdog_dir = self.cupcake_dir.join("watchdog");
        if watchdog_dir.exists() {
            Some(watchdog_dir)
        } else {
            None
        }
    }

    /// Get global watchdog directory path (~/.config/cupcake/watchdog/)
    ///
    /// Returns Some if the directory exists, None otherwise.
    pub fn global_watchdog_dir(&self) -> Option<PathBuf> {
        self.global_root.as_ref().and_then(|root| {
            let watchdog_dir = root.join("watchdog");
            if watchdog_dir.exists() {
                Some(watchdog_dir)
            } else {
                None
            }
        })
    }
}

// Core engine modules - discovery and compilation
pub mod compiler;
pub mod metadata;
pub mod scanner;

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

// Re-export metadata types for public API
pub use metadata::{PolicyMetadata, RoutingDirective};

/// Configuration for engine initialization
/// Provides optional overrides for engine behavior via CLI flags
#[derive(Debug, Clone)]
pub struct EngineConfig {
    /// The AI coding agent harness type (REQUIRED)
    /// Determines which event schema and response format to use
    pub harness: crate::harness::types::HarnessType,

    /// Override WASM maximum memory (in bytes)
    /// If None, uses default 10MB with 1MB-100MB enforcement
    pub wasm_max_memory: Option<usize>,

    /// Override OPA binary path
    /// If None, uses bundled OPA or system PATH
    pub opa_path: Option<PathBuf>,

    /// Override global config directory
    /// If None, uses platform-specific default (~/.config/cupcake or ~/Library/Application Support/cupcake)
    pub global_config: Option<PathBuf>,

    /// Enable routing diagnostics debug output
    /// If true, writes routing maps to .cupcake/debug/routing/
    pub debug_routing: bool,
}

impl EngineConfig {
    /// Create a new EngineConfig with required harness parameter
    pub fn new(harness: crate::harness::types::HarnessType) -> Self {
        Self {
            harness,
            wasm_max_memory: None,
            opa_path: None,
            global_config: None,
            debug_routing: false,
        }
    }
}

/// Represents a discovered policy unit with its metadata.
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

        // Step 2: Parse selectors and build policy units
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

        if self.policies.is_empty() {
            warn!("No valid policies found in directory");
            return Ok(());
        }

        // Step 3: Build routing map
        self.build_routing_map();
        info!("Built routing map with {} entries", self.routing_map.len());

        // No entrypoint mapping needed - Hybrid Model uses single aggregation entrypoint

        // Step 4: Compile unified WASM module with OPA path from CLI
        let wasm_bytes =
            compiler::compile_policies(&self.policies, self.config.opa_path.clone()).await?;
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
                    let global_wasm_bytes = compiler::compile_policies_with_namespace(
                        &self.global_policies,
                        "cupcake.global.system",
                        self.config.opa_path.clone(),
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
            } else if package_name.ends_with(".system") {
                // System policies don't need routing - they're aggregation endpoints
                // This covers both cupcake.system and cupcake.global.system
                debug!(
                    "System policy {} has no routing directive (this is expected)",
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
            // System policies are allowed to have no metadata
            if package_name.ends_with(".system") {
                debug!(
                    "System policy {} has no metadata block (this is allowed)",
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
            // Create all routing keys for this policy from metadata
            let routing_keys = routing::create_all_routing_keys_from_metadata(&policy.routing);

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

    /// Evaluate policies for a hook event.
    ///
    /// This is the main public API for policy evaluation.
    #[instrument(
        name = "evaluate",
        skip(self, input, debug_capture),
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
        mut debug_capture: Option<&mut DebugCapture>,
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

        // PHASE 1: Evaluate global policies first (if they exist)
        if self.global_wasm_runtime.is_some() {
            debug!("Phase 1: Evaluating global policies");
            let (global_decision, global_decision_set) = self
                .evaluate_global(&safe_input, event_name, tool_name)
                .await?;

            // Early termination on global blocking decisions
            match &global_decision {
                decision::FinalDecision::Halt { reason, .. } => {
                    info!("Global policy HALT - immediate termination: {}", reason);
                    // Execute global actions before returning
                    if let Some(ref global_rulebook) = self.global_rulebook {
                        info!("Global rulebook found - executing global actions");
                        self.execute_actions_with_rulebook(
                            &global_decision,
                            &global_decision_set,
                            global_rulebook,
                        )
                        .await;
                    } else {
                        warn!("No global rulebook - cannot execute global actions!");
                    }
                    current_span.record("final_decision", "GlobalHalt");
                    current_span.record("duration_ms", eval_start.elapsed().as_millis());
                    return Ok(global_decision);
                }
                decision::FinalDecision::Deny { reason, .. } => {
                    info!("Global policy DENY - immediate termination: {}", reason);
                    // Execute global actions before returning
                    if let Some(ref global_rulebook) = self.global_rulebook {
                        self.execute_actions_with_rulebook(
                            &global_decision,
                            &global_decision_set,
                            global_rulebook,
                        )
                        .await;
                    }
                    current_span.record("final_decision", "GlobalDeny");
                    current_span.record("duration_ms", eval_start.elapsed().as_millis());
                    return Ok(global_decision);
                }
                decision::FinalDecision::Block { reason, .. } => {
                    info!("Global policy BLOCK - immediate termination: {}", reason);
                    // Execute global actions before returning
                    if let Some(ref global_rulebook) = self.global_rulebook {
                        self.execute_actions_with_rulebook(
                            &global_decision,
                            &global_decision_set,
                            global_rulebook,
                        )
                        .await;
                    }
                    current_span.record("final_decision", "GlobalBlock");
                    current_span.record("duration_ms", eval_start.elapsed().as_millis());
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
        let matched_policies: Vec<PolicyUnit> = self
            .route_event(event_name, tool_name)
            .into_iter()
            .cloned()
            .collect();

        // Capture routing results
        if let Some(ref mut debug) = debug_capture {
            debug.routed = !matched_policies.is_empty();
            debug.matched_policies = matched_policies
                .iter()
                .map(|p| p.package_name.clone())
                .collect();
        }

        if matched_policies.is_empty() {
            info!("No policies matched for this event - allowing");
            current_span.record("matched_policy_count", 0);
            current_span.record("final_decision", "Allow");
            current_span.record("duration_ms", eval_start.elapsed().as_millis());
            return Ok(decision::FinalDecision::Allow { context: vec![] });
        }

        current_span.record("matched_policy_count", matched_policies.len());
        info!("Found {} matching policies", matched_policies.len());

        // Step 2: Gather Signals - collect all required signals from matched policies
        let enriched_input = self
            .gather_signals(&safe_input, &matched_policies, debug_capture.as_deref_mut())
            .await?;

        // Step 3: Evaluate using single aggregation entrypoint with enriched input
        debug!("About to evaluate decision set with enriched input");
        let decision_set = self.evaluate_decision_set(&enriched_input).await?;

        // Capture WASM evaluation results
        if let Some(ref mut debug) = debug_capture {
            debug.wasm_decision_set = Some(decision_set.clone());
        }

        // Step 4: Apply Intelligence Layer synthesis
        let final_decision = synthesis::SynthesisEngine::synthesize(&decision_set)?;

        info!("Synthesized final decision: {:?}", final_decision);

        // Capture synthesis results
        if let Some(ref mut debug) = debug_capture {
            debug.final_decision = Some(final_decision.clone());
        }

        // Step 5: Execute actions based on decision (async, non-blocking)
        self.execute_actions_with_debug(
            &final_decision,
            &decision_set,
            debug_capture.as_deref_mut(),
        )
        .await;

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
    async fn evaluate_global(
        &self,
        input: &Value,
        event_name: &str,
        tool_name: Option<&str>,
    ) -> Result<(decision::FinalDecision, decision::DecisionSet)> {
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
            ));
        }

        info!("Found {} matching global policies", global_matched.len());

        // Gather signals for global policies (using global rulebook if available)
        let enriched_input = if let Some(ref global_rulebook) = self.global_rulebook {
            self.gather_signals_with_rulebook(input, &global_matched, global_rulebook)
                .await?
        } else {
            input.clone()
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

        Ok((global_decision, global_decision_set))
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

    /// Gather signals with a specific rulebook
    async fn gather_signals_with_rulebook(
        &self,
        input: &Value,
        matched_policies: &[PolicyUnit],
        rulebook: &rulebook::Rulebook,
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
            let is_global_builtin = policy
                .package_name
                .starts_with("cupcake.global.policies.builtins.");
            let is_project_builtin = policy
                .package_name
                .starts_with("cupcake.policies.builtins.");

            if is_global_builtin || is_project_builtin {
                // Extract the builtin name from the package
                let prefix = if is_global_builtin {
                    "cupcake.global.policies.builtins."
                } else {
                    "cupcake.policies.builtins."
                };

                let builtin_name = policy.package_name.strip_prefix(prefix).unwrap_or("");

                // Add all signals that match this builtin's pattern
                let signal_prefix = format!("__builtin_{builtin_name}_");
                for signal_name in rulebook.signals.keys() {
                    if signal_name.starts_with(&signal_prefix) {
                        debug!(
                            "Auto-adding signal '{}' for global builtin '{}'",
                            signal_name, builtin_name
                        );
                        required_signals.insert(signal_name.clone());
                    }
                }

                // Also check for signals without the trailing underscore (like __builtin_system_protection_paths)
                let signal_prefix_no_underscore = format!("__builtin_{builtin_name}");
                for signal_name in rulebook.signals.keys() {
                    if signal_name.starts_with(&signal_prefix_no_underscore)
                        && !signal_name.starts_with(&signal_prefix)
                    {
                        debug!(
                            "Auto-adding signal '{}' for global builtin '{}'",
                            signal_name, builtin_name
                        );
                        required_signals.insert(signal_name.clone());
                    }
                }
            }
        }

        // Always inject builtin config from the provided rulebook
        let mut enriched_input = input.clone();
        if let Some(input_obj) = enriched_input.as_object_mut() {
            debug!("Injecting builtin configs from rulebook in gather_signals_with_rulebook");
            let builtin_config = rulebook.builtins.to_json_configs();

            if !builtin_config.is_empty() {
                debug!("Injected {} builtin configurations", builtin_config.len());
                input_obj.insert(
                    "builtin_config".to_string(),
                    serde_json::Value::Object(builtin_config),
                );
            }
        }

        if required_signals.is_empty() {
            debug!("No signals required - returning with builtin config");
            return Ok(enriched_input);
        }

        let signal_names: Vec<String> = required_signals.into_iter().collect();
        info!("Gathering {} signals from rulebook", signal_names.len());

        // Execute signals using the provided rulebook, passing the event data
        let signal_data = self
            .execute_signals_from_rulebook(&signal_names, rulebook, input)
            .await
            .unwrap_or_else(|e| {
                warn!("Signal execution failed: {}", e);
                std::collections::HashMap::new()
            });

        // Merge signal data into already-enriched input
        if let Some(obj) = enriched_input.as_object_mut() {
            obj.insert("signals".to_string(), serde_json::json!(signal_data));
        }

        Ok(enriched_input)
    }

    /// Execute signals from a specific rulebook
    async fn execute_signals_from_rulebook(
        &self,
        signal_names: &[String],
        rulebook: &rulebook::Rulebook,
        event_data: &Value,
    ) -> Result<std::collections::HashMap<String, Value>> {
        // Use the rulebook's execute_signals_with_input method to pass event data
        rulebook
            .execute_signals_with_input(signal_names, event_data)
            .await
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

    /// Gather signals required by the matched policies and enrich the input
    #[instrument(
        name = "gather_signals",
        skip(self, input, matched_policies, debug_capture),
        fields(
            signal_count = tracing::field::Empty,
            signals_executed = tracing::field::Empty,
            duration_ms = tracing::field::Empty
        )
    )]
    async fn gather_signals(
        &self,
        input: &Value,
        matched_policies: &[PolicyUnit],
        mut debug_capture: Option<&mut DebugCapture>,
    ) -> Result<Value> {
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
        if let Some(rulebook) = &self.rulebook {
            for policy in matched_policies {
                if policy
                    .package_name
                    .starts_with("cupcake.policies.builtins.")
                {
                    // This is a builtin policy - add signals that match its pattern
                    let builtin_name = policy
                        .package_name
                        .strip_prefix("cupcake.policies.builtins.")
                        .unwrap_or("");

                    // Special handling for post_edit_check - only add the signal for the actual file extension
                    // This optimization prevents running ALL validation commands when only one applies
                    if builtin_name == "post_edit_check" {
                        if let Some(signal_name) = rulebook.builtins.get_post_edit_signal(input) {
                            if rulebook.signals.contains_key(&signal_name) {
                                debug!(
                                    "Auto-adding signal '{}' for post_edit_check builtin",
                                    signal_name
                                );
                                required_signals.insert(signal_name);
                            }
                        }
                    } else {
                        // For other builtins, add all matching signals
                        let signal_prefix = format!("__builtin_{builtin_name}_");
                        for signal_name in rulebook.signals.keys() {
                            if signal_name.starts_with(&signal_prefix) {
                                debug!(
                                    "Auto-adding signal '{}' for builtin '{}'",
                                    signal_name, builtin_name
                                );
                                required_signals.insert(signal_name.clone());
                            }
                        }
                    }
                }
            }
        }

        // Always inject builtin config, even when no signals are required
        let mut enriched_input = input.clone();
        if let Some(input_obj) = enriched_input.as_object_mut() {
            // Inject builtin configuration directly (no shell execution needed for static values)
            let mut builtin_config = serde_json::Map::new();

            // First inject project configs (baseline from project)
            if let Some(project_rulebook) = &self.rulebook {
                debug!("Injecting builtin configs from project rulebook");
                builtin_config.extend(project_rulebook.builtins.to_json_configs());
            }

            // Then inject global configs (override project - global enforcement takes precedence)
            if let Some(global_rulebook) = &self.global_rulebook {
                debug!("Injecting builtin configs from global rulebook (overrides project)");
                builtin_config.extend(global_rulebook.builtins.to_json_configs());
            }

            if !builtin_config.is_empty() {
                debug!(
                    "Injected {} builtin configurations total (global + project)",
                    builtin_config.len()
                );
                // Debug: print the actual builtin_config
                debug!("Builtin config contents: {:?}", builtin_config);
                input_obj.insert(
                    "builtin_config".to_string(),
                    serde_json::Value::Object(builtin_config),
                );
            } else {
                debug!("No builtin configurations to inject");
            }
        }

        // Check if watchdog should run (needed to decide whether to skip signal gathering)
        // Watchdog only runs on pre-action events (MVP scope)
        // Supported events by harness:
        // - Claude Code / OpenCode: PreToolUse
        // - Cursor: beforeShellExecution, beforeMCPExecution
        let is_pre_action_event = input
            .get("hook_event_name")
            .and_then(|v| v.as_str())
            .map(|s| {
                matches!(
                    s,
                    // Claude Code / OpenCode
                    "PreToolUse" |
                    // Cursor pre-action events
                    "beforeShellExecution" |
                    "beforeMCPExecution"
                )
            })
            .unwrap_or(false);
        let watchdog_should_run = self.watchdog.is_some() && is_pre_action_event;

        if required_signals.is_empty() && !watchdog_should_run {
            debug!("No signals required and watchdog not enabled - returning with builtin config");
            return Ok(enriched_input);
        }

        let signal_names: Vec<String> = required_signals.into_iter().collect();
        info!(
            "Gathering {} signals: {:?}",
            signal_names.len(),
            signal_names
        );

        // Capture configured signals
        if let Some(ref mut debug) = debug_capture {
            debug.signals_configured = signal_names.clone();
        }

        // Execute signals if we have a rulebook, passing the event data (input)
        let signal_data = if let Some(rulebook) = &self.rulebook {
            let results = self
                .execute_signals_with_trust_and_debug(
                    &signal_names,
                    rulebook,
                    input,
                    debug_capture.as_deref_mut(),
                )
                .await
                .unwrap_or_else(|e| {
                    warn!("Signal execution failed: {}", e);
                    std::collections::HashMap::<String, serde_json::Value>::new()
                });
            results
        } else {
            debug!("No rulebook available - no signals collected");
            std::collections::HashMap::<String, serde_json::Value>::new()
        };

        // Merge signal data into already-enriched input (which has builtin_config)
        let mut signal_count = signal_data.len();
        // enriched_input already has builtin_config from above
        if let Some(input_obj) = enriched_input.as_object_mut() {
            let mut signals_obj = serde_json::to_value(signal_data)?;

            // Execute Watchdog if enabled (is_pre_action_event already computed above)
            if let Some(ref watchdog) = self.watchdog {
                if is_pre_action_event {
                    debug!(
                        "Executing Watchdog evaluation for {:?} event",
                        input.get("hook_event_name")
                    );
                    let watchdog_input = crate::watchdog::Watchdog::input_from_event(input);
                    let watchdog_output = watchdog.evaluate(watchdog_input).await;
                    debug!(
                        "Watchdog result: allow={}, confidence={}",
                        watchdog_output.allow, watchdog_output.confidence
                    );

                    if let Some(signals_map) = signals_obj.as_object_mut() {
                        signals_map.insert(
                            "watchdog".to_string(),
                            serde_json::to_value(&watchdog_output)?,
                        );
                        signal_count += 1;
                    }
                } else {
                    debug!(
                        "Skipping Watchdog - only runs on pre-action events (got {:?})",
                        input.get("hook_event_name")
                    );
                }
            }

            input_obj.insert("signals".to_string(), signals_obj);
        }

        // Record span fields
        let duration = start.elapsed();
        let current_span = tracing::Span::current();
        current_span.record("signal_count", signal_count);
        current_span.record("signals_executed", signal_names.join(",").as_str());
        current_span.record("duration_ms", duration.as_millis());

        debug!("Input enriched with {} signal values", signal_count);
        trace!(
            signal_count = signal_count,
            duration_ms = duration.as_millis(),
            "Signal gathering complete"
        );

        Ok(enriched_input)
    }

    /// Execute signals with trust verification and debug capture
    async fn execute_signals_with_trust_and_debug(
        &self,
        signal_names: &[String],
        rulebook: &rulebook::Rulebook,
        event_data: &Value,
        mut debug_capture: Option<&mut DebugCapture>,
    ) -> Result<HashMap<String, serde_json::Value>> {
        use crate::debug::SignalExecution;
        use futures::future::join_all;

        if signal_names.is_empty() {
            return Ok(HashMap::new());
        }

        debug!(
            "Executing {} signals with trust verification",
            signal_names.len()
        );

        let futures: Vec<_> = signal_names
            .iter()
            .map(|name| {
                let name = name.clone();
                let trust_verifier = self.trust_verifier.clone();
                let signal_config = rulebook.get_signal(&name).cloned();
                let event_data = event_data.clone();

                async move {
                    // Get the signal config
                    let signal = match signal_config {
                        Some(s) => s,
                        None => {
                            return (
                                name.clone(),
                                Err(anyhow::anyhow!("Signal '{}' not found", name)),
                                None,
                            );
                        }
                    };

                    // Verify trust if enabled
                    if let Some(verifier) = &trust_verifier {
                        if let Err(e) = verifier.verify_script(&signal.command).await {
                            // Log trust violation specially (TrustError logs itself on creation)
                            return (
                                name.clone(),
                                Err(anyhow::anyhow!("Trust verification failed: {}", e)),
                                None,
                            );
                        }
                    }

                    // Execute the signal with event data and measure time
                    let signal_start = std::time::Instant::now();
                    let result = rulebook.execute_signal_with_input(&name, &event_data).await;
                    let signal_duration = signal_start.elapsed();

                    // Create signal execution record for debug
                    let signal_execution = SignalExecution {
                        name: name.clone(),
                        command: signal.command.clone(),
                        result: result.as_ref().unwrap_or(&serde_json::Value::Null).clone(),
                        duration_ms: Some(signal_duration.as_millis()),
                    };

                    (name, result, Some(signal_execution))
                }
            })
            .collect();

        let results = join_all(futures).await;

        let mut signal_data = HashMap::new();
        for (name, result, signal_execution) in results {
            match result {
                Ok(value) => {
                    debug!("Signal '{}' executed successfully", name);
                    signal_data.insert(name, value);

                    // Capture signal execution if debug enabled
                    if let (Some(ref mut debug), Some(execution)) =
                        (&mut debug_capture, signal_execution)
                    {
                        debug.signals_executed.push(execution);
                    }
                }
                Err(e) => {
                    // Log error but don't fail the whole evaluation
                    error!("Signal '{}' failed: {}", name, e);

                    // Still capture failed signal execution for debug
                    if let (Some(ref mut debug), Some(execution)) =
                        (&mut debug_capture, signal_execution)
                    {
                        debug.signals_executed.push(execution);
                        debug.add_error(format!("Signal '{name}' failed: {e}"));
                    }
                }
            }
        }

        Ok(signal_data)
    }

    /// Execute actions based on the final decision and decision set
    /// Execute actions with a specific rulebook
    async fn execute_actions_with_rulebook(
        &self,
        final_decision: &decision::FinalDecision,
        decision_set: &decision::DecisionSet,
        rulebook: &rulebook::Rulebook,
    ) {
        info!(
            "execute_actions_with_rulebook called with decision: {:?}",
            final_decision
        );

        // Determine working directory based on whether this is a global rulebook
        // Check if we're using the global rulebook by comparing pointers
        let is_global = self
            .global_rulebook
            .as_ref()
            .map(|gb| std::ptr::eq(gb as *const _, rulebook as *const _))
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
            decision::FinalDecision::Halt { reason, .. } => {
                info!("Executing actions for HALT decision: {}", reason);
                info!(
                    "Number of halt decisions in set: {}",
                    decision_set.halts.len()
                );
                self.execute_rule_specific_actions(&decision_set.halts, rulebook, &working_dir)
                    .await;
            }
            decision::FinalDecision::Deny { reason, .. } => {
                info!("Executing actions for DENY decision: {}", reason);

                // Execute general denial actions
                for action in &rulebook.actions.on_any_denial {
                    self.execute_single_action(action, &working_dir).await;
                }

                // Execute rule-specific actions for denials
                self.execute_rule_specific_actions(&decision_set.denials, rulebook, &working_dir)
                    .await;
            }
            decision::FinalDecision::Block { reason, .. } => {
                info!("Executing actions for BLOCK decision: {}", reason);
                self.execute_rule_specific_actions(&decision_set.blocks, rulebook, &working_dir)
                    .await;
            }
            decision::FinalDecision::Ask { .. } => {
                debug!("ASK decision - no automatic actions");
            }
            decision::FinalDecision::Allow { .. } => {
                debug!("ALLOW decision - no actions needed");
            }
            decision::FinalDecision::AllowOverride { .. } => {
                debug!("ALLOW_OVERRIDE decision - no actions needed");
            }
        }
    }

    /// Execute actions with debug capture
    async fn execute_actions_with_debug(
        &self,
        final_decision: &decision::FinalDecision,
        decision_set: &decision::DecisionSet,
        mut debug_capture: Option<&mut DebugCapture>,
    ) {
        let Some(rulebook) = &self.rulebook else {
            debug!("No rulebook available - no actions to execute");
            return;
        };

        // Capture configured actions before execution
        if let Some(ref mut debug) = debug_capture {
            // Collect actions that would be configured based on the decision
            let mut configured_actions = Vec::new();

            match final_decision {
                decision::FinalDecision::Deny { .. } => {
                    // General denial actions
                    for action in &rulebook.actions.on_any_denial {
                        configured_actions.push(format!("on_any_denial: {}", action.command));
                    }

                    // Rule-specific actions for denials
                    for decision_obj in &decision_set.denials {
                        if let Some(actions) =
                            rulebook.actions.by_rule_id.get(&decision_obj.rule_id)
                        {
                            for action in actions {
                                configured_actions
                                    .push(format!("{}: {}", decision_obj.rule_id, action.command));
                            }
                        }
                    }
                }
                decision::FinalDecision::Halt { .. } => {
                    // Rule-specific actions for halts
                    for decision_obj in &decision_set.halts {
                        if let Some(actions) =
                            rulebook.actions.by_rule_id.get(&decision_obj.rule_id)
                        {
                            for action in actions {
                                configured_actions
                                    .push(format!("{}: {}", decision_obj.rule_id, action.command));
                            }
                        }
                    }
                }
                decision::FinalDecision::Block { .. } => {
                    // Rule-specific actions for blocks
                    for decision_obj in &decision_set.blocks {
                        if let Some(actions) =
                            rulebook.actions.by_rule_id.get(&decision_obj.rule_id)
                        {
                            for action in actions {
                                configured_actions
                                    .push(format!("{}: {}", decision_obj.rule_id, action.command));
                            }
                        }
                    }
                }
                _ => {
                    // No actions for Ask, Allow, AllowOverride
                }
            }

            debug.actions_configured = configured_actions;
        }

        self.execute_actions_with_rulebook_and_debug(
            final_decision,
            decision_set,
            rulebook,
            debug_capture,
        )
        .await;
    }

    /// Execute actions with rulebook and debug capture
    async fn execute_actions_with_rulebook_and_debug(
        &self,
        final_decision: &decision::FinalDecision,
        decision_set: &decision::DecisionSet,
        rulebook: &rulebook::Rulebook,
        mut debug_capture: Option<&mut DebugCapture>,
    ) {
        info!(
            "execute_actions_with_rulebook_and_debug called with decision: {:?}",
            final_decision
        );

        let working_dir = &self.paths.root.clone();

        // Execute actions based on decision type, capturing execution details
        match final_decision {
            decision::FinalDecision::Halt { reason, .. } => {
                info!("Executing actions for HALT decision: {}", reason);
                self.execute_rule_specific_actions_with_debug(
                    &decision_set.halts,
                    rulebook,
                    working_dir,
                    debug_capture.as_deref_mut(),
                )
                .await;
            }
            decision::FinalDecision::Deny { reason, .. } => {
                info!("Executing actions for DENY decision: {}", reason);

                // Execute general denial actions
                for action in &rulebook.actions.on_any_denial {
                    self.execute_single_action_with_debug(
                        action,
                        working_dir,
                        debug_capture.as_deref_mut(),
                    )
                    .await;
                }

                // Execute rule-specific actions for denials
                self.execute_rule_specific_actions_with_debug(
                    &decision_set.denials,
                    rulebook,
                    working_dir,
                    debug_capture.as_deref_mut(),
                )
                .await;
            }
            decision::FinalDecision::Block { reason, .. } => {
                info!("Executing actions for BLOCK decision: {}", reason);
                self.execute_rule_specific_actions_with_debug(
                    &decision_set.blocks,
                    rulebook,
                    working_dir,
                    debug_capture,
                )
                .await;
            }
            decision::FinalDecision::Ask { .. } => {
                debug!("ASK decision - no automatic actions");
            }
            decision::FinalDecision::Allow { .. } => {
                debug!("ALLOW decision - no actions needed");
            }
            decision::FinalDecision::AllowOverride { .. } => {
                debug!("ALLOW_OVERRIDE decision - no actions needed");
            }
        }
    }

    /// Execute actions for a specific set of decision objects (by rule ID)
    async fn execute_rule_specific_actions(
        &self,
        decisions: &[decision::DecisionObject],
        rulebook: &rulebook::Rulebook,
        working_dir: &std::path::PathBuf,
    ) {
        info!(
            "execute_rule_specific_actions: Checking actions for {} decision objects",
            decisions.len()
        );
        info!(
            "Available action rules: {:?}",
            rulebook.actions.by_rule_id.keys().collect::<Vec<_>>()
        );

        for decision_obj in decisions {
            let rule_id = &decision_obj.rule_id;
            info!("Looking for actions for rule ID: {}", rule_id);

            if let Some(actions) = rulebook.actions.by_rule_id.get(rule_id) {
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

    /// Execute actions for a specific set of decision objects with debug capture
    async fn execute_rule_specific_actions_with_debug(
        &self,
        decisions: &[decision::DecisionObject],
        rulebook: &rulebook::Rulebook,
        working_dir: &std::path::PathBuf,
        mut debug_capture: Option<&mut DebugCapture>,
    ) {
        info!(
            "execute_rule_specific_actions_with_debug: Checking actions for {} decision objects",
            decisions.len()
        );

        // Create a list of actions to execute to avoid borrowing issues
        let mut actions_to_execute = Vec::new();

        for decision_obj in decisions {
            let rule_id = &decision_obj.rule_id;

            if let Some(actions) = rulebook.actions.by_rule_id.get(rule_id) {
                info!("Found {} actions for rule {}", actions.len(), rule_id);
                for action in actions {
                    actions_to_execute.push(action.clone());
                }
            }
        }

        // Now execute all actions
        for action in actions_to_execute {
            info!("About to execute action: {}", action.command);
            self.execute_single_action_with_debug(
                &action,
                working_dir,
                debug_capture.as_deref_mut(),
            )
            .await;
        }
    }

    /// Execute a single action command
    async fn execute_single_action(
        &self,
        action: &rulebook::ActionConfig,
        working_dir: &std::path::PathBuf,
    ) {
        self.execute_single_action_with_debug(action, working_dir, None)
            .await;
    }

    /// Execute a single action command with debug capture
    async fn execute_single_action_with_debug(
        &self,
        action: &rulebook::ActionConfig,
        working_dir: &std::path::PathBuf,
        mut debug_capture: Option<&mut DebugCapture>,
    ) {
        debug!(
            "Executing action: {} in directory: {:?}",
            action.command, working_dir
        );

        // Capture action execution
        if let Some(ref mut debug) = debug_capture {
            let action_execution = crate::debug::ActionExecution {
                name: format!("action_{}", debug.actions_executed.len()),
                command: action.command.clone(),
                duration_ms: None, // Could be captured if we measure execution time
                exit_code: None,   // Could be captured from command result
            };
            debug.actions_executed.push(action_execution);
        }

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
            // On Windows, .sh files need to be invoked through bash
            let is_script_path = (command.starts_with('/') || command.starts_with("./"))
                && !command.contains("&&")
                && !command.contains("||")
                && !command.contains(';')
                && !command.contains('|')
                && !command.contains('>');

            let is_shell_script = command.ends_with(".sh");

            let result = if is_script_path && !is_shell_script {
                // It's a script file (but not .sh), execute directly
                // Extract the directory from the script path to use as working directory
                let script_path = std::path::Path::new(&command);
                let script_working_dir = script_path
                    .parent()
                    .and_then(|p| p.parent())
                    .and_then(|p| p.parent())
                    .unwrap_or(&working_dir);

                tokio::process::Command::new(&command)
                    .current_dir(script_working_dir)
                    .output()
                    .await
            } else if is_shell_script && cfg!(windows) {
                // On Windows, .sh files must be invoked through bash
                // Extract the directory from the script path to use as working directory
                let script_path = std::path::Path::new(&command);
                let script_working_dir = script_path
                    .parent()
                    .and_then(|p| p.parent())
                    .and_then(|p| p.parent())
                    .unwrap_or(&working_dir);

                // Convert Windows path to Git Bash compatible Unix-style path
                // C:\Users\foo -> /c/Users/foo
                let bash_path = if command.len() >= 3 && command.chars().nth(1) == Some(':') {
                    let drive = command.chars().next().unwrap().to_lowercase();
                    let path_part = &command[2..].replace('\\', "/");
                    format!("/{drive}{path_part}")
                } else {
                    command.replace('\\', "/")
                };

                tokio::process::Command::new(*SHELL_COMMAND)
                    .arg(&bash_path)
                    .current_dir(script_working_dir)
                    .output()
                    .await
            } else {
                // It's a shell command or a .sh script on Unix, use shell -c
                // Use the provided working directory
                let result = tokio::process::Command::new(*SHELL_COMMAND)
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
                        warn!(
                            "Action failed: {} - stderr: {} stdout: {}",
                            command, stderr, stdout
                        );
                    }
                }
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
