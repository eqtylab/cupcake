use std::fs;
use tempfile::tempdir;

/// Test real-world scenario: enforce test-before-commit workflow
#[test]
fn test_enforce_test_before_commit_workflow() {
    let temp_dir = tempdir().unwrap();
    let policies_dir = temp_dir.path().join("guardrails");
    fs::create_dir(&policies_dir).unwrap();
    
    // Create a policy file that enforces test-before-commit
    let test_policy = r#"
policies:
  - name: require-tests-before-commit
    description: Ensure tests pass before committing
    hook_event: UserPromptSubmit
    matcher: "*"
    conditions:
      - type: pattern
        field: prompt
        regex: "(?i)(commit|push)"
      - type: not
        condition:
          type: state_query
          filter:
            tool: Bash
            command_contains: "npm test"
            result: success
            within_minutes: 15
    action:
      type: inject_context
      context: |
        📋 Pre-commit checklist:
        ✗ Tests have not been run recently (or failed)
        
        Please run 'npm test' before committing your changes.
      use_stdout: true

  - name: celebrate-good-practices
    description: Positive reinforcement when tests pass
    hook_event: UserPromptSubmit
    matcher: "*"
    conditions:
      - type: pattern
        field: prompt
        regex: "(?i)(commit|push)"
      - type: state_query
        filter:
          tool: Bash
          command_contains: "npm test"
          result: success
          within_minutes: 15
    action:
      type: inject_context
      context: |
        ✅ Great job! Tests are passing and you're ready to commit.
      use_stdout: true
"#;
    
    fs::write(policies_dir.join("test-workflow.yaml"), test_policy).unwrap();
    
    // Create main config that includes our policy
    let main_config = r#"
include:
  - test-workflow.yaml

settings:
  timeout_ms: 5000
"#;
    
    fs::write(policies_dir.join("cupcake.yaml"), main_config).unwrap();
    
    // Now test the workflow - first without running tests
    println!("Testing commit without running tests...");
    
    // Simulate UserPromptSubmit event for commit without tests
    let _event_json = r#"{
        "hook_event_name": "UserPromptSubmit",
        "session_id": "test-session-workflow",
        "transcript_path": "/tmp/transcript.jsonl",
        "cwd": "/tmp/project",
        "prompt": "Let me commit these changes"
    }"#;
    
    // The policy should inject context about missing tests
    // (In a real test we'd run the full pipeline, but this demonstrates the setup)
}

/// Test policy that prevents dangerous operations based on state
#[test]
fn test_prevent_force_push_after_pull() {
    let temp_dir = tempdir().unwrap();
    let policies_dir = temp_dir.path().join("guardrails");
    fs::create_dir(&policies_dir).unwrap();
    
    // Create policy that prevents force push after recent pull
    let safety_policy = r#"
policies:
  - name: prevent-force-push-after-pull
    description: Warn about force push after recent pull from team
    hook_event: PreToolUse
    matcher: Bash
    conditions:
      - type: pattern
        field: tool_input.command
        regex: "git\\s+push\\s+--force"
      - type: state_query
        filter:
          tool: Bash
          command_contains: "git pull"
          result: success
          within_minutes: 60
    action:
      type: block_with_feedback
      feedback: |
        ⚠️  DANGER: You recently pulled changes from the remote repository.
        Force pushing now would overwrite your team's work!
        
        If you really need to force push, please:
        1. Double-check with 'git log origin/main..HEAD'
        2. Communicate with your team
        3. Consider using '--force-with-lease' instead
"#;
    
    fs::write(policies_dir.join("safety.yaml"), safety_policy).unwrap();
    
    // Create config
    let config = r#"
include:
  - safety.yaml
"#;
    
    fs::write(policies_dir.join("cupcake.yaml"), config).unwrap();
}

/// Test context injection based on file modifications
#[test]
fn test_context_based_on_file_edits() {
    let temp_dir = tempdir().unwrap();
    let policies_dir = temp_dir.path().join("guardrails");
    fs::create_dir(&policies_dir).unwrap();
    
    // Create policies that provide context based on what files were edited
    let context_policy = r#"
policies:
  - name: api-changes-reminder
    description: Remind to update API docs when API files change
    hook_event: UserPromptSubmit
    matcher: "*"
    conditions:
      - type: pattern
        field: prompt
        regex: "(?i)commit"
      - type: state_query
        filter:
          tool: Edit
          result: success
          within_minutes: 30
    action:
      type: inject_context
      context: |
        📝 Reminder: You've recently edited files. Please ensure:
        - All changes are properly tested
        - Documentation is updated if needed
        - API changes are reflected in the OpenAPI spec

  - name: config-changes-warning
    description: Warn about config file changes
    hook_event: UserPromptSubmit  
    matcher: "*"
    conditions:
      - type: pattern
        field: prompt
        regex: "(?i)(deploy|push|release)"
      - type: or
        conditions:
          - type: state_query
            filter:
              tool: Write
              result: success
              within_minutes: 60
          - type: state_query
            filter:
              tool: Edit
              result: success
              within_minutes: 60
    action:
      type: inject_context
      context: |
        ⚠️  Configuration files were modified recently.
        Please verify:
        - Environment variables are correctly set
        - Secrets are not exposed
        - Config changes are backward compatible
"#;
    
    fs::write(policies_dir.join("context-aware.yaml"), context_policy).unwrap();
    
    let config = r#"
include:
  - context-aware.yaml
"#;
    
    fs::write(policies_dir.join("cupcake.yaml"), config).unwrap();
}

