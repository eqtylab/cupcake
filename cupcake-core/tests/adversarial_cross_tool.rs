//! Adversarial test suite for TOB-EQTY-LAB-CUPCAKE-2
//! Tests defenses against cross-tool bypass via tool-specific routing

use anyhow::Result;
use cupcake_core::engine::Engine;
use serde_json::json;
use std::fs;
use tempfile::TempDir;

/// Test that protected paths block ALL file modification tools
#[tokio::test]
#[cfg(feature = "deterministic-tests")]
async fn test_cross_tool_protection_coverage() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let cupcake_dir = temp_dir.path().join(".cupcake");
    let policies_dir = cupcake_dir.join("policies");
    let claude_dir = policies_dir.join("claude");
    let system_dir = claude_dir.join("system");
    let builtins_dir = claude_dir.join("builtins");
    let shared_system_dir = cupcake_dir.join("system");

    fs::create_dir_all(&system_dir)?;
    fs::create_dir_all(&builtins_dir)?;
    fs::create_dir_all(&shared_system_dir)?;

    // Write system evaluation policy
    let evaluate_policy = include_str!("fixtures/system_evaluate.rego");
    fs::write(system_dir.join("evaluate.rego"), evaluate_policy)?;

    // Write helper library
    let helpers_commands = include_str!("../../fixtures/system/commands.rego");
    fs::write(shared_system_dir.join("commands.rego"), helpers_commands)?;

    // Use protected_paths builtin with expanded metadata
    let protected_policy = include_str!("../../fixtures/claude/builtins/protected_paths.rego");
    fs::write(builtins_dir.join("protected_paths.rego"), protected_policy)?;

    let rulebook_content = r#"
builtins:
  protected_paths:
    enabled: true
    message: "Critical configuration files are protected"
    paths:
      - "config.production.json"
      - "database.yml"
      - ".env.prod"
"#;
    fs::write(cupcake_dir.join("rulebook.yml"), rulebook_content)?;

    let empty_global = TempDir::new()?;
    let config = cupcake_core::engine::EngineConfig {
        global_config: Some(empty_global.path().to_path_buf()),
        harness: cupcake_core::harness::types::HarnessType::ClaudeCode,
        wasm_max_memory: None,
        opa_path: None,
        debug_routing: false,
    };
    let engine = Engine::new_with_config(temp_dir.path(), config).await?;

    // Test ALL file modification tools are blocked
    let cross_tool_attacks = vec![
        // Write tool
        (
            "Write",
            json!({
                "file_path": "config.production.json",
                "content": "malicious config"
            }),
        ),
        // Edit tool
        (
            "Edit",
            json!({
                "file_path": "database.yml",
                "old_string": "host: localhost",
                "new_string": "host: evil.com"
            }),
        ),
        // MultiEdit tool (if supported)
        (
            "MultiEdit",
            json!({
                "edits": [{
                    "file_path": ".env.prod",
                    "old_string": "API_KEY=secret",
                    "new_string": "API_KEY=compromised"
                }]
            }),
        ),
        // NotebookEdit tool
        (
            "NotebookEdit",
            json!({
                "notebook_path": "config.production.json",
                "cell_id": "cell-1",
                "new_source": "malicious code"
            }),
        ),
        // Bash tool with various write commands
        (
            "Bash",
            json!({
                "command": "echo 'malicious' > config.production.json"
            }),
        ),
        (
            "Bash",
            json!({
                "command": "sed -i 's/old/new/' database.yml"
            }),
        ),
        (
            "Bash",
            json!({
                "command": "vim .env.prod"
            }),
        ),
    ];

    for (tool_name, tool_input) in cross_tool_attacks {
        let event = json!({
            "hook_event_name": "PreToolUse",
            "session_id": "test",
            "transcript_path": "/tmp/transcript.md",
            "cwd": temp_dir.path().to_string_lossy(),
            "tool_name": tool_name,
            "tool_input": tool_input
        });

        let decision = engine.evaluate(&event, None).await?;
        match decision {
            cupcake_core::engine::decision::FinalDecision::Halt { reason, .. } => {
                // Good - all tools are blocked
                assert!(
                    reason.contains("protected"),
                    "Tool {tool_name} should mention protection: {reason}"
                );
            }
            cupcake_core::engine::decision::FinalDecision::Allow { .. } => {
                panic!(
                    "VULNERABILITY: Tool '{tool_name}' bypassed protection! \
                     Cross-tool bypass detected - policy missing required_tools metadata."
                );
            }
            _ => {
                // Other decisions like Ask are OK as long as not Allow
                println!("Tool {tool_name} returned non-Allow decision: {decision:?}");
            }
        }
    }

    Ok(())
}

