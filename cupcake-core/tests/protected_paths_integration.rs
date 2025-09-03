//! Integration test for protected_paths builtin
//! 
//! Tests that protected paths allow reads but block writes

use anyhow::Result;
use cupcake_core::engine::Engine;
use serde_json::json;
use std::fs;
use tempfile::TempDir;

/// Test that protected_paths blocks writes but allows reads
#[tokio::test]
#[cfg(feature = "deterministic-tests")]
async fn test_protected_paths_read_write_distinction() -> Result<()> {
    // Create a temporary directory for test
    let temp_dir = TempDir::new()?;
    let cupcake_dir = temp_dir.path().join(".cupcake");
    let policies_dir = cupcake_dir.join("policies");
    let system_dir = policies_dir.join("system");
    let builtins_dir = policies_dir.join("builtins");
    
    fs::create_dir_all(&system_dir)?;
    fs::create_dir_all(&builtins_dir)?;
    
    // Use the authoritative system evaluation policy
    let evaluate_policy = include_str!("../../examples/0_start_here_demo/.cupcake/policies/system/evaluate.rego");
    fs::write(system_dir.join("evaluate.rego"), evaluate_policy)?;
    
    // Use the actual protected_paths policy
    let protected_policy = include_str!("../../examples/.cupcake/policies/builtins/protected_paths.rego");
    fs::write(builtins_dir.join("protected_paths.rego"), protected_policy)?;
    
    // Create guidebook with protected_paths configuration
    // The builtin generates its own signals from the config
    let guidebook_content = r#"
builtins:
  protected_paths:
    enabled: true
    message: "This file is protected"
    paths:
      - "production.env"
      - "src/legacy/"
      - "*.secret"
"#;
    fs::write(cupcake_dir.join("guidebook.yml"), guidebook_content)?;
    
    // Create the engine
    let engine = Engine::new(temp_dir.path()).await?;
    
    eprintln!("=== Testing protected_paths builtin ===");
    eprintln!("Config should have protected: production.env, src/legacy/, *.secret");
    
    // Test 1: BLOCK Write operation on protected file
    let write_event = json!({
        "hook_event_name": "PreToolUse",
        "tool_name": "Write",
        "tool_input": {
            "file_path": "production.env",
            "content": "malicious content"
        }
    });
    
    let decision = engine.evaluate(&write_event).await?;
    match decision {
        cupcake_core::engine::decision::FinalDecision::Halt { reason } => {
            assert!(reason.contains("protected"), "Should mention protected: {}", reason);
        }
        _ => panic!("Expected Halt for write to protected file, got: {:?}", decision)
    }
    
    // Test 2: ALLOW Read operation on protected file
    let read_event = json!({
        "hook_event_name": "PreToolUse",
        "tool_name": "Read",
        "tool_input": {
            "file_path": "production.env"
        }
    });
    
    let decision = engine.evaluate(&read_event).await?;
    match decision {
        cupcake_core::engine::decision::FinalDecision::Allow { .. } => {
            // Good - reads are allowed
        }
        _ => panic!("Expected Allow for read of protected file, got: {:?}", decision)
    }
    
    // Test 3: BLOCK Edit operation on directory contents
    let edit_event = json!({
        "hook_event_name": "PreToolUse",
        "tool_name": "Edit",
        "tool_input": {
            "file_path": "src/legacy/old_code.rs",
            "old_string": "old",
            "new_string": "new"
        }
    });
    
    let decision = engine.evaluate(&edit_event).await?;
    match decision {
        cupcake_core::engine::decision::FinalDecision::Halt { reason } => {
            assert!(reason.contains("protected"), "Should mention protected: {}", reason);
        }
        _ => panic!("Expected Halt for edit in protected directory, got: {:?}", decision)
    }
    
    // Test 4: BLOCK write to glob pattern match
    let secret_write = json!({
        "hook_event_name": "PreToolUse",
        "tool_name": "Write",
        "tool_input": {
            "file_path": "config.secret",
            "content": "secrets"
        }
    });
    
    let decision = engine.evaluate(&secret_write).await?;
    match decision {
        cupcake_core::engine::decision::FinalDecision::Halt { reason } => {
            assert!(reason.contains("protected"), "Should mention protected: {}", reason);
        }
        _ => panic!("Expected Halt for write to .secret file, got: {:?}", decision)
    }
    
    // Test 5: ALLOW write to non-protected file
    let normal_write = json!({
        "hook_event_name": "PreToolUse",
        "tool_name": "Write",
        "tool_input": {
            "file_path": "src/main.rs",
            "content": "normal content"
        }
    });
    
    let decision = engine.evaluate(&normal_write).await?;
    match decision {
        cupcake_core::engine::decision::FinalDecision::Allow { .. } => {
            // Good - writes to non-protected files are allowed
        }
        _ => panic!("Expected Allow for write to non-protected file, got: {:?}", decision)
    }
    
    Ok(())
}