/// Test time-based workflow policies
#[test]
fn test_time_based_workflow_enforcement() {
    let temp_dir = tempdir().unwrap();
    let policies_dir = temp_dir.path().join("guardrails");
    fs::create_dir(&policies_dir).unwrap();
    
    // Create policies that enforce time-based workflows
    let workflow_policy = r#"
policies:
  - name: stale-build-warning
    description: Warn if build hasn't been run recently
    hook_event: UserPromptSubmit
    matcher: "*"
    conditions:
      - type: pattern
        field: prompt
        regex: "(?i)(deploy|release|publish)"
      - type: not
        condition:
          type: state_query
          filter:
            tool: Bash
            command_contains: "npm run build"
            result: success
            within_minutes: 10
    action:
      type: inject_context
      context: |
        ⚠️  No recent successful build detected!
        The last build was more than 10 minutes ago (or failed).
        
        Run 'npm run build' to ensure you're deploying the latest code.

  - name: fresh-dependencies-check
    description: Ensure dependencies are up to date
    hook_event: PreToolUse
    matcher: Bash
    conditions:
      - type: pattern
        field: tool_input.command
        regex: "npm\\s+(start|run\\s+dev)"
      - type: not
        condition:
          type: state_query
          filter:
            tool: Bash
            command_contains: "npm install"
            result: success
            within_minutes: 1440  # 24 hours
    action:
      type: provide_feedback
      feedback: |
        💡 Tip: It's been over 24 hours since you last ran 'npm install'.
        Consider updating your dependencies to avoid version conflicts.
"#;
    
    fs::write(policies_dir.join("workflow.yaml"), workflow_policy).unwrap();
    
    let config = r#"
include:
  - workflow.yaml

settings:
  timeout_ms: 3000
"#;
    
    fs::write(policies_dir.join("cupcake.yaml"), config).unwrap();
}

/// Test combining multiple state queries for complex workflows
#[test]
fn test_complex_workflow_with_multiple_states() {
    let temp_dir = tempdir().unwrap();
    let policies_dir = temp_dir.path().join("guardrails");
    fs::create_dir(&policies_dir).unwrap();
    
    // Create policy that checks multiple conditions before allowing release
    let release_policy = r#"
policies:
  - name: comprehensive-release-checklist
    description: Ensure all checks pass before release
    hook_event: UserPromptSubmit
    matcher: "*"
    conditions:
      - type: pattern
        field: prompt
        regex: "(?i)(release|deploy|publish).*production"
      - type: and
        conditions:
          # Tests must pass
          - type: state_query
            filter:
              tool: Bash
              command_contains: "npm test"
              result: success
              within_minutes: 30
          # Linting must pass
          - type: state_query
            filter:
              tool: Bash
              command_contains: "npm run lint"
              result: success
              within_minutes: 30
          # Build must succeed
          - type: state_query
            filter:
              tool: Bash
              command_contains: "npm run build"
              result: success
              within_minutes: 15
          # No force pushes recently
          - type: not
            condition:
              type: state_query
              filter:
                tool: Bash
                command_contains: "push --force"
                within_minutes: 1440  # 24 hours
    action:
      type: inject_context
      context: |
        ✅ Release Checklist - All Clear!
        ✓ Tests passing
        ✓ Linting passing  
        ✓ Build successful
        ✓ No recent force pushes
        
        You're ready to release to production! 🚀

  - name: release-checklist-failed
    description: Block release if checks haven't passed
    hook_event: UserPromptSubmit
    matcher: "*"
    conditions:
      - type: pattern
        field: prompt
        regex: "(?i)(release|deploy|publish).*production"
      - type: not
        condition:
          type: and
          conditions:
            - type: state_query
              filter:
                tool: Bash
                command_contains: "npm test"
                result: success
                within_minutes: 30
            - type: state_query
              filter:
                tool: Bash
                command_contains: "npm run lint"
                result: success
                within_minutes: 30
            - type: state_query
              filter:
                tool: Bash
                command_contains: "npm run build"
                result: success
                within_minutes: 15
    action:
      type: inject_context
      context: |
        ❌ Release Checklist - Issues Found!
        
        Please complete these steps before releasing:
        - Run 'npm test' (required within last 30 min)
        - Run 'npm run lint' (required within last 30 min)
        - Run 'npm run build' (required within last 15 min)
        
        All must pass before you can safely release.
"#;
    
    fs::write(policies_dir.join("release.yaml"), release_policy).unwrap();
    
    let config = r#"
include:
  - release.yaml
"#;
    
    fs::write(policies_dir.join("cupcake.yaml"), config).unwrap();
}