/// Test rulebook security blocks all tools for .cupcake modifications
#[tokio::test]
#[cfg(feature = "deterministic-tests")]
async fn test_cupcake_protection_cross_tool() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let cupcake_dir = temp_dir.path().join(".cupcake");
    let policies_dir = cupcake_dir.join("policies");
    let claude_dir = policies_dir.join("claude");
    let system_dir = claude_dir.join("system");
    let builtins_dir = claude_dir.join("builtins");
    let shared_system_dir = cupcake_dir.join("system");

    fs::create_dir_all(&system_dir)?;
    fs::create_dir_all(&builtins_dir)?;
    fs::create_dir_all(&shared_system_dir)?;

    let evaluate_policy = include_str!("fixtures/system_evaluate.rego");
    fs::write(system_dir.join("evaluate.rego"), evaluate_policy)?;

    let helpers_commands = include_str!("../../fixtures/system/commands.rego");
    fs::write(shared_system_dir.join("commands.rego"), helpers_commands)?;

    // rulebook_security_guardrails should block ALL tools
    let rulebook_policy =
        include_str!("../../fixtures/claude/builtins/rulebook_security_guardrails.rego");
    fs::write(
        builtins_dir.join("rulebook_security_guardrails.rego"),
        rulebook_policy,
    )?;

    let rulebook_content = r#"
builtins:
  rulebook_security_guardrails:
    enabled: true
"#;
    fs::write(cupcake_dir.join("rulebook.yml"), rulebook_content)?;

    let empty_global = TempDir::new()?;
    let config = cupcake_core::engine::EngineConfig {
        global_config: Some(empty_global.path().to_path_buf()),
        harness: cupcake_core::harness::types::HarnessType::ClaudeCode,
        wasm_max_memory: None,
        opa_path: None,
        debug_routing: false,
    };
    let engine = Engine::new_with_config(temp_dir.path(), config).await?;

    // Test various tools trying to access .cupcake
    let tools_and_inputs = vec![
        (
            "Read",
            json!({
                "file_path": ".cupcake/rulebook.yml"
            }),
        ),
        (
            "Write",
            json!({
                "file_path": ".cupcake/policies/evil.rego",
                "content": "malicious policy"
            }),
        ),
        (
            "Edit",
            json!({
                "file_path": ".cupcake/rulebook.yml",
                "old_string": "enabled: true",
                "new_string": "enabled: false"
            }),
        ),
        (
            "Grep",
            json!({
                "pattern": "secret",
                "path": ".cupcake/"
            }),
        ),
        (
            "Glob",
            json!({
                "pattern": ".cupcake/**/*.rego"
            }),
        ),
        (
            "Bash",
            json!({
                "command": "cat .cupcake/rulebook.yml"
            }),
        ),
        (
            "Bash",
            json!({
                "command": "rm -rf .cupcake"
            }),
        ),
    ];

    for (tool_name, tool_input) in tools_and_inputs {
        let event = json!({
            "hook_event_name": "PreToolUse",
            "session_id": "test",
            "transcript_path": "/tmp/transcript.md",
            "cwd": temp_dir.path().to_string_lossy(),
            "tool_name": tool_name,
            "tool_input": tool_input
        });

        let decision = engine.evaluate(&event, None).await?;
        match decision {
            cupcake_core::engine::decision::FinalDecision::Halt { reason, .. } => {
                assert!(
                    reason.contains("Cupcake") || reason.contains("protected"),
                    "Tool {tool_name} should mention Cupcake protection: {reason}"
                );
            }
            _ => panic!(
                "VULNERABILITY: Tool '{tool_name}' was not blocked from accessing .cupcake! \
                 Got: {decision:?}"
            ),
        }
    }

    Ok(())
}