/// Test Bash command whitelisting for protected paths
#[tokio::test]
#[cfg(feature = "deterministic-tests")]
async fn test_protected_paths_bash_whitelist() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let cupcake_dir = temp_dir.path().join(".cupcake");
    let policies_dir = cupcake_dir.join("policies");
    let system_dir = policies_dir.join("system");
    let builtins_dir = policies_dir.join("builtins");
    
    fs::create_dir_all(&system_dir)?;
    fs::create_dir_all(&builtins_dir)?;
    
    let evaluate_policy = include_str!("../../examples/0_start_here_demo/.cupcake/policies/system/evaluate.rego");
    fs::write(system_dir.join("evaluate.rego"), evaluate_policy)?;
    
    let protected_policy = include_str!("../../examples/.cupcake/policies/builtins/protected_paths.rego");
    fs::write(builtins_dir.join("protected_paths.rego"), protected_policy)?;
    
    let guidebook_content = r#"
builtins:
  protected_paths:
    enabled: true
    paths:
      - "secure.txt"
"#;
    fs::write(cupcake_dir.join("guidebook.yml"), guidebook_content)?;
    
    let engine = Engine::new(temp_dir.path()).await?;
    
    // Test whitelisted read commands are ALLOWED
    let read_commands = vec![
        "cat secure.txt",
        "less secure.txt",
        "grep pattern secure.txt",
        "head -n 10 secure.txt",
        "wc -l secure.txt",
    ];
    
    for cmd in read_commands {
        let bash_event = json!({
            "hook_event_name": "PreToolUse",
            "tool_name": "Bash",
            "tool_input": {
                "command": cmd
            }
        });
        
        let decision = engine.evaluate(&bash_event).await?;
        match decision {
            cupcake_core::engine::decision::FinalDecision::Allow { .. } => {
                // Good - read commands are allowed
            }
            _ => panic!("Expected Allow for read command '{}', got: {:?}", cmd, decision)
        }
    }
    
    // Test non-whitelisted commands are BLOCKED
    let write_commands = vec![
        "echo 'data' > secure.txt",
        "mv secure.txt backup.txt",
        "rm secure.txt",
        "sed -i 's/old/new/g' secure.txt",
        "vim secure.txt",  // Not in whitelist
    ];
    
    for cmd in write_commands {
        let bash_event = json!({
            "hook_event_name": "PreToolUse",
            "tool_name": "Bash",
            "tool_input": {
                "command": cmd
            }
        });
        
        let decision = engine.evaluate(&bash_event).await?;
        match decision {
            cupcake_core::engine::decision::FinalDecision::Halt { reason } => {
                assert!(reason.contains("read operations allowed"), 
                    "Should mention only read allowed for '{}': {}", cmd, reason);
            }
            _ => panic!("Expected Halt for write command '{}', got: {:?}", cmd, decision)
        }
    }
    
    Ok(())
}