use cupcake_rego::engine::Engine;
use serde_json::json;
use std::fs;
use tempfile::TempDir;
use tokio;

#[tokio::test]
async fn test_simple_ask_rule() {
    // Create a temporary test project structure
    let temp_dir = TempDir::new().unwrap();
    let project_path = temp_dir.path();
    
    // Create .cupcake directory structure
    let cupcake_dir = project_path.join(".cupcake");
    let policies_dir = cupcake_dir.join("policies");
    let system_dir = policies_dir.join("system");
    
    fs::create_dir_all(&system_dir).unwrap();
    
    // Create system evaluation policy
    let system_policy = r#"package cupcake.system

import rego.v1

evaluate := {
    "halts": collect_verbs("halt"),
    "denials": collect_verbs("deny"),
    "blocks": collect_verbs("block"),
    "asks": collect_verbs("ask"),
    "allow_overrides": collect_verbs("allow_override"),
    "add_context": collect_verbs("add_context")
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
"#;
    
    fs::write(system_dir.join("evaluate.rego"), system_policy).unwrap();
    
    // Create a simple test policy with ask rule
    let test_policy = r#"package cupcake.policies.simple_test

import rego.v1

# METADATA
# scope: rule
# title: Simple Test Policy
# custom:
#   routing:
#     required_events: ["PreToolUse"]
#     required_tools: ["Bash"]

# Simple ask rule
ask contains decision if {
    contains(input.tool_input.command, "test")
    
    decision := {
        "reason": "This is a test command - confirm?",
        "severity": "LOW",
        "rule_id": "SIMPLE-ASK"
    }
}

# Simple deny rule for comparison
deny contains decision if {
    contains(input.tool_input.command, "danger")
    
    decision := {
        "reason": "Dangerous command detected",
        "severity": "HIGH",
        "rule_id": "SIMPLE-DENY"
    }
}
"#;
    
    fs::write(policies_dir.join("simple_test.rego"), test_policy).unwrap();
    
    // Initialize engine
    let mut engine = Engine::new(&project_path).await.unwrap();
    
    // Test that the deny rule works
    let deny_event = json!({
        "hookEventName": "PreToolUse",
        "tool_name": "Bash",
        "tool_input": {
            "command": "danger command"
        },
        "session_id": "test-deny",
        "cwd": "/tmp"
    });
    
    let deny_decision = engine.evaluate(&deny_event).await.unwrap();
    eprintln!("Deny decision: {:?}", deny_decision);
    assert!(deny_decision.is_blocking(), "Deny rule should block");
    
    // Test that the ask rule works  
    let ask_event = json!({
        "hookEventName": "PreToolUse",
        "tool_name": "Bash",
        "tool_input": {
            "command": "test command"
        },
        "session_id": "test-ask",
        "cwd": "/tmp"
    });
    
    let ask_decision = engine.evaluate(&ask_event).await.unwrap();
    eprintln!("Ask decision: {:?}", ask_decision);
    assert!(ask_decision.requires_confirmation(), "Ask rule should require confirmation");
}