/// Test that tool-specific policies without wildcard metadata miss other tools
#[tokio::test]
#[cfg(feature = "deterministic-tests")]
async fn test_narrow_policy_misses_other_tools() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let cupcake_dir = temp_dir.path().join(".cupcake");
    let policies_dir = cupcake_dir.join("policies");
    let claude_dir = policies_dir.join("claude");
    let system_dir = claude_dir.join("system");

    fs::create_dir_all(&system_dir)?;

    let evaluate_policy = include_str!("fixtures/system_evaluate.rego");
    fs::write(system_dir.join("evaluate.rego"), evaluate_policy)?;

    // Create a policy that ONLY blocks Bash tool
    let narrow_policy = r#"
# METADATA
# scope: package
# custom:
#   routing:
#     required_events: ["PreToolUse"]
#     required_tools: ["Bash"]  # Only routes to Bash!
package cupcake.policies.narrow

import rego.v1

deny contains decision if {
    input.tool_name == "Bash"
    cmd := input.tool_input.command
    contains(cmd, "sensitive.txt")

    decision := {
        "rule_id": "NARROW-001",
        "reason": "Cannot access sensitive file",
        "severity": "HIGH"
    }
}
"#;
    fs::write(claude_dir.join("narrow.rego"), narrow_policy)?;

    let empty_global = TempDir::new()?;
    let config = cupcake_core::engine::EngineConfig {
        global_config: Some(empty_global.path().to_path_buf()),
        harness: cupcake_core::harness::types::HarnessType::ClaudeCode,
        wasm_max_memory: None,
        opa_path: None,
        debug_routing: false,
    };
    let engine = Engine::new_with_config(temp_dir.path(), config).await?;

    // Bash command IS blocked
    let bash_event = json!({
        "hook_event_name": "PreToolUse",
        "session_id": "test",
        "transcript_path": "/tmp/transcript.md",
        "cwd": temp_dir.path().to_string_lossy(),
        "tool_name": "Bash",
        "tool_input": {
            "command": "cat sensitive.txt"
        }
    });

    let decision = engine.evaluate(&bash_event, None).await?;
    match decision {
        cupcake_core::engine::decision::FinalDecision::Deny { .. } => {
            // Good - Bash is blocked
        }
        _ => panic!("Bash command should be blocked, got: {decision:?}"),
    }

    // But Read tool bypasses the policy!
    let read_event = json!({
        "hook_event_name": "PreToolUse",
        "session_id": "test",
        "transcript_path": "/tmp/transcript.md",
        "cwd": temp_dir.path().to_string_lossy(),
        "tool_name": "Read",
        "tool_input": {
            "file_path": "sensitive.txt"
        }
    });

    let decision = engine.evaluate(&read_event, None).await?;
    match decision {
        cupcake_core::engine::decision::FinalDecision::Allow { .. } => {
            println!(
                "VULNERABILITY DEMONSTRATED: Read tool bypassed the narrow policy! \
                 Policy only had required_tools: [\"Bash\"], missing other tools."
            );
        }
        _ => panic!(
            "This test demonstrates the vulnerability - Read should bypass. Got: {decision:?}"
        ),
    }

    // Similarly, Write tool also bypasses
    let write_event = json!({
        "hook_event_name": "PreToolUse",
        "session_id": "test",
        "transcript_path": "/tmp/transcript.md",
        "cwd": temp_dir.path().to_string_lossy(),
        "tool_name": "Write",
        "tool_input": {
            "file_path": "sensitive.txt",
            "content": "overwritten!"
        }
    });

    let decision = engine.evaluate(&write_event, None).await?;
    match decision {
        cupcake_core::engine::decision::FinalDecision::Allow { .. } => {
            println!("Write tool also bypassed the narrow policy!");
        }
        _ => panic!("Write should also bypass. Got: {decision:?}"),
    }

    Ok(())
}

