use super::CommandHandler;
use crate::config::loader::PolicyLoader;
use crate::engine::actions::{ActionContext, ActionExecutor, ActionResult};
use crate::engine::conditions::EvaluationContext;
use crate::engine::evaluation::PolicyEvaluator;
use crate::engine::events::HookEvent;
use crate::engine::response::EngineDecision;
use crate::Result;
use chrono::Utc;
use std::collections::HashMap;
use std::io::{self, Read};

/// Handler for the `run` command
pub struct RunCommand {
    pub event: String,
    pub config: String,
    pub debug: bool,
}

impl CommandHandler for RunCommand {
    fn execute(&self) -> Result<()> {
        if self.debug {
            eprintln!("Debug: Event: {}", self.event);
            eprintln!("Debug: Config file: {}", self.config);
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

        // 2. Load configuration (settings and policies) from file(s)
        let configuration = match self.load_configuration() {
            Ok(config) => config,
            Err(e) => {
                eprintln!("Error loading configuration: {}", e);
                if self.debug {
                    eprintln!("Debug: Graceful degradation - allowing operation due to configuration loading error");
                }
                std::process::exit(0); // Graceful degradation - allow operation
            }
        };

        if self.debug {
            eprintln!(
                "Debug: Loaded {} composed policies",
                configuration.policies.len()
            );
            for (i, policy) in configuration.policies.iter().enumerate() {
                eprintln!(
                    "Debug: Policy {}: {} ({}:{})",
                    i, policy.name, policy.hook_event, policy.matcher
                );
            }
        }

        // 3. Get current directory (used by action context)
        let _current_dir =
            std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));

        // 4. Build evaluation context
        let evaluation_context = self.build_evaluation_context(&hook_event);

        // 5. Execute two-pass evaluation
        let mut policy_evaluator = PolicyEvaluator::new();
        let mut action_executor = ActionExecutor::with_settings(configuration.settings);

        let evaluation_result = match policy_evaluator.evaluate(
            &configuration.policies,
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
                self.send_response_safely(EngineDecision::Allow { reason: None }, &hook_event)
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

        // Special handling for UserPromptSubmit context injection
        let is_user_prompt_submit = hook_event.event_name() == "UserPromptSubmit";

        // For UserPromptSubmit, we'll handle stdout differently
        if !is_user_prompt_submit {
            // Output soft feedback to stdout if we're allowing the operation (non-UserPromptSubmit)
            if matches!(evaluation_result.decision, EngineDecision::Allow { .. })
                && !evaluation_result.feedback_messages.is_empty()
            {
                // Combine all soft feedback messages
                let feedback_output = evaluation_result.feedback_messages.join("\n");
                println!("{}", feedback_output);
            }
        }

        // 6. Execute actions for matched policies
        if self.debug {
            eprintln!(
                "Debug: Executing actions for {} matched policies",
                evaluation_result.matched_policies.len()
            );
        }

        // Execute actions for each matched policy
        let action_results = self.execute_matched_actions(
            &evaluation_result.matched_policies,
            &hook_event,
            &mut action_executor,
        )?;

        // Check if any action resulted in a block or collect context for injection
        let mut final_decision = evaluation_result.decision.clone();
        let mut context_to_inject = Vec::new();

        for (_policy_name, result) in &action_results {
            match result {
                ActionResult::Block { feedback } => {
                    // An action execution resulted in block - override the evaluation decision
                    final_decision = EngineDecision::Block {
                        feedback: feedback.clone(),
                    };
                    break;
                }
                ActionResult::Ask { reason } => {
                    // An action execution resulted in ask - override the evaluation decision
                    final_decision = EngineDecision::Ask {
                        reason: reason.clone(),
                    };
                    break;
                }
                ActionResult::Success {
                    feedback: Some(ctx),
                    ..
                } if is_user_prompt_submit => {
                    // Collect context from InjectContext actions
                    context_to_inject.push(ctx.clone());
                }
                _ => {}
            }
        }

        // 8. Send response to Claude Code
        // For PostToolUse events, soft feedback should use exit code 2 so Claude sees it
        let response_decision = if hook_event.event_name() == "PostToolUse"
            && matches!(final_decision, EngineDecision::Allow { .. })
            && !evaluation_result.feedback_messages.is_empty()
        {
            // Convert soft feedback to a "block" for PostToolUse so Claude sees it
            EngineDecision::Block {
                feedback: evaluation_result.feedback_messages.join("\n"),
            }
        } else {
            final_decision
        };

        // For UserPromptSubmit, handle context injection via stdout or JSON
        if is_user_prompt_submit {
            self.send_response_with_context(response_decision, context_to_inject)
        } else {
            self.send_response_safely(response_decision, &hook_event)
        }
    }

