use std::fs;
use std::time::Instant;
use tempfile::tempdir;

#[test]
fn test_yaml_loading_performance() {
    // Create a test environment with realistic policy structure
    let temp_dir = tempdir().unwrap();
    let guardrails_dir = temp_dir.path().join("guardrails");
    let policies_dir = guardrails_dir.join("policies");
    fs::create_dir_all(&policies_dir).unwrap();

    // Create root config
    let root_config = r#"
settings:
  debug_mode: false

imports:
  - "policies/*.yaml"
"#;
    fs::write(guardrails_dir.join("cupcake.yaml"), root_config).unwrap();

    // Create a realistic policy file with multiple policies
    let policy_content = r#"
PreToolUse:
  "Bash":
    - name: "Block dangerous commands"
      description: "Prevent execution of dangerous system commands"
      conditions:
        - type: "pattern"
          field: "tool_input.command"
          regex: "^(rm|dd|format)\\s+.*(-rf|--force)"
      action:
        type: "block_with_feedback"
        feedback_message: "Dangerous command blocked for safety"
        include_context: true
    
    - name: "Git commit checks"
      conditions:
        - type: "pattern"
          field: "tool_input.command"
          regex: "^git\\s+commit"
      action:
        type: "provide_feedback"
        message: "Remember to run tests before committing"
        include_context: false
    
    - name: "NPM install warning"
      conditions:
        - type: "pattern"
          field: "tool_input.command"
          regex: "^npm\\s+install"
      action:
        type: "provide_feedback"
        message: "Consider using npm ci for reproducible builds"
        include_context: false

  "Write|Edit":
    - name: "Python formatting"
      conditions:
        - type: "pattern"
          field: "tool_input.file_path"
          regex: "\\.py$"
      action:
        type: "provide_feedback"
        message: "Run black formatter after editing Python files"
        include_context: false

PostToolUse:
  "Write":
    - name: "File creation logging"
      conditions:
        - type: "match"
          field: "tool_name"
          value: "Write"
      action:
        type: "provide_feedback"
        message: "File created successfully"
        include_context: false
"#;

    // Create multiple policy files with unique names to simulate real usage
    for i in 0..5 {
        let filename = format!("{:02}-policies.yaml", i);
        // Modify policy names to be unique per file
        let unique_content = policy_content.replace("Block dangerous commands", &format!("Block dangerous commands {}", i))
            .replace("Git commit checks", &format!("Git commit checks {}", i))
            .replace("NPM install warning", &format!("NPM install warning {}", i))
            .replace("Python formatting", &format!("Python formatting {}", i))
            .replace("File creation logging", &format!("File creation logging {}", i));
        fs::write(policies_dir.join(&filename), &unique_content).unwrap();
    }

    // Warm up - load once to ensure file system caches are warm
    let mut loader = cupcake::config::loader::PolicyLoader::new();
    let _ = loader.load_and_compose_policies(temp_dir.path()).unwrap();

    // Measure loading time over multiple iterations
    const ITERATIONS: u32 = 100;
    let mut total_duration = std::time::Duration::new(0, 0);
    let mut min_duration = std::time::Duration::new(u64::MAX, 0);
    let mut max_duration = std::time::Duration::new(0, 0);

    for _ in 0..ITERATIONS {
        let mut loader = cupcake::config::loader::PolicyLoader::new();
        
        let start = Instant::now();
        let policies = loader.load_and_compose_policies(temp_dir.path()).unwrap();
        let duration = start.elapsed();
        
        total_duration += duration;
        min_duration = min_duration.min(duration);
        max_duration = max_duration.max(duration);
        
        // Verify we loaded the expected number of policies
        assert_eq!(policies.len(), 25); // 5 policies per file * 5 files
    }

    let avg_duration = total_duration / ITERATIONS;
    
    println!("=== YAML Loading Performance ===");
    println!("Iterations: {}", ITERATIONS);
    println!("Total policies loaded: 25 (5 files × 5 policies)");
    println!("Average time: {:?}", avg_duration);
    println!("Min time: {:?}", min_duration);
    println!("Max time: {:?}", max_duration);
    println!("================================");

    // Assert sub-100ms requirement (with some margin for CI environments)
    assert!(
        avg_duration.as_millis() < 100,
        "Average loading time {:?} exceeds 100ms target",
        avg_duration
    );
}

#[test]
fn test_yaml_parsing_vs_composition_performance() {
    // This test separates parsing time from composition time
    let temp_dir = tempdir().unwrap();
    let policy_yaml = r#"
PreToolUse:
  "Bash":
    - name: "Test policy"
      conditions:
        - type: "pattern"
          field: "tool_input.command"
          regex: ".*"
      action:
        type: "provide_feedback"
        message: "Test"
        include_context: false
"#;

    // Test 1: Pure YAML parsing performance
    let mut parse_total = std::time::Duration::new(0, 0);
    const ITERATIONS: u32 = 1000;
    
    for _ in 0..ITERATIONS {
        let start = Instant::now();
        let _: cupcake::config::types::PolicyFragment = 
            serde_yaml_ng::from_str(policy_yaml).unwrap();
        parse_total += start.elapsed();
    }
    
    let avg_parse = parse_total / ITERATIONS;
    
    // Test 2: Full load cycle (includes file I/O)
    let guardrails_dir = temp_dir.path().join("guardrails");
    let policies_dir = guardrails_dir.join("policies");
    fs::create_dir_all(&policies_dir).unwrap();
    
    fs::write(
        guardrails_dir.join("cupcake.yaml"),
        "settings:\n  debug_mode: false\nimports:\n  - \"policies/*.yaml\"\n"
    ).unwrap();
    fs::write(policies_dir.join("test.yaml"), policy_yaml).unwrap();
    
    let mut load_total = std::time::Duration::new(0, 0);
    
    // Fewer iterations for full load test since it includes I/O
    const LOAD_ITERATIONS: u32 = 100;
    
    for _ in 0..LOAD_ITERATIONS {
        let mut loader = cupcake::config::loader::PolicyLoader::new();
        let start = Instant::now();
        let _ = loader.load_and_compose_policies(temp_dir.path()).unwrap();
        load_total += start.elapsed();
    }
    
    let avg_load = load_total / LOAD_ITERATIONS;
    
    println!("=== YAML Parsing vs Full Load ===");
    println!("Parse iterations: {}", ITERATIONS);
    println!("Load iterations: {}", LOAD_ITERATIONS);
    println!("Pure YAML parsing: {:?}", avg_parse);
    println!("Full load cycle: {:?}", avg_load);
    println!("=================================");
    
    // Both should be well under 100ms
    assert!(avg_parse.as_millis() < 10, "Parsing takes too long");
    assert!(avg_load.as_millis() < 100, "Full load takes too long");
}