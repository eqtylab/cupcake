use serde_json::Value;
use std::fs;
use tempfile::TempDir;

/// Helper to create a test directory with minimal Cupcake structure
fn setup_test_dir(_event_name: &str) -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    let project_path = temp_dir.path();

    // Create .cupcake directory structure
    let cupcake_dir = project_path.join(".cupcake");
    let policies_dir = cupcake_dir.join("policies");
    let system_dir = policies_dir.join("system");
    let signals_dir = cupcake_dir.join("signals");
    let actions_dir = cupcake_dir.join("actions");

    fs::create_dir_all(&system_dir).unwrap();
    fs::create_dir_all(&signals_dir).unwrap();
    fs::create_dir_all(&actions_dir).unwrap();

    // Create minimal guidebook with no builtins
    let guidebook = r#"
version: "1.0"
builtins: {}
signals: {}
actions: {}
"#;
    fs::write(cupcake_dir.join("guidebook.yml"), guidebook).unwrap();

    // Create system evaluate policy (required for compilation)
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

    temp_dir
}

/// Helper to write a test policy for a specific event
fn write_test_policy(
    policies_dir: &std::path::Path,
    package_name: &str,
    required_events: Vec<&str>,
    required_tools: Option<Vec<&str>>,
) {
    let mut metadata = format!(
        r#"# METADATA
# scope: package
# custom:
#   routing:
#     required_events: {required_events:?}"#
    );

    if let Some(tools) = required_tools {
        metadata.push_str(&format!("\n#     required_tools: {tools:?}"));
    }

    let policy = format!(
        r#"{}
package cupcake.policies.{}

import rego.v1

deny contains decision if {{
    input.hook_event_name == "{}"
    decision := {{
        "reason": "test policy for {}",
        "severity": "LOW",
        "rule_id": "TEST-{}"
    }}
}}"#,
        metadata,
        package_name,
        required_events[0],
        required_events[0],
        required_events[0].to_uppercase()
    );

    let policy_file = policies_dir.join(format!("{package_name}.rego"));
    fs::write(policy_file, policy).unwrap();
}

/// Helper to get Claude CLI path
fn get_claude_path() -> String {
    // Try environment variable first (for CI/custom installs)
    if let Ok(path) = std::env::var("CLAUDE_CLI_PATH") {
        eprintln!("[DEBUG] CLAUDE_CLI_PATH env var: {path}");
        if std::path::Path::new(&path).exists() {
            eprintln!("[DEBUG] Claude CLI found at env path: {path}");
            return path;
        } else {
            eprintln!("[DEBUG] Claude CLI path from env doesn't exist: {path}");
        }
    } else {
        eprintln!("[DEBUG] CLAUDE_CLI_PATH env var not set");
    }

    // Default to HOME-based path
    let home = std::env::var("HOME").unwrap_or_else(|_| {
        eprintln!("[DEBUG] HOME env var not set, trying USERPROFILE (Windows)");
        std::env::var("USERPROFILE").expect("Neither HOME nor USERPROFILE set")
    });
    let claude_path = format!("{home}/.claude/local/claude");

    if !std::path::Path::new(&claude_path).exists() {
        panic!(
            "Claude CLI not found at {claude_path}. Set CLAUDE_CLI_PATH env var or install Claude."
        );
    }

    eprintln!("[DEBUG] Claude CLI found at default path: {claude_path}");
    claude_path
}