/// Test that expanded metadata prevents cross-tool bypass
#[tokio::test]
#[cfg(feature = "deterministic-tests")]
async fn test_expanded_metadata_prevents_bypass() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let cupcake_dir = temp_dir.path().join(".cupcake");
    let policies_dir = cupcake_dir.join("policies");
    let claude_dir = policies_dir.join("claude");
    let system_dir = claude_dir.join("system");

    fs::create_dir_all(&system_dir)?;

    let evaluate_policy = include_str!("fixtures/system_evaluate.rego");
    fs::write(system_dir.join("evaluate.rego"), evaluate_policy)?;

    // Create a policy with EXPANDED metadata covering all relevant tools
    let comprehensive_policy = r#"
# METADATA
# scope: package
# custom:
#   routing:
#     required_events: ["PreToolUse"]
#     required_tools: ["Bash", "Read", "Write", "Edit", "Grep", "Glob"]
package cupcake.policies.comprehensive

import rego.v1

deny contains decision if {
    # Any tool trying to access the sensitive file is blocked
    sensitive_access

    decision := {
        "rule_id": "COMPREHENSIVE-001",
        "reason": "Cannot access sensitive file via any tool",
        "severity": "HIGH"
    }
}

sensitive_access if {
    input.tool_name == "Bash"
    contains(input.tool_input.command, "sensitive.txt")
}

sensitive_access if {
    input.tool_name == "Read"
    input.tool_input.file_path == "sensitive.txt"
}

sensitive_access if {
    input.tool_name == "Write"
    input.tool_input.file_path == "sensitive.txt"
}

sensitive_access if {
    input.tool_name == "Edit"
    input.tool_input.file_path == "sensitive.txt"
}

sensitive_access if {
    input.tool_name == "Grep"
    contains(input.tool_input.path, "sensitive.txt")
}

sensitive_access if {
    input.tool_name == "Glob"
    contains(input.tool_input.pattern, "sensitive")
}
"#;
    fs::write(claude_dir.join("comprehensive.rego"), comprehensive_policy)?;

    let empty_global = TempDir::new()?;
    let config = cupcake_core::engine::EngineConfig {
        global_config: Some(empty_global.path().to_path_buf()),
        harness: cupcake_core::harness::types::HarnessType::ClaudeCode,
        wasm_max_memory: None,
        opa_path: None,
        debug_routing: false,
    };
    let engine = Engine::new_with_config(temp_dir.path(), config).await?;

    // Test that ALL tools are now blocked
    let test_cases = vec![
        (
            "Bash",
            json!({
                "command": "cat sensitive.txt"
            }),
        ),
        (
            "Read",
            json!({
                "file_path": "sensitive.txt"
            }),
        ),
        (
            "Write",
            json!({
                "file_path": "sensitive.txt",
                "content": "data"
            }),
        ),
        (
            "Edit",
            json!({
                "file_path": "sensitive.txt",
                "old_string": "old",
                "new_string": "new"
            }),
        ),
        (
            "Grep",
            json!({
                "pattern": "secret",
                "path": "sensitive.txt"
            }),
        ),
        (
            "Glob",
            json!({
                "pattern": "**/sensitive*.txt"
            }),
        ),
    ];

    for (tool_name, tool_input) in test_cases {
        let event = json!({
            "hook_event_name": "PreToolUse",
            "session_id": "test",
            "transcript_path": "/tmp/transcript.md",
            "cwd": temp_dir.path().to_string_lossy(),
            "tool_name": tool_name,
            "tool_input": tool_input
        });

        let decision = engine.evaluate(&event, None).await?;
        match decision {
            cupcake_core::engine::decision::FinalDecision::Deny { reason, .. } => {
                assert_eq!(
                    reason, "Cannot access sensitive file via any tool",
                    "Tool {tool_name} blocked with correct reason"
                );
            }
            _ => panic!(
                "Tool '{tool_name}' should be blocked by comprehensive policy! Got: {decision:?}"
            ),
        }
    }

    Ok(())
}

