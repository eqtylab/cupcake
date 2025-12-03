//! Watchdog CLI command implementation
//!
//! Provides the `cupcake watchdog` command for standalone LLM-as-judge evaluation.
//! This module is only available when the `watchdog` feature is enabled.

use anyhow::{anyhow, Context, Result};
use std::fs;
use std::io::{self, Read};
use std::path::PathBuf;

use cupcake_core::engine::global_config::GlobalPaths;
use cupcake_core::watchdog::Watchdog;

/// Execute the watchdog command
///
/// Loads configuration with proper precedence:
/// 1. Project: `.cupcake/watchdog/` (derived from config_path)
/// 2. Global: Platform-specific config directory (via GlobalPaths::discover())
/// 3. Defaults
///
/// # Arguments
///
/// * `config_path` - Path to the project's .cupcake directory or rulebook.yml
/// * `model_override` - Optional model to use instead of configured one
/// * `input_file` - Optional file to read event JSON from (stdin if None)
/// * `dry_run` - If true, logs config but skips actual LLM calls
pub async fn run(
    config_path: PathBuf,
    model_override: Option<String>,
    input_file: Option<PathBuf>,
    dry_run: bool,
) -> Result<()> {
    // Read event JSON from file or stdin
    let event_json: serde_json::Value = if let Some(path) = input_file {
        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read input file: {path:?}"))?;
        serde_json::from_str(&content).context("Failed to parse input JSON")?
    } else {
        let mut stdin_buffer = String::new();
        io::stdin()
            .read_to_string(&mut stdin_buffer)
            .context("Failed to read from stdin")?;
        serde_json::from_str(&stdin_buffer).context("Failed to parse stdin JSON")?
    };

    // Determine project watchdog directory
    // config_path is typically .cupcake/rulebook.yml or .cupcake/
    let project_watchdog_dir = config_path.parent().map(|p| p.join("watchdog"));

    // Use GlobalPaths::discover() for platform-specific global config
    // This ensures CLI uses the same path as the engine:
    // - Linux: ~/.config/cupcake/watchdog/
    // - macOS: ~/Library/Application Support/cupcake/watchdog/
    // - Windows: %APPDATA%\cupcake\watchdog\
    let global_watchdog_dir = GlobalPaths::discover()
        .ok()
        .flatten()
        .map(|paths| paths.root.join("watchdog"));

    // Use from_directories_with_dry_run() to load config with proper precedence:
    // project directory -> global directory -> defaults
    // dry_run is passed from CLI flag, not config files
    let mut watchdog = Watchdog::from_directories_with_dry_run(
        project_watchdog_dir.as_deref(),
        global_watchdog_dir.as_deref(),
        dry_run,
    )
    .context("Failed to initialize Watchdog from directories")?;

    // Apply model override if provided via CLI flag
    if let Some(model) = model_override {
        watchdog.override_model(model);
    }

    if !watchdog.is_enabled() {
        return Err(anyhow!(
            "Watchdog failed to initialize. Check that OPENROUTER_API_KEY is set."
        ));
    }

    // Build input and evaluate
    let watchdog_input = Watchdog::input_from_event(&event_json);
    let output = watchdog.evaluate(watchdog_input).await;

    // Output result as JSON
    println!("{}", serde_json::to_string_pretty(&output)?);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_global_paths_discovery_does_not_panic() {
        // Just verify that GlobalPaths::discover() works without panicking
        let result = GlobalPaths::discover();
        // Result can be Ok(Some(_)), Ok(None), or Err(_) depending on system
        // We just want to ensure it doesn't panic
        assert!(result.is_ok() || result.is_err());
    }
}