/// Helper to verify routing map contains expected entries
async fn verify_routing(project_path: &std::path::Path, expected_key: &str, expected_policy: &str) {
    // Create .claude directory
    let claude_dir = project_path.join(".claude");
    fs::create_dir_all(&claude_dir).unwrap();

    // Determine the command to use - prefer built binary in CI
    let command = if std::env::var("CI").is_ok() {
        // In CI, use the built binary directly
        let target_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("target")
            .join("release")
            .join(if cfg!(windows) {
                "cupcake.exe"
            } else {
                "cupcake"
            });

        if target_dir.exists() {
            eprintln!("[DEBUG] Using built binary in CI: {target_dir:?}");
            format!("{} eval", target_dir.display())
        } else {
            // Fallback to debug build
            let debug_target = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .unwrap()
                .join("target")
                .join("debug")
                .join(if cfg!(windows) {
                    "cupcake.exe"
                } else {
                    "cupcake"
                });

            eprintln!(
                "[DEBUG] Release binary not found, trying debug: {debug_target:?}"
            );
            format!("{} eval", debug_target.display())
        }
    } else {
        // Local development - use cargo run
        let cargo_manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("Cargo.toml");
        eprintln!("[DEBUG] Using cargo run for local development");
        format!(
            "cargo run --manifest-path {} -- eval",
            cargo_manifest.display()
        )
    };

    eprintln!("[DEBUG] Hook command: {command}");

    // Create settings.json with UserPromptSubmit hook to trigger on "hello world"
    let settings = format!(
        r#"{{
  "hooks": {{
    "UserPromptSubmit": [
      {{
        "hooks": [
          {{
            "type": "command",
            "command": "{command}",
            "timeout": 120,
            "env": {{
              "CUPCAKE_DEBUG_ROUTING": "1",
              "RUST_LOG": "info"
            }}
          }}
        ]
      }}
    ]
  }}
}}"#
    );
    fs::write(claude_dir.join("settings.json"), settings).unwrap();

    // Get claude CLI path
    let claude_path = get_claude_path();
    eprintln!("[DEBUG] Running claude command from: {project_path:?}");
    eprintln!(
        "[DEBUG] Claude command: {claude_path} -p 'hello world' --model sonnet"
    );

    let output = std::process::Command::new(&claude_path)
        .args(["-p", "hello world", "--model", "sonnet"])
        .current_dir(project_path)
        .env("CUPCAKE_DEBUG_ROUTING", "1") // This adds to inherited env
        .output()
        .expect("Failed to execute claude command");

    eprintln!("[DEBUG] Claude exit status: {:?}", output.status.code());
    eprintln!(
        "[DEBUG] Claude stdout length: {} bytes",
        output.stdout.len()
    );
    eprintln!(
        "[DEBUG] Claude stderr length: {} bytes",
        output.stderr.len()
    );

    if !output.status.success() {
        panic!(
            "Claude command failed with status: {:?}\nSTDOUT:\n{}\nSTDERR:\n{}",
            output.status.code(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    // Wait for hooks to complete and files to be written
    eprintln!("[DEBUG] Waiting 2 seconds for hooks to complete...");
    std::thread::sleep(std::time::Duration::from_secs(2));

    // Find and read the routing map JSON
    let debug_dir = project_path.join(".cupcake/debug/routing");
    eprintln!("[DEBUG] Looking for debug directory: {debug_dir:?}");
    eprintln!("[DEBUG] Debug dir exists: {}", debug_dir.exists());

    if debug_dir.exists() {
        eprintln!("[DEBUG] Contents of .cupcake/debug/routing:");
        if let Ok(entries) = fs::read_dir(&debug_dir) {
            for entry in entries.flatten() {
                eprintln!("  - {:?}", entry.path());
            }
        }
    } else {
        eprintln!("[DEBUG] Checking parent directories:");
        eprintln!(
            "  .cupcake exists: {}",
            project_path.join(".cupcake").exists()
        );
        eprintln!(
            "  .cupcake/debug exists: {}",
            project_path.join(".cupcake/debug").exists()
        );
    }

    let entries = fs::read_dir(&debug_dir)
        .expect("Debug directory should exist")
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .starts_with("routing_map_")
                && e.path().extension() == Some(std::ffi::OsStr::new("json"))
        })
        .collect::<Vec<_>>();

    assert!(
        !entries.is_empty(),
        "Should have generated routing map JSON"
    );

    let json_path = entries[0].path();
    let json_content = fs::read_to_string(&json_path).unwrap();
    let routing_data: Value = serde_json::from_str(&json_content).unwrap();

    // Verify the expected routing key exists
    let routing_entries = &routing_data["project"]["routing_entries"];
    assert!(
        routing_entries.get(expected_key).is_some(),
        "Expected routing key '{}' not found in map. Available keys: {}",
        expected_key,
        routing_entries
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect::<Vec<_>>()
            .join(", ")
    );

    // Verify the policy is mapped to that key
    let policies = routing_entries[expected_key].as_array().unwrap();
    let policy_names: Vec<String> = policies
        .iter()
        .map(|p| p["package_name"].as_str().unwrap().to_string())
        .collect();

    assert!(
        policy_names.contains(&format!("cupcake.policies.{expected_policy}")),
        "Expected policy 'cupcake.policies.{expected_policy}' not found in routing for key '{expected_key}'. Found: {policy_names:?}"
    );
}

