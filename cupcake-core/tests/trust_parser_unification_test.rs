//! Integration test to verify trust system uses the same parser as the engine
//!
//! This test ensures the security fix for the parser divergence issue works correctly.
//! The trust system must see ALL scripts that the engine will execute, including:
//! - Scripts from rulebook.yml
//! - Auto-discovered scripts from signals/ and actions/ directories
//! - Complex action structures like on_any_denial

use anyhow::Result;
use cupcake_core::engine::rulebook::Rulebook;
use std::path::PathBuf;

fn get_fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/trust_parser_test")
}

#[tokio::test]
async fn test_trust_parser_sees_all_scripts() -> Result<()> {
    let fixture_dir = get_fixture_path();
    let rulebook_path = fixture_dir.join("rulebook.yml");
    let signals_dir = fixture_dir.join("signals");
    let actions_dir = fixture_dir.join("actions");

    // Load using the engine's parser with auto-discovery
    let rulebook =
        Rulebook::load_with_conventions(&rulebook_path, &signals_dir, &actions_dir).await?;

    // Verify we found all scripts:
    // From rulebook.yml:
    //   - explicit_signal
    //   - on_any_denial action
    //   - RULE-001 action
    // Auto-discovered:
    //   - auto_signal.sh from signals/
    //   - RULE-002.sh from actions/

    // Check signals (should have 2: explicit + auto-discovered)
    assert_eq!(rulebook.signals.len(), 2, "Should find 2 signals");
    assert!(
        rulebook.signals.contains_key("explicit_signal"),
        "Should have explicit_signal from YAML"
    );
    assert!(
        rulebook.signals.contains_key("auto_signal"),
        "Should have auto_signal from directory"
    );

    // Check on_any_denial actions (should have 1)
    assert_eq!(
        rulebook.actions.on_any_denial.len(),
        1,
        "Should have on_any_denial action"
    );

    // Check rule-specific actions (should have 2: RULE-001 from YAML, RULE-002 auto-discovered)
    assert_eq!(
        rulebook.actions.by_rule_id.len(),
        2,
        "Should find 2 rule-specific action sets"
    );
    assert!(
        rulebook.actions.by_rule_id.contains_key("RULE-001"),
        "Should have RULE-001 from YAML"
    );
    assert!(
        rulebook.actions.by_rule_id.contains_key("RULE-002"),
        "Should have RULE-002 from directory"
    );

    // Verify the auto-discovered script has the correct path
    let auto_signal = rulebook.signals.get("auto_signal").unwrap();
    eprintln!("Auto-discovered signal path: {}", auto_signal.command);

    // On Windows, paths use backslashes
    let expected_suffix = if cfg!(windows) {
        "signals\\auto_signal.sh"
    } else {
        "signals/auto_signal.sh"
    };

    assert!(
        auto_signal.command.ends_with(expected_suffix),
        "Auto-discovered signal should have full path ending with '{expected_suffix}', got: '{}'",
        auto_signal.command
    );

    Ok(())
}

#[tokio::test]
async fn test_trust_parser_handles_missing_directories() -> Result<()> {
    let fixture_dir = get_fixture_path();
    let rulebook_path = fixture_dir.join("rulebook.yml");
    let fake_signals_dir = fixture_dir.join("nonexistent_signals");
    let fake_actions_dir = fixture_dir.join("nonexistent_actions");

    // Should not fail even if directories don't exist
    let rulebook =
        Rulebook::load_with_conventions(&rulebook_path, &fake_signals_dir, &fake_actions_dir)
            .await?;

    // Should still load the explicit entries from YAML
    assert!(
        rulebook.signals.contains_key("explicit_signal"),
        "Should have explicit_signal from YAML"
    );
    assert_eq!(
        rulebook.actions.on_any_denial.len(),
        1,
        "Should have on_any_denial from YAML"
    );

    Ok(())
}

#[test]
fn test_fixture_files_exist() {
    let fixture_dir = get_fixture_path();

    // Verify our test fixtures are in place
    assert!(
        fixture_dir.join("rulebook.yml").exists(),
        "rulebook.yml fixture missing"
    );
    assert!(
        fixture_dir.join("signals/auto_signal.sh").exists(),
        "auto_signal.sh fixture missing"
    );
    assert!(
        fixture_dir.join("actions/RULE-002.sh").exists(),
        "RULE-002.sh fixture missing"
    );
}