/// Test wildcard policies (no required_tools) match all tools
#[tokio::test]
#[cfg(feature = "deterministic-tests")]
async fn test_wildcard_policy_matches_all_tools() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let cupcake_dir = temp_dir.path().join(".cupcake");
    let policies_dir = cupcake_dir.join("policies");
    let claude_dir = policies_dir.join("claude");
    let system_dir = claude_dir.join("system");

    fs::create_dir_all(&system_dir)?;

    let evaluate_policy = include_str!("fixtures/system_evaluate.rego");
    fs::write(system_dir.join("evaluate.rego"), evaluate_policy)?;

    // Create a wildcard policy (only required_events, no required_tools)
    let wildcard_policy = r#"
# METADATA
# scope: package
# custom:
#   routing:
#     required_events: ["PreToolUse"]
#     # NO required_tools - matches ALL tools!
package cupcake.policies.wildcard

import rego.v1

deny contains decision if {
    # Check for sensitive pattern in any field
    input_contains_sensitive

    decision := {
        "rule_id": "WILDCARD-001",
        "reason": "Sensitive operation detected",
        "severity": "HIGH"
    }
}

input_contains_sensitive if {
    walk(input, [path, value])
    value == "SENSITIVE_PATTERN"
}
"#;
    fs::write(claude_dir.join("wildcard.rego"), wildcard_policy)?;

    let empty_global = TempDir::new()?;
    let config = cupcake_core::engine::EngineConfig {
        global_config: Some(empty_global.path().to_path_buf()),
        harness: cupcake_core::harness::types::HarnessType::ClaudeCode,
        wasm_max_memory: None,
        opa_path: None,
        debug_routing: false,
    };
    let engine = Engine::new_with_config(temp_dir.path(), config).await?;

    // Test that wildcard policy matches ALL tools
    let tools = vec![
        ("Bash", json!({"command": "SENSITIVE_PATTERN"})),
        ("Read", json!({"file_path": "SENSITIVE_PATTERN"})),
        ("Write", json!({"content": "SENSITIVE_PATTERN"})),
        ("Edit", json!({"old_string": "SENSITIVE_PATTERN"})),
        ("Grep", json!({"pattern": "SENSITIVE_PATTERN"})),
        ("WebFetch", json!({"url": "SENSITIVE_PATTERN"})),
        ("Task", json!({"prompt": "SENSITIVE_PATTERN"})),
    ];

    for (tool_name, tool_input) in tools {
        let event = json!({
            "hook_event_name": "PreToolUse",
            "session_id": "test",
            "transcript_path": "/tmp/transcript.md",
            "cwd": temp_dir.path().to_string_lossy(),
            "tool_name": tool_name,
            "tool_input": tool_input
        });

        let decision = engine.evaluate(&event, None).await?;
        match decision {
            cupcake_core::engine::decision::FinalDecision::Deny { reason, .. } => {
                assert_eq!(
                    reason, "Sensitive operation detected",
                    "Wildcard policy should match tool: {tool_name}"
                );
            }
            _ => panic!(
                "Wildcard policy should match ALL tools! Tool '{tool_name}' was not caught. \
                 Got: {decision:?}"
            ),
        }
    }

    Ok(())
}
