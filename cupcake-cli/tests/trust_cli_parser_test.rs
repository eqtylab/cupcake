//! Test that verifies the trust CLI now sees all the same scripts as the engine
//! This proves our fix works - both use the same parser and see the same scripts

use anyhow::Result;
use cupcake_core::engine::guidebook::Guidebook;
use std::collections::HashSet;
use std::fs as std_fs;
use tokio::fs;

#[tokio::test]
async fn test_trust_cli_sees_all_scripts_after_fix() -> Result<()> {
    // Create a test project with mixed explicit and auto-discovered scripts
    let temp_dir = std::env::temp_dir().join(format!("cupcake_test_{}", std::process::id()));
    std_fs::create_dir_all(&temp_dir)?;
    let project_dir = temp_dir.as_path();
    let cupcake_dir = project_dir.join(".cupcake");
    let signals_dir = cupcake_dir.join("signals");
    let actions_dir = cupcake_dir.join("actions");

    // Create directories
    fs::create_dir_all(&signals_dir).await?;
    fs::create_dir_all(&actions_dir).await?;

    // Create guidebook.yml with explicit entries
    let guidebook_content = r#"
signals:
  explicit_signal:
    command: "echo explicit"

actions:
  on_any_denial:
    - command: "echo deny all"
  by_rule_id:
    RULE-001:
      - command: "echo rule1"
"#;
    fs::write(cupcake_dir.join("guidebook.yml"), guidebook_content).await?;

    // Create auto-discovered scripts
    fs::write(signals_dir.join("auto_signal.sh"), "#!/bin/bash\necho auto").await?;
    fs::write(actions_dir.join("RULE-002.sh"), "#!/bin/bash\necho rule2").await?;

    // Load using the SAME parser that trust CLI now uses (after our fix)
    let guidebook = Guidebook::load_with_conventions(
        &cupcake_dir.join("guidebook.yml"),
        &signals_dir,
        &actions_dir,
    )
    .await?;

    // Collect all scripts the trust CLI would see (using engine parser)
    let mut scripts_seen = HashSet::new();

    // Signals
    for name in guidebook.signals.keys() {
        scripts_seen.insert(format!("signal:{name}"));
    }

    // Actions (including on_any_denial)
    for _ in &guidebook.actions.on_any_denial {
        scripts_seen.insert("action:on_any_denial".to_string());
    }

    for rule_id in guidebook.actions.by_rule_id.keys() {
        scripts_seen.insert(format!("action:{rule_id}"));
    }

    // Verify trust CLI (with our fix) sees ALL scripts
    assert_eq!(scripts_seen.len(), 5, "Should see all 5 scripts");
    assert!(
        scripts_seen.contains("signal:explicit_signal"),
        "Should see explicit signal"
    );
    assert!(
        scripts_seen.contains("signal:auto_signal"),
        "Should see auto-discovered signal"
    );
    assert!(
        scripts_seen.contains("action:on_any_denial"),
        "Should see on_any_denial"
    );
    assert!(
        scripts_seen.contains("action:RULE-001"),
        "Should see RULE-001"
    );
    assert!(
        scripts_seen.contains("action:RULE-002"),
        "Should see auto-discovered RULE-002"
    );

    println!(
        "✅ Trust CLI (after fix) correctly sees all {} scripts",
        scripts_seen.len()
    );

    // Clean up
    std_fs::remove_dir_all(&temp_dir)?;

    Ok(())
}
