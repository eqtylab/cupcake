//! Integration tests for Watchdog configuration loading and precedence
//!
//! Tests use static fixtures in `tests/fixtures/watchdog/config-setup/`
//!
//! Fixture structure:
//! - project_only/     - Only project config, no global
//! - global_only/      - Only global config, no project
//! - project_overrides_global/ - Both exist, project wins
//! - mixed_sources/    - Project has config.json, global has prompts
//!
//! Note: dry_run is a CLI-only flag, not part of file config.

use cupcake_core::watchdog::{WatchdogConfig, WatchdogPrompts};
use std::path::Path;

/// Get the fixtures base path
fn fixtures_path() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/watchdog/config-setup")
}

// ============================================================================
// Config Precedence Tests
// ============================================================================

#[test]
fn test_project_only_config() {
    let fixtures = fixtures_path().join("project_only");

    let config = WatchdogConfig::load_from_directory(
        Some(&fixtures.join("project")),
        None, // No global
    );

    assert!(config.enabled);
    let or_config = config.openrouter.as_ref().expect("Should have openrouter config");
    assert_eq!(or_config.model, "project-model");
    assert_eq!(config.timeout_seconds, 30);
}

#[test]
fn test_global_only_config() {
    let fixtures = fixtures_path().join("global_only");

    let config = WatchdogConfig::load_from_directory(
        None, // No project
        Some(&fixtures.join("global")),
    );

    assert!(config.enabled);
    let or_config = config.openrouter.as_ref().expect("Should have openrouter config");
    assert_eq!(or_config.model, "global-model");
    assert!(!config.fail_open(), "on_error=deny means fail_open=false");
}

#[test]
fn test_project_overrides_global() {
    let fixtures = fixtures_path().join("project_overrides_global");

    let config = WatchdogConfig::load_from_directory(
        Some(&fixtures.join("project")),
        Some(&fixtures.join("global")),
    );

    assert!(config.enabled);
    let or_config = config.openrouter.as_ref().expect("Should have openrouter config");
    assert_eq!(
        or_config.model, "project-wins",
        "Project config should override global"
    );
    assert_eq!(
        config.timeout_seconds, 25,
        "Project timeout should be used"
    );
}

#[test]
fn test_no_dirs_uses_defaults() {
    // Pass non-existent paths
    let config = WatchdogConfig::load_from_directory(None, None);

    assert!(config.enabled, "Should be enabled with defaults");
    let or_config = config.openrouter.as_ref().expect("Should have default openrouter config");
    assert_eq!(or_config.model, "google/gemini-2.5-flash");
    assert_eq!(or_config.api_key_env, "OPENROUTER_API_KEY");
}

// ============================================================================
// Prompts Precedence Tests
// ============================================================================

#[test]
fn test_project_only_prompts() {
    let fixtures = fixtures_path().join("project_only");

    let prompts = WatchdogPrompts::load(
        Some(&fixtures.join("project")),
        None,
    );

    assert_eq!(prompts.system_prompt, "Project system prompt");
    assert_eq!(prompts.user_template, "Project: {{event}}");
}

#[test]
fn test_global_only_prompts() {
    let fixtures = fixtures_path().join("global_only");

    let prompts = WatchdogPrompts::load(
        None,
        Some(&fixtures.join("global")),
    );

    assert_eq!(prompts.system_prompt, "Global system prompt");
    // No user.txt in global_only, should use default
    assert_eq!(prompts.user_template, "{{event}}");
}

#[test]
fn test_mixed_sources_project_config_global_prompts() {
    let fixtures = fixtures_path().join("mixed_sources");

    // Config from project
    let config = WatchdogConfig::load_from_directory(
        Some(&fixtures.join("project")),
        Some(&fixtures.join("global")),
    );

    let or_config = config.openrouter.as_ref().expect("Should have openrouter config");
    assert_eq!(or_config.model, "project-model");

    // Prompts from global (project has no prompt files)
    let prompts = WatchdogPrompts::load(
        Some(&fixtures.join("project")),
        Some(&fixtures.join("global")),
    );

    assert_eq!(prompts.system_prompt, "Org-wide security prompt");
    assert_eq!(prompts.user_template, "Org template: {{event}}");
}

#[test]
fn test_prompts_render_template() {
    let fixtures = fixtures_path().join("mixed_sources");

    let prompts = WatchdogPrompts::load(
        Some(&fixtures.join("project")),
        Some(&fixtures.join("global")),
    );

    let event = serde_json::json!({
        "tool_name": "Bash",
        "command": "rm -rf /"
    });

    let rendered = prompts.render_user_message(&event);

    assert!(rendered.contains("Org template:"));
    assert!(rendered.contains("\"tool_name\": \"Bash\""));
    assert!(rendered.contains("rm -rf /"));
}

#[test]
fn test_default_prompts_when_no_files() {
    let prompts = WatchdogPrompts::load(None, None);

    assert!(
        prompts.system_prompt.contains("security reviewer"),
        "Default system prompt should contain 'security reviewer'"
    );
    assert_eq!(prompts.user_template, "{{event}}");
}
