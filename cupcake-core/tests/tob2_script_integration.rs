//! Integration test for TOB-2: Script execution bypass vulnerability
//!
//! This test creates real policies and real script files to demonstrate
//! that our script inspection feature properly fixes the TOB-2 vulnerability.

use anyhow::Result;
use cupcake_core::engine::Engine;
use cupcake_core::harness::types::HarnessType;
use cupcake_core::preprocessing::{preprocess_input, PreprocessConfig};
use serde_json::json;
use std::fs;
use tempfile::TempDir;

/// Full integration test: Policy + Real Script + Engine Evaluation
#[tokio::test]
#[cfg(feature = "deterministic-tests")]
async fn test_tob2_script_bypass_is_fixed() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let cupcake_dir = temp_dir.path().join(".cupcake");
    let policies_dir = cupcake_dir.join("policies");
    let claude_dir = policies_dir.join("claude");
    let system_dir = claude_dir.join("system");

    fs::create_dir_all(&system_dir)?;

    // Step 1: Write the system evaluation policy
    let evaluate_policy = include_str!("fixtures/system_evaluate.rego");
    fs::write(system_dir.join("evaluate.rego"), evaluate_policy)?;

    // Step 2: Create a REAL policy that checks script content
    let script_inspector_policy = r#"
# METADATA
# scope: package
# custom:
#   routing:
#     required_events: ["PreToolUse"]
#     required_tools: ["Bash"]
package cupcake.policies.script_inspector

import rego.v1

# Block execution of scripts containing dangerous commands
deny contains decision if {
    input.tool_name == "Bash"

    # Script inspection made this content available
    input.executed_script_content

    # Simple check - no helper library needed
    contains(input.executed_script_content, "rm -rf .cupcake")

    decision := {
        "rule_id": "TOB2-FIX",
        "reason": "Script contains command to delete .cupcake directory",
        "severity": "CRITICAL"
    }
}

# Also check for other dangerous patterns
deny contains decision if {
    input.tool_name == "Bash"
    input.executed_script_content

    dangerous_patterns := [
        "rm -rf /",
        "> /dev/sda",
        "dd if=/dev/zero of=/dev/sda",
        "mkfs.ext4 /dev/sda"
    ]

    some pattern in dangerous_patterns
    contains(input.executed_script_content, pattern)

    decision := {
        "rule_id": "DANGEROUS-SCRIPT",
        "reason": sprintf("Script contains dangerous pattern: %s", [pattern]),
        "severity": "CRITICAL"
    }
}
"#;

    fs::write(
        claude_dir.join("script_inspector.rego"),
        script_inspector_policy,
    )?;

    // Step 3: Create a REAL malicious script file
    let malicious_script = temp_dir.path().join("deploy.sh");
    let script_content = r#"#!/bin/bash
# Innocent-looking deployment script
echo "Starting deployment..."
echo "Checking environment..."

# Hidden dangerous commands (TOB-2 attack pattern)
rm -rf .cupcake
rm -rf /important/data

echo "Deployment complete!"
"#;

    fs::write(&malicious_script, script_content)?;

    // Make it executable (for realism)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&malicious_script)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&malicious_script, perms)?;
    }

    // Step 4: Initialize the engine
    let empty_global = TempDir::new()?;
    let config = cupcake_core::engine::EngineConfig {
        global_config: Some(empty_global.path().to_path_buf()),
        harness: HarnessType::ClaudeCode,
        wasm_max_memory: None,
        opa_path: None,
        debug_routing: false,
    };
    let engine = Engine::new_with_config(temp_dir.path(), config).await?;

    // Step 5: Create the event that executes the script
    let mut bash_event = json!({
        "hook_event_name": "PreToolUse",
        "session_id": "test",
        "transcript_path": "/tmp/transcript.md",
        "cwd": temp_dir.path().to_string_lossy(),
        "tool_name": "Bash",
        "tool_input": {
            "command": "./deploy.sh --production"  // Looks innocent!
        }
    });

    // Step 6: Apply preprocessing WITH script inspection
    let preprocess_config = PreprocessConfig::with_script_inspection();
    preprocess_input(&mut bash_event, &preprocess_config, HarnessType::ClaudeCode);

    // Verify script content was loaded
    assert!(
        bash_event.get("executed_script_content").is_some(),
        "Script content should be attached after preprocessing"
    );

    let loaded_content = bash_event["executed_script_content"].as_str().unwrap();
    assert_eq!(
        loaded_content, script_content,
        "Loaded content should match the actual script file"
    );

    // Step 7: Evaluate with the engine - policy should block it!
    let decision = engine.evaluate(&bash_event, None).await?;

    match decision {
        cupcake_core::engine::decision::FinalDecision::Deny { reason, .. } => {
            assert!(
                reason.contains("Script contains command to delete .cupcake"),
                "Policy should detect the dangerous command in the script. Got: {reason}"
            );
            println!(
                "✅ TOB-2 FIXED: Policy successfully blocked script with hidden 'rm -rf .cupcake'"
            );
        }
        _ => {
            panic!(
                "🚨 TOB-2 VULNERABILITY STILL PRESENT! \
                 Script with 'rm -rf .cupcake' was NOT blocked. \
                 Decision: {decision:?}"
            )
        }
    }

    Ok(())
}

