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
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/watchdog/config-setup")
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
    let or_config = config
        .openrouter
        .as_ref()
        .expect("Should have openrouter config");
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
    let or_config = config
        .openrouter
        .as_ref()
        .expect("Should have openrouter config");
    assert_eq!(or_config.model, "global-model");
    assert!(
        !config.allows_on_error(),
        "on_error=deny means allows_on_error=false"
    );
}

#[test]
fn test_project_overrides_global() {
    let fixtures = fixtures_path().join("project_overrides_global");

    let config = WatchdogConfig::load_from_directory(
        Some(&fixtures.join("project")),
        Some(&fixtures.join("global")),
    );

    assert!(config.enabled);
    let or_config = config
        .openrouter
        .as_ref()
        .expect("Should have openrouter config");
    assert_eq!(
        or_config.model, "project-wins",
        "Project config should override global"
    );
    assert_eq!(config.timeout_seconds, 25, "Project timeout should be used");
}

#[test]
fn test_no_dirs_uses_defaults() {
    // Pass non-existent paths
    let config = WatchdogConfig::load_from_directory(None, None);

    assert!(config.enabled, "Should be enabled with defaults");
    let or_config = config
        .openrouter
        .as_ref()
        .expect("Should have default openrouter config");
    assert_eq!(or_config.model, "google/gemini-2.5-flash");
    assert_eq!(or_config.api_key_env, "OPENROUTER_API_KEY");
}

// ============================================================================
// Prompts Precedence Tests
// ============================================================================

#[test]
fn test_project_only_prompts() {
    let fixtures = fixtures_path().join("project_only");

    let prompts =
        WatchdogPrompts::load(Some(&fixtures.join("project")), None).expect("Should load prompts");

    assert_eq!(prompts.system_prompt, "Project system prompt");
    assert_eq!(prompts.user_template, "Project: {{event}}");
}

#[test]
fn test_global_only_prompts() {
    let fixtures = fixtures_path().join("global_only");

    let prompts =
        WatchdogPrompts::load(None, Some(&fixtures.join("global"))).expect("Should load prompts");

    assert_eq!(prompts.system_prompt, "Global system prompt");
    // No user.txt in global_only, should use default (includes {{rules_context}})
    assert!(prompts.user_template.contains("{{event}}"));
    assert!(prompts.user_template.contains("{{rules_context}}"));
}

#[test]
fn test_mixed_sources_project_config_global_prompts() {
    let fixtures = fixtures_path().join("mixed_sources");

    // Config from project
    let config = WatchdogConfig::load_from_directory(
        Some(&fixtures.join("project")),
        Some(&fixtures.join("global")),
    );

    let or_config = config
        .openrouter
        .as_ref()
        .expect("Should have openrouter config");
    assert_eq!(or_config.model, "project-model");

    // Prompts from global (project has no prompt files)
    let prompts = WatchdogPrompts::load(
        Some(&fixtures.join("project")),
        Some(&fixtures.join("global")),
    )
    .expect("Should load prompts");

    assert_eq!(prompts.system_prompt, "Org-wide security prompt");
    assert_eq!(prompts.user_template, "Org template: {{event}}");
}

#[test]
fn test_prompts_render_template() {
    let fixtures = fixtures_path().join("mixed_sources");

    let prompts = WatchdogPrompts::load(
        Some(&fixtures.join("project")),
        Some(&fixtures.join("global")),
    )
    .expect("Should load prompts");

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
    let prompts = WatchdogPrompts::load(None, None).expect("Should load default prompts");

    assert!(
        prompts.system_prompt.contains("security reviewer"),
        "Default system prompt should contain 'security reviewer'"
    );
    assert!(prompts.user_template.contains("{{event}}"));
    assert!(prompts.user_template.contains("{{rules_context}}"));
}

// ============================================================================
// Rules Context Tests
// ============================================================================

#[test]
fn test_rules_context_loads_files() {
    use cupcake_core::watchdog::WatchdogDirConfig;

    let fixtures = fixtures_path().join("rules_context");
    let project_dir = fixtures.join("project");

    // Load config to get rules_context
    let dir_config = WatchdogDirConfig::load_from_dir(&project_dir).expect("Should load config");

    let rules_context = dir_config.rules_context.expect("Should have rules_context");

    assert_eq!(rules_context.root_path, "..");
    assert_eq!(rules_context.files.len(), 2);
    assert!(rules_context.files.contains(&"CLAUDE.md".to_string()));
    assert!(rules_context.files.contains(&"RULES.md".to_string()));
    // Default strict mode should be true
    assert!(rules_context.strict, "strict should default to true");
}

