use super::CommandHandler;
use crate::config::loader::PolicyLoader;
use crate::config::types::ComposedPolicy;
use crate::engine::conditions::EvaluationContext;
use crate::engine::evaluation::PolicyEvaluator;
use crate::engine::events::HookEvent;
use crate::engine::response::PolicyDecision;
use crate::state::manager::StateManager;
use crate::Result;
use chrono::Utc;
use std::collections::HashMap;
use std::io::{self, Read};

/// Handler for the `run` command
pub struct RunCommand {
    pub event: String,
    pub timeout: u32,
    pub policy_file: String,
    pub debug: bool,
}

impl CommandHandler for RunCommand {
    fn execute(&self) -> Result<()> {
        if self.debug {
            eprintln!("Debug: Event: {}", self.event);
            eprintln!("Debug: Timeout: {}s", self.timeout);
            eprintln!("Debug: Policy file: {}", self.policy_file);
        }

        // 1. Read hook event JSON from stdin
        let hook_event = match self.read_hook_event_from_stdin() {
            Ok(event) => event,
            Err(e) => {
                eprintln!("Error reading hook event: {}", e);
                if self.debug {
                    eprintln!(
                        "Debug: Graceful degradation - allowing operation due to input error"
                    );
                }
                std::process::exit(0); // Graceful degradation - allow operation
            }
        };

        if self.debug {
            eprintln!("Debug: Parsed hook event: {:?}", hook_event);
        }

        // 2. Load policies from file(s)
        let policies = match self.load_policies() {
            Ok(policies) => policies,
            Err(e) => {
                eprintln!("Error loading policies: {}", e);
                if self.debug {
                    eprintln!("Debug: Graceful degradation - allowing operation due to policy loading error");
                }
                std::process::exit(0); // Graceful degradation - allow operation
            }
        };

        if self.debug {
            eprintln!("Debug: Loaded {} composed policies", policies.len());
            for (i, policy) in policies.iter().enumerate() {
                eprintln!(
                    "Debug: Policy {}: {} ({}:{})",
                    i, policy.name, policy.hook_event, policy.matcher
                );
            }
        }

        // 3. Initialize state manager
        let current_dir = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
        let mut state_manager = StateManager::new(&current_dir)?;

        // 4. Build evaluation context
        let evaluation_context = self.build_evaluation_context(&hook_event);

        // 5. Execute two-pass evaluation
        let mut policy_evaluator = PolicyEvaluator::new();
        let evaluation_result = match policy_evaluator.evaluate(
            &policies,
            &hook_event,
            &evaluation_context,
        ) {
            Ok(result) => result,
            Err(e) => {
                eprintln!("Error during policy evaluation: {}", e);
                if self.debug {
                    eprintln!(
                        "Debug: Graceful degradation - allowing operation due to evaluation error"
                    );
                }
                // Graceful degradation - allow operation on evaluation error
                self.send_response_safely(PolicyDecision::Allow)
            }
        };

        if self.debug {
            eprintln!(
                "Debug: Evaluation complete - Decision: {:?}",
                evaluation_result.decision
            );
            if !evaluation_result.feedback_messages.is_empty() {
                eprintln!(
                    "Debug: Collected feedback messages: {:?}",
                    evaluation_result.feedback_messages
                );
            }
        }

        // Output soft feedback to stdout if we're allowing the operation
        if matches!(evaluation_result.decision, PolicyDecision::Allow)
            && !evaluation_result.feedback_messages.is_empty()
        {
            // Combine all soft feedback messages
            let feedback_output = evaluation_result.feedback_messages.join("\n");
            println!("{}", feedback_output);
        }

        // 6. Track tool usage for PostToolUse events
        if let Err(e) = self.track_tool_usage(&mut state_manager, &hook_event) {
            if self.debug {
                eprintln!("Debug: Failed to track tool usage: {}", e);
            }
            // Non-critical error - continue with response
        }

        // 7. Send response to Claude Code
        // For PostToolUse events, soft feedback should use exit code 2 so Claude sees it
        let final_decision = if hook_event.event_name() == "PostToolUse"
            && matches!(evaluation_result.decision, PolicyDecision::Allow)
            && !evaluation_result.feedback_messages.is_empty()
        {
            // Convert soft feedback to a "block" for PostToolUse so Claude sees it
            PolicyDecision::Block {
                feedback: evaluation_result.feedback_messages.join("\n"),
            }
        } else {
            evaluation_result.decision
        };

        self.send_response_safely(final_decision)
    }

