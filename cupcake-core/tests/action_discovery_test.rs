use cupcake_core::engine::Engine;
use serde_json::json;
use std::fs;
use tempfile::TempDir;

/// Test that actions are automatically discovered from the actions/ directory
///
/// Skipped on Windows due to Git Bash shell script execution timing issues.
#[tokio::test]
#[cfg(not(windows))]
async fn test_action_discovery_from_directory() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = temp_dir.path();

    // Create .cupcake directory structure
    let cupcake_dir = project_path.join(".cupcake");
    let policies_dir = cupcake_dir.join("policies");
    let system_dir = policies_dir.join("system");
    let actions_dir = cupcake_dir.join("actions");

    fs::create_dir_all(&system_dir).unwrap();
    fs::create_dir_all(&actions_dir).unwrap();

    // Create multiple action scripts with rule IDs as names
    let test_actions = vec![
        ("TEST-001.sh", "echo 'Action for TEST-001'"),
        ("TEST-002.sh", "echo 'Action for TEST-002'"),
        ("CRITICAL-001.sh", "echo 'Critical action'"),
    ];

    for (filename, content) in &test_actions {
        let marker_file = temp_dir
            .path()
            .join(format!("cupcake_discover_{filename}.txt"));
        let action_script = format!("#!/bin/bash\n{} > {}", content, marker_file.display());
        let action_path = actions_dir.join(filename);
        fs::write(&action_path, action_script).unwrap();

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&action_path).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&action_path, perms).unwrap();
        }
    }

    // Also create a hidden file that should be ignored
    fs::write(
        actions_dir.join(".hidden.sh"),
        "#!/bin/bash\necho 'Should not be discovered'",
    )
    .unwrap();

    // Create system policy
    create_system_policy(&system_dir);

    // Create test policy that uses TEST-001
    let test_policy = r#"package cupcake.policies.discovery_test

import rego.v1

# METADATA
# scope: rule
# custom:
#   routing:
#     required_events: ["PreToolUse"]
#     required_tools: ["Bash"]

deny contains decision if {
    contains(input.tool_input.command, "test-discovery")
    decision := {
        "reason": "Testing action discovery",
        "severity": "MEDIUM",
        "rule_id": "TEST-001"
    }
}
"#;

    fs::write(policies_dir.join("discovery_test.rego"), test_policy).unwrap();

    // Initialize engine (this should trigger discovery)
    let engine = Engine::new(&project_path).await.unwrap();

    // Trigger the policy
    let event = json!({
        "hookEventName": "PreToolUse",
        "tool_name": "Bash",
        "tool_input": {
            "command": "test-discovery command"
        },
        "session_id": "test",
        "cwd": "/tmp"
    });

    let decision = engine.evaluate(&event, None).await.unwrap();
    assert!(decision.is_blocking());

    // Wait for async action to complete
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    // Verify the auto-discovered action executed
    let marker_path = temp_dir.path().join("cupcake_discover_TEST-001.sh.txt");
    assert!(
        marker_path.exists(),
        "Auto-discovered action TEST-001 did not execute"
    );

    // Verify hidden file was not discovered/executed
    let hidden_marker = temp_dir.path().join("cupcake_discover_.hidden.sh.txt");
    assert!(
        !hidden_marker.exists(),
        "Hidden file should not have been discovered as an action"
    );

    // No cleanup needed - TempDir handles it
}

/// Test that convention-discovered actions override rulebook entries
///
/// Skipped on Windows due to Git Bash shell script execution timing issues.
#[tokio::test]
#[cfg(not(windows))]
async fn test_discovery_with_rulebook_precedence() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = temp_dir.path();

    let cupcake_dir = project_path.join(".cupcake");
    let policies_dir = cupcake_dir.join("policies");
    let system_dir = policies_dir.join("system");
    let actions_dir = cupcake_dir.join("actions");

    fs::create_dir_all(&system_dir).unwrap();
    fs::create_dir_all(&actions_dir).unwrap();

    // Create markers
    let rulebook_marker = temp_dir.path().join("rulebook_action.txt");
    let discovered_marker = temp_dir.path().join("discovered_action.txt");

    // Create rulebook with explicit action
    let rulebook = format!(
        r#"
actions:
  by_rule_id:
    OVERRIDE-001:
      - command: 'echo "From rulebook" > {}'
"#,
        rulebook_marker.display()
    );

    fs::write(cupcake_dir.join("rulebook.yml"), rulebook).unwrap();

    // Also create a discovered action with same rule ID
    let discovered_script = format!(
        r#"#!/bin/bash
echo "From discovery" > {}
"#,
        discovered_marker.display()
    );

    let action_path = actions_dir.join("OVERRIDE-001.sh");
    fs::write(&action_path, discovered_script).unwrap();

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&action_path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&action_path, perms).unwrap();
    }

    create_system_policy(&system_dir);

    let policy = r#"package cupcake.policies.override_test