#[tokio::test]
async fn test_pretooluse_routing() {
    let temp_dir = setup_test_dir("pretooluse");
    let policies_dir = temp_dir.path().join(".cupcake/policies");

    write_test_policy(
        &policies_dir,
        "test_pretooluse",
        vec!["PreToolUse"],
        Some(vec!["Bash"]),
    );

    verify_routing(temp_dir.path(), "PreToolUse:Bash", "test_pretooluse").await;
}

#[tokio::test]
async fn test_posttooluse_routing() {
    let temp_dir = setup_test_dir("posttooluse");
    let policies_dir = temp_dir.path().join(".cupcake/policies");

    write_test_policy(
        &policies_dir,
        "test_posttooluse",
        vec!["PostToolUse"],
        Some(vec!["Write"]),
    );

    verify_routing(temp_dir.path(), "PostToolUse:Write", "test_posttooluse").await;
}

#[tokio::test]
async fn test_userpromptsubmit_routing() {
    let temp_dir = setup_test_dir("userpromptsubmit");
    let policies_dir = temp_dir.path().join(".cupcake/policies");

    write_test_policy(
        &policies_dir,
        "test_userpromptsubmit",
        vec!["UserPromptSubmit"],
        None,
    );

    verify_routing(temp_dir.path(), "UserPromptSubmit", "test_userpromptsubmit").await;
}

#[tokio::test]
async fn test_sessionstart_routing() {
    let temp_dir = setup_test_dir("sessionstart");
    let policies_dir = temp_dir.path().join(".cupcake/policies");

    write_test_policy(
        &policies_dir,
        "test_sessionstart",
        vec!["SessionStart"],
        None,
    );

    verify_routing(temp_dir.path(), "SessionStart", "test_sessionstart").await;
}

#[tokio::test]
async fn test_notification_routing() {
    let temp_dir = setup_test_dir("notification");
    let policies_dir = temp_dir.path().join(".cupcake/policies");

    write_test_policy(
        &policies_dir,
        "test_notification",
        vec!["Notification"],
        None,
    );

    verify_routing(temp_dir.path(), "Notification", "test_notification").await;
}

#[tokio::test]
async fn test_stop_routing() {
    let temp_dir = setup_test_dir("stop");
    let policies_dir = temp_dir.path().join(".cupcake/policies");

    write_test_policy(&policies_dir, "test_stop", vec!["Stop"], None);

    verify_routing(temp_dir.path(), "Stop", "test_stop").await;
}

#[tokio::test]
async fn test_subagentstop_routing() {
    let temp_dir = setup_test_dir("subagentstop");
    let policies_dir = temp_dir.path().join(".cupcake/policies");

    write_test_policy(
        &policies_dir,
        "test_subagentstop",
        vec!["SubagentStop"],
        None,
    );

    verify_routing(temp_dir.path(), "SubagentStop", "test_subagentstop").await;
}

#[tokio::test]
async fn test_precompact_routing() {
    let temp_dir = setup_test_dir("precompact");
    let policies_dir = temp_dir.path().join(".cupcake/policies");

    write_test_policy(&policies_dir, "test_precompact", vec!["PreCompact"], None);

    verify_routing(temp_dir.path(), "PreCompact", "test_precompact").await;
}