    fn name(&self) -> &'static str {
        "run"
    }
}

impl RunCommand {
    /// Create new run command
    pub fn new(event: String, config: String, debug: bool) -> Self {
        Self {
            event,
            config,
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

    /// Load configuration (settings and policies) using the new YAML composition engine
    fn load_configuration(&self) -> Result<crate::config::loader::LoadedConfiguration> {
        let mut loader = PolicyLoader::new();

        if !self.config.is_empty() {
            // User specified a config file - load from that file
            let config_path = std::path::Path::new(&self.config);
            let configuration = loader.load_configuration(config_path)?;

            if self.debug {
                eprintln!(
                    "Debug: Loaded configuration from config file: {}",
                    self.config
                );
                eprintln!(
                    "Debug: Found and composed {} policies",
                    configuration.policies.len()
                );
                eprintln!(
                    "Debug: Timeout setting: {}ms",
                    configuration.settings.timeout_ms
                );
            }

            Ok(configuration)
        } else {
            // No config specified - use auto-discovery
            let current_dir = std::env::current_dir().map_err(|e| {
                crate::CupcakeError::Config(format!("Failed to get current directory: {}", e))
            })?;

            let configuration = loader.load_configuration_from_directory(&current_dir)?;

            if self.debug {
                eprintln!("Debug: Searched for YAML configuration starting from:");
                eprintln!("  - {}/guardrails/cupcake.yaml", current_dir.display());
                eprintln!(
                    "Debug: Found and composed {} policies",
                    configuration.policies.len()
                );
                eprintln!(
                    "Debug: Timeout setting: {}ms",
                    configuration.settings.timeout_ms
                );
            }

            Ok(configuration)
        }
    }

    /// Build action context from hook event
    fn build_action_context(&self, hook_event: &HookEvent) -> ActionContext {
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
            | HookEvent::PreCompact { common, .. }
            | HookEvent::UserPromptSubmit { common, .. } => {
                (common.session_id.clone(), String::new(), HashMap::new())
            }
        };

        // Use cwd from hook data as authoritative source
        let current_dir = hook_event.common().cwd.clone();
        let current_dir_path = std::path::PathBuf::from(current_dir);

        ActionContext::new(
            tool_name,
            tool_input,
            current_dir_path,
            std::env::vars().collect(),
            session_id,
        )
    }

    /// Build evaluation context from hook event
    fn build_evaluation_context(&self, hook_event: &HookEvent) -> EvaluationContext {
        let (session_id, tool_name, tool_input, prompt) = match hook_event {
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
                None,
            ),
            HookEvent::UserPromptSubmit { common, prompt } => (
                common.session_id.clone(),
                String::new(),
                HashMap::new(),
                Some(prompt.clone()),
            ),
            HookEvent::Notification { common, .. }
            | HookEvent::Stop { common, .. }
            | HookEvent::SubagentStop { common, .. }
            | HookEvent::PreCompact { common, .. } => (
                common.session_id.clone(),
                String::new(),
                HashMap::new(),
                None,
            ),
        };

        // Use cwd from hook data as authoritative source
        let current_dir = hook_event.common().cwd.clone();
        let current_dir_path = std::path::PathBuf::from(current_dir);

        EvaluationContext {
            event_type: hook_event.event_name().to_string(),
            tool_name,
            tool_input,
            session_id,
            current_dir: current_dir_path,
            env_vars: std::env::vars().collect(),
            timestamp: Utc::now(),
            prompt,
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

    /// Execute actions for matched policies
    fn execute_matched_actions(
        &self,
        matched_policies: &[crate::engine::evaluation::MatchedPolicy],
        hook_event: &HookEvent,
        action_executor: &mut ActionExecutor,
    ) -> Result<Vec<(String, ActionResult)>> {
        let mut results = Vec::new();
        for matched_policy in matched_policies {
            if self.debug {
                eprintln!(
                    "Debug: Executing action for policy '{}': {:?}",
                    matched_policy.name, matched_policy.action
                );
            }

            // Build action context
            let action_context = self.build_action_context(hook_event);

            // Execute the action
            let result = action_executor.execute(&matched_policy.action, &action_context);

            match &result {
                ActionResult::Success { feedback, .. } => {
                    if let Some(msg) = feedback {
                        if self.debug {
                            eprintln!("Debug: Action feedback: {}", msg);
                        }
                    }
                }
                ActionResult::Block { feedback } => {
                    if self.debug {
                        eprintln!("Debug: Action execution resulted in block: {}", feedback);
                    }
                }
                ActionResult::Allow { .. } => {
                    if self.debug {
                        eprintln!("Debug: Action execution resulted in allow");
                    }
                }
                ActionResult::Ask { reason } => {
                    if self.debug {
                        eprintln!("Debug: Action execution resulted in ask: {}", reason);
                    }
                }
                ActionResult::Error { message } => {
                    eprintln!(
                        "Error executing action for policy '{}': {}",
                        matched_policy.name, message
                    );
                    // Continue with other actions - graceful degradation
                }
            }

            results.push((matched_policy.name.clone(), result));
        }
        Ok(results)
    }

    /// Send response to Claude Code with error handling
    fn send_response_safely(&self, decision: EngineDecision, hook_event: &HookEvent) -> ! {
        use crate::engine::response::ResponseHandler;

        // Use ResponseHandler which implements the JSON protocol
        let handler = ResponseHandler::new(self.debug);

        // Use hook-aware response method for correct JSON format per event type
        handler.send_response_for_hook(decision, hook_event.event_name());
    }

    /// Send response with context injection for UserPromptSubmit events
    fn send_response_with_context(
        &self,
        decision: EngineDecision,
        context_to_inject: Vec<String>,
    ) -> ! {
        use crate::engine::response::{CupcakeResponse, HookSpecificOutput, ResponseHandler};

        let handler = ResponseHandler::new(self.debug);

        // For UserPromptSubmit, we have special handling based on the decision
        match &decision {
            EngineDecision::Allow { .. } => {
                // For Allow, check if we have context to inject
                if !context_to_inject.is_empty() {
                    // Use stdout method: print context to stdout with exit code 0
                    let combined_context = context_to_inject.join("\n");
                    if self.debug {
                        eprintln!("Debug: Injecting context via stdout for UserPromptSubmit");
                        eprintln!("Debug: Context length: {} chars", combined_context.len());
                    }
                    println!("{}", combined_context);
                    std::process::exit(0);
                } else {
                    // No context to inject, just allow
                    if self.debug {
                        eprintln!("Debug: Allowing UserPromptSubmit without context injection");
                    }
                    std::process::exit(0);
                }
            }
            EngineDecision::Block { feedback } => {
                // For Block, use JSON response to block prompt processing
                if self.debug {
                    eprintln!("Debug: Blocking UserPromptSubmit with feedback");
                }
                let response = CupcakeResponse {
                    hook_specific_output: None,
                    continue_execution: Some(false),
                    stop_reason: Some(feedback.clone()),
                    suppress_output: None,
                };
                handler.send_json_response(response);
            }
            EngineDecision::Ask { reason } => {
                // For Ask, use JSON response with UserPromptSubmit output
                if self.debug {
                    eprintln!("Debug: Sending Ask response for UserPromptSubmit");
                }
                let response = CupcakeResponse {
                    hook_specific_output: Some(HookSpecificOutput::UserPromptSubmit {
                        additional_context: Some(reason.clone()),
                    }),
                    continue_execution: Some(true),
                    stop_reason: None,
                    suppress_output: Some(false),
                };
                handler.send_json_response(response);
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
            "".to_string(), // Auto-discovery mode
            false,
        );

        assert_eq!(cmd.event, "PreToolUse");
        assert_eq!(cmd.config, ""); // Auto-discovery mode
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
            "cwd": "/home/user/project",
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
            "cwd": "/home/user/project",
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
        use std::env;
        use tempfile::tempdir;

        // Test that policy loading fails when no guardrails config exists
        let temp_dir = tempdir().unwrap();
        let original_dir = env::current_dir().unwrap();

        // Change to temp directory where no guardrails config exists
        env::set_current_dir(temp_dir.path()).unwrap();

        let cmd = RunCommand::new(
            "PreToolUse".to_string(),
            "".to_string(), // Auto-discovery mode
            false,
        );

        // This should fail since no guardrails/cupcake.yaml exists
        let result = cmd.load_configuration();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("No guardrails/cupcake.yaml found"));

        // Restore original directory
        env::set_current_dir(original_dir).unwrap();
    }

    // Note: We can't test the full execute() method easily because it calls std::process::exit
    // Integration tests in tests/ directory handle the full execution path
}