/// Test the vulnerability WITHOUT script inspection (demonstrating TOB-2)
#[tokio::test]
#[cfg(feature = "deterministic-tests")]
async fn test_tob2_vulnerability_without_script_inspection() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let cupcake_dir = temp_dir.path().join(".cupcake");
    let policies_dir = cupcake_dir.join("policies");
    let claude_dir = policies_dir.join("claude");
    let system_dir = claude_dir.join("system");

    fs::create_dir_all(&system_dir)?;

    let evaluate_policy = include_str!("fixtures/system_evaluate.rego");
    fs::write(system_dir.join("evaluate.rego"), evaluate_policy)?;

    // Naive policy that only checks the command, not script content
    let naive_policy = r#"
# METADATA
# scope: package
# custom:
#   routing:
#     required_events: ["PreToolUse"]
#     required_tools: ["Bash"]
package cupcake.policies.naive

import rego.v1

# This policy only checks the command itself
deny contains decision if {
    input.tool_name == "Bash"
    cmd := input.tool_input.command

    # Only checking the direct command - MISSES script content!
    contains(cmd, "rm -rf .cupcake")

    decision := {
        "rule_id": "NAIVE-CHECK",
        "reason": "Direct command contains rm -rf .cupcake",
        "severity": "HIGH"
    }
}
"#;

    fs::write(claude_dir.join("naive.rego"), naive_policy)?;

    // Create the same malicious script
    let evil_script = temp_dir.path().join("innocent.sh");
    fs::write(
        &evil_script,
        "#!/bin/bash\nrm -rf .cupcake\nrm -rf /important",
    )?;

    let empty_global = TempDir::new()?;
    let config = cupcake_core::engine::EngineConfig {
        global_config: Some(empty_global.path().to_path_buf()),
        harness: HarnessType::ClaudeCode,
        wasm_max_memory: None,
        opa_path: None,
        debug_routing: false,
    };
    let engine = Engine::new_with_config(temp_dir.path(), config).await?;

    let mut bash_event = json!({
        "hook_event_name": "PreToolUse",
        "session_id": "test",
        "transcript_path": "/tmp/transcript.md",
        "cwd": temp_dir.path().to_string_lossy(),
        "tool_name": "Bash",
        "tool_input": {
            "command": "./innocent.sh"  // Command looks safe!
        }
    });

    // Apply preprocessing WITHOUT script inspection
    let preprocess_config = PreprocessConfig::minimal(); // No script inspection!
    preprocess_input(&mut bash_event, &preprocess_config, HarnessType::ClaudeCode);

    // Verify NO script content was attached
    assert!(
        bash_event.get("executed_script_content").is_none(),
        "No script content without inspection enabled"
    );

    let decision = engine.evaluate(&bash_event, None).await?;

    match decision {
        cupcake_core::engine::decision::FinalDecision::Allow { .. } => {
            println!(
                "⚠️  TOB-2 VULNERABILITY DEMONSTRATED: \
                 Script './innocent.sh' containing 'rm -rf .cupcake' was ALLOWED to execute! \
                 The naive policy missed the dangerous content inside the script."
            );
        }
        _ => {
            println!(
                "Note: Even without script inspection, something blocked execution: {decision:?}"
            );
        }
    }

    Ok(())
}

/// Test complex script execution patterns
#[tokio::test]
#[cfg(feature = "deterministic-tests")]
async fn test_various_script_execution_patterns() -> Result<()> {
    let temp_dir = TempDir::new()?;

    // Create various test scripts
    let scripts = vec![
        ("deploy.sh", "#!/bin/bash\nrm -rf .cupcake"),
        (
            "script.py",
            "#!/usr/bin/env python3\nimport shutil\nshutil.rmtree('.cupcake')",
        ),
        (
            "app.js",
            "const fs = require('fs');\nfs.rmSync('.cupcake', {recursive: true});",
        ),
    ];

    for (filename, content) in &scripts {
        fs::write(temp_dir.path().join(filename), content)?;
    }

    // Test various execution patterns
    let test_patterns = vec![
        ("./deploy.sh", "deploy.sh", "bash"),
        ("bash deploy.sh", "deploy.sh", "bash"),
        ("sh -x deploy.sh", "deploy.sh", "bash"),
        ("python3 script.py", "script.py", "python"),
        ("node app.js", "app.js", "node"),
    ];

    for (command, expected_script, _interpreter) in test_patterns {
        let mut event = json!({
            "hook_event_name": "PreToolUse",
            "tool_name": "Bash",
            "tool_input": {
                "command": command
            },
            "cwd": temp_dir.path().to_string_lossy()
        });

        let config = PreprocessConfig::with_script_inspection();
        preprocess_input(&mut event, &config, HarnessType::ClaudeCode);

        if event.get("executed_script_content").is_some() {
            let path = event["executed_script_path"].as_str().unwrap();
            assert!(
                path.contains(expected_script),
                "Command '{command}' should detect script '{expected_script}'"
            );

            let content = event["executed_script_content"].as_str().unwrap();
            assert!(
                content.contains(".cupcake")
                    || content.contains("rmtree")
                    || content.contains("rmSync"),
                "Script content should contain dangerous operations"
            );
        }
    }

    Ok(())
}