#[test]
fn test_rules_context_included_in_prompts() {
    use cupcake_core::watchdog::WatchdogDirConfig;

    let fixtures = fixtures_path().join("rules_context");
    let project_dir = fixtures.join("project");

    // Load config to get rules_context
    let dir_config = WatchdogDirConfig::load_from_dir(&project_dir).expect("Should load config");
    let rules_context = dir_config.rules_context.as_ref();

    // Load prompts with rules context
    let prompts = WatchdogPrompts::load_with_rules_context(Some(&project_dir), None, rules_context)
        .expect("Should load prompts");

    // Verify rules_context contains both files
    assert!(
        prompts.rules_context.contains("=== CLAUDE.md ==="),
        "Should contain CLAUDE.md header"
    );
    assert!(
        prompts.rules_context.contains("=== RULES.md ==="),
        "Should contain RULES.md header"
    );

    // Verify content from CLAUDE.md
    assert!(
        prompts
            .rules_context
            .contains("Never delete files without explicit user confirmation"),
        "Should contain CLAUDE.md content"
    );
    assert!(
        prompts
            .rules_context
            .contains("Do not access files outside the project directory"),
        "Should contain CLAUDE.md content"
    );

    // Verify content from RULES.md
    assert!(
        prompts
            .rules_context
            .contains("Always use parameterized queries"),
        "Should contain RULES.md content"
    );
    assert!(
        prompts
            .rules_context
            .contains("Never drop tables in production"),
        "Should contain RULES.md content"
    );

    // Verify the prefix instruction is included
    assert!(
        prompts
            .rules_context
            .contains("Determine if the agent action breaks any of the rules"),
        "Should contain rules context prefix"
    );
}

#[test]
fn test_rules_context_rendered_in_user_message() {
    use cupcake_core::watchdog::WatchdogDirConfig;

    let fixtures = fixtures_path().join("rules_context");
    let project_dir = fixtures.join("project");

    // Load config and prompts
    let dir_config = WatchdogDirConfig::load_from_dir(&project_dir).expect("Should load config");
    let rules_context = dir_config.rules_context.as_ref();

    let prompts = WatchdogPrompts::load_with_rules_context(Some(&project_dir), None, rules_context)
        .expect("Should load prompts");

    // Render a user message
    let event = serde_json::json!({
        "hook_event_name": "PreToolUse",
        "tool_name": "Bash",
        "tool_input": {
            "command": "rm -rf /important/data"
        }
    });

    let rendered = prompts.render_user_message(&event);

    // Event should be included
    assert!(rendered.contains("\"tool_name\": \"Bash\""));
    assert!(rendered.contains("rm -rf /important/data"));

    // Rules context should be included
    assert!(rendered.contains("Never delete files without explicit user confirmation"));
    assert!(rendered.contains("=== CLAUDE.md ==="));

    // Placeholders should be replaced
    assert!(!rendered.contains("{{event}}"));
    assert!(!rendered.contains("{{rules_context}}"));
}

#[test]
fn test_rules_context_strict_mode_fails_on_missing_file() {
    use cupcake_core::watchdog::RulesContext;

    let fixtures = fixtures_path().join("rules_context");
    let project_dir = fixtures.join("project");

    // Create a rules context with strict mode (default) and a non-existent file
    let rules_context = RulesContext {
        root_path: "..".to_string(),
        files: vec!["CLAUDE.md".to_string(), "nonexistent.md".to_string()],
        strict: true, // Default behavior
    };

    let result =
        WatchdogPrompts::load_with_rules_context(Some(&project_dir), None, Some(&rules_context));

    assert!(
        result.is_err(),
        "Should fail when strict=true and file is missing"
    );
    let err = result.unwrap_err();
    assert!(
        err.to_string().contains("nonexistent.md"),
        "Error message should mention the missing file"
    );
}

#[test]
fn test_rules_context_non_strict_mode_ignores_missing_file() {
    use cupcake_core::watchdog::RulesContext;

    let fixtures = fixtures_path().join("rules_context");
    let project_dir = fixtures.join("project");

    // Create a rules context with strict=false and a non-existent file
    let rules_context = RulesContext {
        root_path: "..".to_string(),
        files: vec!["CLAUDE.md".to_string(), "nonexistent.md".to_string()],
        strict: false, // Graceful degradation
    };

    let prompts =
        WatchdogPrompts::load_with_rules_context(Some(&project_dir), None, Some(&rules_context))
            .expect("Should load prompts with strict=false");

    // CLAUDE.md should be loaded
    assert!(prompts.rules_context.contains("=== CLAUDE.md ==="));
    assert!(prompts.rules_context.contains("Never delete files"));

    // nonexistent.md should be silently skipped
    assert!(!prompts.rules_context.contains("nonexistent.md"));
}