    fn name(&self) -> &'static str {
        "run"
    }
}

impl RunCommand {
    /// Create new run command
    pub fn new(event: String, timeout: u32, policy_file: String, debug: bool) -> Self {
        Self {
            event,
            timeout,
            policy_file,
            debug,
        }
    }

    /// Read and parse hook event JSON from stdin
    fn read_hook_event_from_stdin(&self) -> Result<HookEvent> {
        let mut input = String::new();
        io::stdin().read_to_string(&mut input)?;

        if input.trim().is_empty() {
            return Err(crate::CupcakeError::HookEvent(
                "No input received from stdin".to_string(),
            ));
        }

        if self.debug {
            eprintln!("Debug: Raw stdin input: {}", input.trim());
        }

        serde_json::from_str(&input)
            .map_err(|e| crate::CupcakeError::HookEvent(format!("Invalid JSON from stdin: {}", e)))
    }

    /// Load policies using the new YAML composition engine
    fn load_policies(&self) -> Result<Vec<ComposedPolicy>> {
        let mut loader = PolicyLoader::new();

        // Get current directory for policy discovery
        let current_dir = std::env::current_dir().map_err(|e| {
            crate::CupcakeError::Config(format!("Failed to get current directory: {}", e))
        })?;

        // Use the new YAML composition engine
        let policies = loader.load_and_compose_policies(&current_dir)?;

        if self.debug {
            eprintln!("Debug: Searched for YAML policies starting from:");
            eprintln!("  - {}/guardrails/cupcake.yaml", current_dir.display());
            eprintln!("Debug: Found and composed {} policies", policies.len());
        }

        Ok(policies)
    }

    /// Build evaluation context from hook event
    fn build_evaluation_context(&self, hook_event: &HookEvent) -> EvaluationContext {
        let (session_id, tool_name, tool_input) = match hook_event {
            HookEvent::PreToolUse {
                common,
                tool_name,
                tool_input,
            }
            | HookEvent::PostToolUse {
                common,
                tool_name,
                tool_input,
                ..
            } => (
                common.session_id.clone(),
                tool_name.clone(),
                self.extract_tool_input(tool_input),
            ),
            HookEvent::Notification { common, .. }
            | HookEvent::Stop { common, .. }
            | HookEvent::SubagentStop { common, .. }
            | HookEvent::PreCompact { common, .. } => {
                (common.session_id.clone(), String::new(), HashMap::new())
            }
        };

        EvaluationContext {
            event_type: hook_event.event_name().to_string(),
            tool_name,
            tool_input,
            session_id,
            current_dir: std::env::current_dir().unwrap_or_default(),
            env_vars: std::env::vars().collect(),
            timestamp: Utc::now(),
            full_session_state: None, // Will be loaded by state manager if needed
        }
    }

    /// Extract tool input as a map of string values
    fn extract_tool_input(
        &self,
        tool_input: &serde_json::Value,
    ) -> HashMap<String, serde_json::Value> {
        match tool_input {
            serde_json::Value::Object(map) => map.clone().into_iter().collect(),
            _ => HashMap::new(),
        }
    }

    /// Track tool usage in state for PostToolUse events
    fn track_tool_usage(
        &self,
        state_manager: &mut StateManager,
        hook_event: &HookEvent,
    ) -> Result<()> {
        if let HookEvent::PostToolUse {
            common,
            tool_name,
            tool_input,
            tool_response,
        } = hook_event
        {
            // Extract input as HashMap
            let input_map = self.extract_tool_input(tool_input);

            // Determine success based on tool response (simplified - could be enhanced)
            let success = !tool_response.is_null();

            state_manager.add_tool_usage(
                &common.session_id,
                tool_name.clone(),
                input_map,
                success,
                Some(tool_response.clone()),
                None, // Duration not available from hook event
            )?;
        }
        Ok(())
    }