#[tokio::test]
async fn test_wildcard_policy_routing() {
    use std::time::Instant;

    let test_start = Instant::now();
    eprintln!("[TIMING] Test started");

    let temp_dir = setup_test_dir("wildcard");
    let policies_dir = temp_dir.path().join(".cupcake/policies");
    eprintln!("[TIMING] Setup complete: {:?}", test_start.elapsed());

    // Create a wildcard policy (empty tools means all tools)
    write_test_policy(
        &policies_dir,
        "test_wildcard",
        vec!["PreToolUse"],
        Some(vec![]), // Empty tools = wildcard
    );

    // Also create a specific tool policy to test coexistence
    write_test_policy(
        &policies_dir,
        "test_specific",
        vec!["PreToolUse"],
        Some(vec!["Bash"]),
    );
    eprintln!("[TIMING] Policies written: {:?}", test_start.elapsed());

    // Create .claude directory and settings.json
    let claude_dir = temp_dir.path().join(".claude");
    fs::create_dir_all(&claude_dir).unwrap();

    // Determine the command to use - prefer built binary in CI
    let command = if std::env::var("CI").is_ok() {
        // In CI, use the built binary directly
        let target_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("target")
            .join("release")
            .join(if cfg!(windows) {
                "cupcake.exe"
            } else {
                "cupcake"
            });

        if target_dir.exists() {
            eprintln!("[DEBUG] Using built binary in CI: {target_dir:?}");
            format!("{} eval", target_dir.display())
        } else {
            // Fallback to debug build
            let debug_target = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .unwrap()
                .join("target")
                .join("debug")
                .join(if cfg!(windows) {
                    "cupcake.exe"
                } else {
                    "cupcake"
                });

            eprintln!(
                "[DEBUG] Release binary not found, trying debug: {debug_target:?}"
            );
            format!("{} eval", debug_target.display())
        }
    } else {
        // Local development - use cargo run
        let cargo_manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("Cargo.toml");
        format!(
            "cargo run --manifest-path {} -- eval",
            cargo_manifest.display()
        )
    };

    let settings = format!(
        r#"{{
  "hooks": {{
    "UserPromptSubmit": [
      {{
        "hooks": [
          {{
            "type": "command",
            "command": "{command}",
            "timeout": 120,
            "env": {{
              "CUPCAKE_DEBUG_ROUTING": "1",
              "RUST_LOG": "info"
            }}
          }}
        ]
      }}
    ]
  }}
}}"#
    );
    fs::write(claude_dir.join("settings.json"), settings).unwrap();
    eprintln!("[TIMING] Settings written: {:?}", test_start.elapsed());

    // Get claude CLI path
    let claude_path = get_claude_path();
    eprintln!("[TIMING] Starting Claude execution with sonnet model...");
    let claude_start = Instant::now();

    let output = std::process::Command::new(claude_path)
        .args(["-p", "hello world", "--model", "sonnet"]) // Changed to sonnet
        .current_dir(temp_dir.path())
        .env("CUPCAKE_DEBUG_ROUTING", "1")
        .output()
        .expect("Failed to execute claude command");

    eprintln!(
        "[TIMING] Claude execution complete: {:?} (total: {:?})",
        claude_start.elapsed(),
        test_start.elapsed()
    );

    if !output.status.success() {
        panic!(
            "Claude command failed: {}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    // Read routing map
    eprintln!("[TIMING] Reading debug files...");
    let read_start = Instant::now();

    let debug_dir = temp_dir.path().join(".cupcake/debug/routing");
    let entries = fs::read_dir(&debug_dir)
        .expect("Debug directory should exist")
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .starts_with("routing_map_")
                && e.path().extension() == Some(std::ffi::OsStr::new("json"))
        })
        .collect::<Vec<_>>();

    let json_path = entries[0].path();
    let json_content = fs::read_to_string(&json_path).unwrap();
    let routing_data: Value = serde_json::from_str(&json_content).unwrap();

    eprintln!(
        "[TIMING] Debug files read: {:?} (total: {:?})",
        read_start.elapsed(),
        test_start.elapsed()
    );

    let routing_entries = &routing_data["project"]["routing_entries"];

    // Verify wildcard appears in event-only key
    eprintln!("[TIMING] Starting assertions...");
    let assert_start = Instant::now();

    let pretooluse_policies = routing_entries["PreToolUse"].as_array().unwrap();
    let wildcard_found = pretooluse_policies
        .iter()
        .any(|p| p["package_name"].as_str().unwrap() == "cupcake.policies.test_wildcard");
    assert!(
        wildcard_found,
        "Wildcard policy should appear in PreToolUse key"
    );

    // Verify both policies appear in specific tool key
    let bash_policies = routing_entries["PreToolUse:Bash"].as_array().unwrap();
    let policy_names: Vec<String> = bash_policies
        .iter()
        .map(|p| p["package_name"].as_str().unwrap().to_string())
        .collect();

    assert!(
        policy_names.contains(&"cupcake.policies.test_specific".to_string()),
        "Specific policy should be in PreToolUse:Bash"
    );
    assert!(
        policy_names.contains(&"cupcake.policies.test_wildcard".to_string()),
        "Wildcard policy should also be in PreToolUse:Bash"
    );

    eprintln!("[TIMING] Assertions complete: {:?}", assert_start.elapsed());
    eprintln!(
        "[TIMING] Test complete - Total time: {:?}",
        test_start.elapsed()
    );
}

