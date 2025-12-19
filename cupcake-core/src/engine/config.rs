//! Engine configuration and path resolution.
//!
//! Contains configuration structs and path resolution logic that are used
//! during engine initialization.

use anyhow::{anyhow, Result};
use once_cell::sync::Lazy;
use std::path::{Path, PathBuf};
use tracing::{debug, info};

use super::global_config;

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
pub static SHELL_COMMAND: Lazy<&'static str> = Lazy::new(find_shell_command);

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