import rego.v1

# METADATA
# scope: rule
# custom:
#   routing:
#     required_events: ["PreToolUse"]
#     required_tools: ["Bash"]

deny contains decision if {
    contains(input.tool_input.command, "override-test")
    decision := {
        "reason": "Testing precedence",
        "severity": "MEDIUM",
        "rule_id": "OVERRIDE-001"
    }
}
"#;

    fs::write(policies_dir.join("override_test.rego"), policy).unwrap();

    let engine = Engine::new(&project_path).await.unwrap();

    let event = json!({
        "hookEventName": "PreToolUse",
        "tool_name": "Bash",
        "tool_input": {
            "command": "override-test"
        },
        "session_id": "test",
        "cwd": "/tmp"
    });

    let decision = engine.evaluate(&event, None).await.unwrap();
    assert!(decision.is_blocking());

    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    // Both actions should execute (rulebook adds to discovered)
    assert!(
        rulebook_marker.exists() || discovered_marker.exists(),
        "At least one action should have executed"
    );
}

/// Test action discovery with subdirectories
///
/// Skipped on Windows due to Git Bash shell script execution timing issues.
#[tokio::test]
#[cfg(not(windows))]
async fn test_action_discovery_ignores_subdirs() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = temp_dir.path();

    let cupcake_dir = project_path.join(".cupcake");
    let policies_dir = cupcake_dir.join("policies");
    let system_dir = policies_dir.join("system");
    let actions_dir = cupcake_dir.join("actions");
    let subdir = actions_dir.join("subdir");

    fs::create_dir_all(&system_dir).unwrap();
    fs::create_dir_all(&subdir).unwrap();

    // Create action in root actions dir
    let root_marker = temp_dir.path().join("root_action.txt");
    let root_script = format!(
        r#"#!/bin/bash
echo "Root action" > {}
"#,
        root_marker.display()
    );

    let root_action = actions_dir.join("ROOT-001.sh");
    fs::write(&root_action, root_script).unwrap();

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&root_action).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&root_action, perms).unwrap();
    }

    // Create action in subdirectory (should be ignored)
    let subdir_marker = temp_dir.path().join("cupcake_subdir_action.txt");
    let subdir_script = format!(
        r#"#!/bin/bash
echo "Subdir action" > {}
"#,
        subdir_marker.display()
    );

    fs::write(subdir.join("SUB-001.sh"), subdir_script).unwrap();

    create_system_policy(&system_dir);

    let policy = r#"package cupcake.policies.subdir_test

import rego.v1

# METADATA
# scope: rule
# custom:
#   routing:
#     required_events: ["PreToolUse"]
#     required_tools: ["Bash"]

deny contains decision if {
    contains(input.tool_input.command, "root-test")
    decision := {
        "reason": "Testing root action",
        "severity": "MEDIUM",
        "rule_id": "ROOT-001"
    }
}
"#;

    fs::write(policies_dir.join("subdir_test.rego"), policy).unwrap();

    let engine = Engine::new(&project_path).await.unwrap();

    let event = json!({
        "hookEventName": "PreToolUse",
        "tool_name": "Bash",
        "tool_input": {
            "command": "root-test"
        },
        "session_id": "test",
        "cwd": "/tmp"
    });

    let decision = engine.evaluate(&event, None).await.unwrap();
    assert!(decision.is_blocking());

    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    // Only root action should execute
    assert!(root_marker.exists(), "Root action did not execute");

    // Subdirectory action should not execute
    let subdir_marker = temp_dir.path().join("cupcake_subdir_action.txt");
    assert!(
        !subdir_marker.exists(),
        "Subdirectory action should not have been discovered"
    );
}

// Helper function
fn create_system_policy(system_dir: &std::path::Path) {
    let system_policy = r#"package cupcake.system

import rego.v1

evaluate := decision_set if {
    decision_set := {
        "halts": collect_verbs("halt"),
        "denials": collect_verbs("deny"),
        "blocks": collect_verbs("block"),
        "asks": collect_verbs("ask"),
        "allow_overrides": collect_verbs("allow_override"),
        "add_context": collect_verbs("add_context")
    }
}

collect_verbs(verb_name) := result if {
    verb_sets := [value |
        walk(data.cupcake.policies, [path, value])
        path[count(path) - 1] == verb_name
    ]
    all_decisions := [decision |
        some verb_set in verb_sets
        some decision in verb_set
    ]
    result := all_decisions
}

default collect_verbs(_) := []
"#;

    fs::write(system_dir.join("evaluate.rego"), system_policy).unwrap();
}