    /// Send response to Claude Code with error handling
    fn send_response_safely(&self, decision: PolicyDecision) -> ! {
        match decision {
            PolicyDecision::Allow => {
                if self.debug {
                    eprintln!("Debug: Allowing operation (exit code 0)");
                }
                std::process::exit(0);
            }
            PolicyDecision::Block { feedback } => {
                if self.debug {
                    eprintln!("Debug: Blocking operation with feedback (exit code 2)");
                }
                eprintln!("{}", feedback);
                std::process::exit(2);
            }
            PolicyDecision::Approve { reason } => {
                if self.debug {
                    eprintln!("Debug: Approving operation (exit code 0)");
                }

                // Send JSON response for approval
                match serde_json::to_string(&crate::engine::response::CupcakeResponse::approve(
                    reason,
                )) {
                    Ok(json) => println!("{}", json),
                    Err(e) => {
                        eprintln!("Error serializing approval response: {}", e);
                        if self.debug {
                            eprintln!("Debug: Graceful degradation - allowing operation despite serialization error");
                        }
                        // Graceful degradation - just allow without JSON response
                    }
                }
                std::process::exit(0);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::events::HookEvent;

    #[test]
    fn test_run_command_creation() {
        let cmd = RunCommand::new(
            "PreToolUse".to_string(),
            60,
            "".to_string(), // Auto-discovery mode
            false,
        );

        assert_eq!(cmd.event, "PreToolUse");
        assert_eq!(cmd.timeout, 60);
        assert_eq!(cmd.policy_file, ""); // Auto-discovery mode
        assert!(!cmd.debug);
        assert_eq!(cmd.name(), "run");
    }

    #[test]
    fn test_hook_event_parsing() {
        // Test parsing a simple PreToolUse event
        let json = r#"
        {
            "hook_event_name": "PreToolUse",
            "session_id": "test-session-123",
            "transcript_path": "/tmp/transcript.jsonl",
            "tool_name": "Bash",
            "tool_input": {
                "command": "echo 'Hello, World!'",
                "description": "Test command"
            }
        }
        "#;

        let event: HookEvent = serde_json::from_str(json).unwrap();

        match event {
            HookEvent::PreToolUse {
                common,
                tool_name,
                tool_input,
            } => {
                assert_eq!(common.session_id, "test-session-123");
                assert_eq!(tool_name, "Bash");
                assert_eq!(tool_input["command"], "echo 'Hello, World!'");
                assert_eq!(tool_input["description"], "Test command");
            }
            _ => panic!("Expected PreToolUse event"),
        }
    }

    #[test]
    fn test_notification_event_parsing() {
        let json = r#"
        {
            "hook_event_name": "Notification",
            "session_id": "test-session-456",
            "transcript_path": "/tmp/transcript.jsonl",
            "message": "Claude needs your permission to use Bash"
        }
        "#;

        let event: HookEvent = serde_json::from_str(json).unwrap();

        match event {
            HookEvent::Notification { common, message } => {
                assert_eq!(common.session_id, "test-session-456");
                assert_eq!(message, "Claude needs your permission to use Bash");
            }
            _ => panic!("Expected Notification event"),
        }
    }

    #[test]
    fn test_policy_loading_with_nonexistent_files() {
        // Test that policy loading works when no policy files exist
        let cmd = RunCommand::new(
            "PreToolUse".to_string(),
            60,
            "".to_string(), // Auto-discovery mode
            false,
        );

        // This should not fail even if no policy files exist
        let policies = cmd.load_policies().unwrap();
        assert_eq!(policies.len(), 0); // Should return empty list
    }

    // Note: We can't test the full execute() method easily because it calls std::process::exit
    // Integration tests in tests/ directory handle the full execution path

    #[test]
    fn test_policy_loading_with_invalid_path() {
        // Test that policy loading fails gracefully with invalid custom path
        let cmd = RunCommand::new(
            "PreToolUse".to_string(),
            60,
            "".to_string(), // Auto-discovery mode
            false,
        );

        let result = cmd.load_policies();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("No guardrails/cupcake.yaml found"));
    }
}