#[tokio::test]
async fn test_multiple_events_routing() {
    let temp_dir = setup_test_dir("multi_event");
    let policies_dir = temp_dir.path().join(".cupcake/policies");

    // Create a policy that handles multiple events
    let policy = r#"# METADATA
# scope: package
# custom:
#   routing:
#     required_events: ["PreToolUse", "PostToolUse"]
#     required_tools: ["Edit"]
package cupcake.policies.test_multi

import rego.v1

deny contains decision if {
    input.hook_event_name in {"PreToolUse", "PostToolUse"}
    input.tool_name == "Edit"
    decision := {
        "reason": "test multi-event policy",
        "severity": "LOW",
        "rule_id": "TEST-MULTI"
    }
}"#;

    fs::write(policies_dir.join("test_multi.rego"), policy).unwrap();

    // Create .claude directory and settings.json
    let claude_dir = temp_dir.path().join(".claude");
    fs::create_dir_all(&claude_dir).unwrap();

    // Determine the command to use - prefer built binary in CI
    let command = if std::env::var("CI").is_ok() {
        // In CI, use the built binary directly
        let target_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("target")
            .join("release")
            .join(if cfg!(windows) {
                "cupcake.exe"
            } else {
                "cupcake"
            });

        if target_dir.exists() {
            eprintln!("[DEBUG] Using built binary in CI: {target_dir:?}");
            format!("{} eval", target_dir.display())
        } else {
            // Fallback to debug build
            let debug_target = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .unwrap()
                .join("target")
                .join("debug")
                .join(if cfg!(windows) {
                    "cupcake.exe"
                } else {
                    "cupcake"
                });

            eprintln!(
                "[DEBUG] Release binary not found, trying debug: {debug_target:?}"
            );
            format!("{} eval", debug_target.display())
        }
    } else {
        // Local development - use cargo run
        let cargo_manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("Cargo.toml");
        format!(
            "cargo run --manifest-path {} -- eval",
            cargo_manifest.display()
        )
    };

    let settings = format!(
        r#"{{
  "hooks": {{
    "UserPromptSubmit": [
      {{
        "hooks": [
          {{
            "type": "command",
            "command": "{command}",
            "timeout": 120,
            "env": {{
              "CUPCAKE_DEBUG_ROUTING": "1",
              "RUST_LOG": "info"
            }}
          }}
        ]
      }}
    ]
  }}
}}"#
    );
    fs::write(claude_dir.join("settings.json"), settings).unwrap();

    // Get claude CLI path
    let claude_path = get_claude_path();
    let output = std::process::Command::new(claude_path)
        .args(["-p", "hello world", "--model", "sonnet"])
        .current_dir(temp_dir.path())
        .env("CUPCAKE_DEBUG_ROUTING", "1")
        .output()
        .expect("Failed to execute claude command");

    if !output.status.success() {
        panic!(
            "Claude command failed: {}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    // Read routing map
    let debug_dir = temp_dir.path().join(".cupcake/debug/routing");
    let entries = fs::read_dir(&debug_dir)
        .expect("Debug directory should exist")
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .starts_with("routing_map_")
                && e.path().extension() == Some(std::ffi::OsStr::new("json"))
        })
        .collect::<Vec<_>>();

    let json_path = entries[0].path();
    let json_content = fs::read_to_string(&json_path).unwrap();
    let routing_data: Value = serde_json::from_str(&json_content).unwrap();

    let routing_entries = &routing_data["project"]["routing_entries"];

    // Verify policy appears in both routing keys
    for key in &["PreToolUse:Edit", "PostToolUse:Edit"] {
        let policies = routing_entries[key]
            .as_array()
            .unwrap_or_else(|| panic!("Key {key} should exist"));
        let found = policies
            .iter()
            .any(|p| p["package_name"].as_str().unwrap() == "cupcake.policies.test_multi");
        assert!(found, "Multi-event policy should appear in {key} key");
    }
}
