use crate::config::types::PolicyFile;
use crate::engine::conditions::{ConditionEvaluator, EvaluationContext};
use crate::engine::response::PolicyDecision;
use crate::Result;

/// Two-pass policy evaluation engine
pub struct PolicyEvaluator {
    condition_evaluator: ConditionEvaluator,
}

/// Result of Pass 1 evaluation (feedback collection)
#[derive(Debug, Clone)]
pub struct FeedbackCollection {
    /// All feedback messages collected from soft actions
    pub feedback_messages: Vec<String>,
}

/// Result of Pass 2 evaluation (hard action detection)
#[derive(Debug, Clone)]
pub enum HardDecision {
    /// No hard action found, allow operation
    Allow,
    /// Block operation with feedback
    Block { feedback: String },
    /// Approve operation
    Approve { reason: Option<String> },
}

/// Complete evaluation result combining both passes
#[derive(Debug, Clone)]
pub struct EvaluationResult {
    /// Final decision (from Pass 2)
    pub decision: PolicyDecision,
    /// All feedback collected (from Pass 1)
    pub feedback_messages: Vec<String>,
}

impl PolicyEvaluator {
    /// Create new policy evaluator
    pub fn new() -> Self {
        Self {
            condition_evaluator: ConditionEvaluator::new(),
        }
    }

    /// Execute two-pass evaluation on the given policies
    pub fn evaluate(
        &mut self,
        policies: &[PolicyFile],
        hook_event: &crate::engine::events::HookEvent,
        evaluation_context: &EvaluationContext,
    ) -> Result<EvaluationResult> {
        // Pass 1: Collect all feedback from soft actions
        let feedback_collection = self.execute_pass_1(policies, hook_event, evaluation_context)?;

        // Pass 2: Find first hard action decision
        let hard_decision = self.execute_pass_2(policies, hook_event, evaluation_context)?;

        // Combine results
        let decision = match hard_decision {
            HardDecision::Allow => {
                // Soft feedback doesn't block - it's just informational
                PolicyDecision::Allow
            }
            HardDecision::Block { feedback } => {
                // Combine hard block feedback with all collected feedback
                let mut all_feedback = vec![feedback];
                all_feedback.extend(feedback_collection.feedback_messages.clone());
                let combined_feedback = all_feedback.join("\n");
                PolicyDecision::Block {
                    feedback: combined_feedback,
                }
            }
            HardDecision::Approve { reason } => PolicyDecision::Approve { reason },
        };

        Ok(EvaluationResult {
            decision,
            feedback_messages: feedback_collection.feedback_messages,
        })
    }

    /// Build ordered list of policies matching the hook event
    fn build_ordered_policy_list<'a>(
        &self,
        policy_files: &'a [PolicyFile],
        hook_event: &crate::engine::events::HookEvent,
    ) -> Result<Vec<&'a crate::config::types::Policy>> {
        let mut ordered_policies = Vec::new();
        let hook_event_name = hook_event.event_name();
        let tool_name = hook_event.tool_name();

        for policy_file in policy_files {
            for policy in &policy_file.policies {
                // Check if policy applies to this hook event
                let policy_event_name = match &policy.hook_event {
                    crate::config::types::HookEventType::PreToolUse => "PreToolUse",
                    crate::config::types::HookEventType::PostToolUse => "PostToolUse",
                    crate::config::types::HookEventType::Notification => "Notification",
                    crate::config::types::HookEventType::Stop => "Stop",
                    crate::config::types::HookEventType::SubagentStop => "SubagentStop",
                    crate::config::types::HookEventType::PreCompact => "PreCompact",
                };

                if policy_event_name != hook_event_name {
                    continue;
                }

                // Check if policy matcher applies to this tool (for PreToolUse/PostToolUse)
                if let Some(ref matcher) = policy.matcher {
                    if let Some(tool) = tool_name {
                        let matcher_regex = regex::Regex::new(matcher).map_err(|e| {
                            crate::CupcakeError::Config(format!(
                                "Invalid matcher regex '{}': {}",
                                matcher, e
                            ))
                        })?;

                        if !matcher_regex.is_match(tool) {
                            continue;
                        }
                    }
                }

                ordered_policies.push(policy);
            }
        }

        Ok(ordered_policies)
    }

    /// Execute Pass 1: Collect all feedback from soft actions
    fn execute_pass_1(
        &mut self,
        policy_files: &[PolicyFile],
        hook_event: &crate::engine::events::HookEvent,
        evaluation_context: &EvaluationContext,
    ) -> Result<FeedbackCollection> {
        let mut feedback_messages = Vec::new();
        let ordered_policies = self.build_ordered_policy_list(policy_files, hook_event)?;

        for policy in ordered_policies {
            // Evaluate all conditions for this policy
            let conditions_match = self.evaluate_policy_conditions(policy, evaluation_context)?;

            if conditions_match {
                // Check if this is a soft action
                if policy.action.is_soft_action() {
                    // Extract feedback message based on action type
                    let feedback =
                        self.extract_feedback_message(&policy.action, evaluation_context);
                    if let Some(msg) = feedback {
                        feedback_messages.push(msg);
                    }
                }
            }
        }

        Ok(FeedbackCollection { feedback_messages })
    }

    /// Execute Pass 2: Find first hard action decision
    fn execute_pass_2(
        &mut self,
        policy_files: &[PolicyFile],
        hook_event: &crate::engine::events::HookEvent,
        evaluation_context: &EvaluationContext,
    ) -> Result<HardDecision> {
        let ordered_policies = self.build_ordered_policy_list(policy_files, hook_event)?;

        for policy in ordered_policies {
            // Evaluate all conditions for this policy
            let conditions_match = self.evaluate_policy_conditions(policy, evaluation_context)?;

            if conditions_match {
                // Check if this is a hard action
                if policy.action.is_hard_action() {
                    return match &policy.action {
                        crate::config::actions::Action::BlockWithFeedback {
                            feedback_message,
                            ..
                        } => {
                            let feedback =
                                self.substitute_templates(feedback_message, evaluation_context);
                            Ok(HardDecision::Block { feedback })
                        }
                        crate::config::actions::Action::Approve { reason } => {
                            let substituted_reason = reason
                                .as_ref()
                                .map(|r| self.substitute_templates(r, evaluation_context));
                            Ok(HardDecision::Approve {
                                reason: substituted_reason,
                            })
                        }
                        crate::config::actions::Action::RunCommand {
                            on_failure,
                            on_failure_feedback,
                            ..
                        } => {
                            // For now, treat run_command with block as immediate block
                            // TODO: Implement actual command execution in Phase 5
                            if matches!(
                                on_failure,
                                crate::config::actions::OnFailureBehavior::Block
                            ) {
                                let feedback = on_failure_feedback
                                    .as_deref()
                                    .unwrap_or("Command execution would be required");
                                let substituted_feedback =
                                    self.substitute_templates(feedback, evaluation_context);
                                Ok(HardDecision::Block {
                                    feedback: substituted_feedback,
                                })
                            } else {
                                continue; // Soft command, keep looking
                            }
                        }
                        _ => continue, // Shouldn't happen for hard actions, but be safe
                    };
                }
            }
        }

        // No hard action found
        Ok(HardDecision::Allow)
    }

    /// Evaluate all conditions for a policy
    fn evaluate_policy_conditions(
        &mut self,
        policy: &crate::config::types::Policy,
        context: &EvaluationContext,
    ) -> Result<bool> {
        // If no conditions, policy always matches
        if policy.conditions.is_empty() {
            return Ok(true);
        }

        // All conditions must match (implicit AND)
        for condition in &policy.conditions {
            // Directly evaluate config condition using 3-primitive model
            let result = self.condition_evaluator.evaluate(condition, context);

            match result {
                crate::engine::conditions::ConditionResult::Match => continue,
                crate::engine::conditions::ConditionResult::NoMatch => return Ok(false),
                crate::engine::conditions::ConditionResult::Error(err) => {
                    eprintln!(
                        "Warning: Condition evaluation error in policy '{}': {}",
                        policy.name, err
                    );
                    return Ok(false); // Graceful degradation
                }
            }
        }

        Ok(true)
    }

    /// Extract feedback message from action
    fn extract_feedback_message(
        &self,
        action: &crate::config::actions::Action,
        context: &EvaluationContext,
    ) -> Option<String> {
        match action {
            crate::config::actions::Action::ProvideFeedback { message, .. } => {
                Some(self.substitute_templates(message, context))
            }
            crate::config::actions::Action::UpdateState { .. } => {
                // State updates don't provide feedback
                None
            }
            _ => None, // Other actions handled elsewhere
        }
    }

    /// Basic template substitution (simplified for Phase 4)
    fn substitute_templates(&self, template: &str, context: &EvaluationContext) -> String {
        let mut result = template.to_string();

        // Substitute basic tool input variables
        if let Some(file_path) = context.tool_input.get("file_path").and_then(|v| v.as_str()) {
            result = result.replace("{{tool_input.file_path}}", file_path);
        }

        if let Some(command) = context.tool_input.get("command").and_then(|v| v.as_str()) {
            result = result.replace("{{tool_input.command}}", command);
        }

        result = result.replace("{{tool_name}}", &context.tool_name);

        result
    }
}

impl Default for PolicyEvaluator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::actions::Action;
    use crate::config::conditions::Condition;
    use crate::config::types::{Policy, PolicyFile, Settings};
    use crate::engine::events::HookEvent;
    use chrono::Utc;
    use pretty_assertions::assert_eq;
    use std::collections::HashMap;

    fn create_test_policy_file() -> PolicyFile {
        PolicyFile {
            schema_version: "1.0".to_string(),
            settings: Settings::default(),
            policies: vec![
                Policy {
                    name: "Test Feedback Policy".to_string(),
                    description: None,
                    hook_event: crate::config::types::HookEventType::PreToolUse,
                    matcher: Some("Edit|Write".to_string()),
                    conditions: vec![Condition::Pattern {
                        field: "tool_input.file_path".to_string(),
                        regex: r"\.rs$".to_string(),
                    }],
                    action: Action::ProvideFeedback {
                        message: "Consider adding tests for Rust files".to_string(),
                        include_context: false,
                    },
                },
                Policy {
                    name: "Block Console Log".to_string(),
                    description: None,
                    hook_event: crate::config::types::HookEventType::PreToolUse,
                    matcher: Some("Edit|Write".to_string()),
                    conditions: vec![Condition::Pattern {
                        field: "tool_input.new_string".to_string(),
                        regex: r"console\.log".to_string(),
                    }],
                    action: Action::BlockWithFeedback {
                        feedback_message: "Remove console.log statements".to_string(),
                        include_context: false,
                    },
                },
            ],
        }
    }

    fn create_test_hook_event() -> HookEvent {
        let mut tool_input = HashMap::new();
        tool_input.insert(
            "file_path".to_string(),
            serde_json::Value::String("src/main.rs".to_string()),
        );
        tool_input.insert(
            "new_string".to_string(),
            serde_json::Value::String("fn main() {\n    console.log(\"test\");\n}".to_string()),
        );

        HookEvent::PreToolUse {
            common: crate::engine::events::CommonEventData {
                session_id: "test-session".to_string(),
                transcript_path: "/tmp/transcript.jsonl".to_string(),
            },
            tool_name: "Edit".to_string(),
            tool_input: serde_json::to_value(tool_input).unwrap(),
        }
    }

    fn create_test_evaluation_context() -> EvaluationContext {
        let mut tool_input = HashMap::new();
        tool_input.insert(
            "file_path".to_string(),
            serde_json::Value::String("src/main.rs".to_string()),
        );
        tool_input.insert(
            "new_string".to_string(),
            serde_json::Value::String("fn main() {\n    console.log(\"test\");\n}".to_string()),
        );

        EvaluationContext {
            event_type: "PreToolUse".to_string(),
            tool_name: "Edit".to_string(),
            tool_input,
            session_id: "test-session".to_string(),
            current_dir: std::path::PathBuf::from("/tmp/test"),
            env_vars: HashMap::new(),
            timestamp: Utc::now(),
            full_session_state: None,
        }
    }

    #[test]
    fn test_policy_evaluator_creation() {
        let _evaluator = PolicyEvaluator::new();
        // Test passes if no panic
    }

    #[test]
    fn test_build_ordered_policy_list() {
        let evaluator = PolicyEvaluator::new();
        let policy_file = create_test_policy_file();
        let hook_event = create_test_hook_event();

        let policy_files = vec![policy_file];
        let ordered_policies = evaluator
            .build_ordered_policy_list(&policy_files, &hook_event)
            .unwrap();

        // Both policies should match PreToolUse event with Edit tool
        assert_eq!(ordered_policies.len(), 2);
        assert_eq!(ordered_policies[0].name, "Test Feedback Policy");
        assert_eq!(ordered_policies[1].name, "Block Console Log");
    }

    #[test]
    fn test_build_ordered_policy_list_no_match() {
        let evaluator = PolicyEvaluator::new();
        let policy_file = create_test_policy_file();

        // Create hook event with non-matching tool
        let hook_event = HookEvent::PreToolUse {
            common: crate::engine::events::CommonEventData {
                session_id: "test-session".to_string(),
                transcript_path: "/tmp/transcript.jsonl".to_string(),
            },
            tool_name: "Bash".to_string(), // Doesn't match Edit|Write matcher
            tool_input: serde_json::json!({"command": "echo test"}),
        };

        let policy_files = vec![policy_file];
        let ordered_policies = evaluator
            .build_ordered_policy_list(&policy_files, &hook_event)
            .unwrap();

        // No policies should match
        assert_eq!(ordered_policies.len(), 0);
    }

    #[test]
    fn test_execute_pass_1_feedback_collection() {
        let mut evaluator = PolicyEvaluator::new();
        let mut policy_file = create_test_policy_file();
        let hook_event = create_test_hook_event();
        let evaluation_context = create_test_evaluation_context();

        // Only include the feedback policy (not the blocking one)
        policy_file.policies = vec![policy_file.policies[0].clone()];

        let policy_files = vec![policy_file];
        let feedback_collection = evaluator
            .execute_pass_1(&policy_files, &hook_event, &evaluation_context)
            .unwrap();

        assert_eq!(feedback_collection.feedback_messages.len(), 1);
        assert_eq!(
            feedback_collection.feedback_messages[0],
            "Consider adding tests for Rust files"
        );
    }

    #[test]
    fn test_execute_pass_2_hard_decision() {
        let mut evaluator = PolicyEvaluator::new();
        let mut policy_file = create_test_policy_file();
        let hook_event = create_test_hook_event();
        let evaluation_context = create_test_evaluation_context();

        // Only include the blocking policy
        policy_file.policies = vec![policy_file.policies[1].clone()];

        let policy_files = vec![policy_file];
        let hard_decision = evaluator
            .execute_pass_2(&policy_files, &hook_event, &evaluation_context)
            .unwrap();

        match hard_decision {
            HardDecision::Block { feedback } => {
                assert_eq!(feedback, "Remove console.log statements");
            }
            _ => panic!("Expected Block decision"),
        }
    }

    #[test]
    fn test_execute_pass_2_no_hard_action() {
        let mut evaluator = PolicyEvaluator::new();
        let mut policy_file = create_test_policy_file();
        let hook_event = create_test_hook_event();
        let evaluation_context = create_test_evaluation_context();

        // Only include the feedback policy (soft action)
        policy_file.policies = vec![policy_file.policies[0].clone()];

        let policy_files = vec![policy_file];
        let hard_decision = evaluator
            .execute_pass_2(&policy_files, &hook_event, &evaluation_context)
            .unwrap();

        match hard_decision {
            HardDecision::Allow => {
                // Expected
            }
            _ => panic!("Expected Allow decision"),
        }
    }

    #[test]
    fn test_full_evaluation_with_block() {
        let mut evaluator = PolicyEvaluator::new();
        let policy_file = create_test_policy_file();
        let hook_event = create_test_hook_event();
        let evaluation_context = create_test_evaluation_context();

        let policy_files = vec![policy_file];
        let result = evaluator
            .evaluate(&policy_files, &hook_event, &evaluation_context)
            .unwrap();

        // Should be blocked due to console.log
        match result.decision {
            PolicyDecision::Block { feedback } => {
                assert!(feedback.contains("Remove console.log statements"));
                assert!(feedback.contains("Consider adding tests for Rust files"));
            }
            _ => panic!("Expected Block decision"),
        }

        // Should have collected feedback from both policies
        assert_eq!(result.feedback_messages.len(), 1);
        assert_eq!(
            result.feedback_messages[0],
            "Consider adding tests for Rust files"
        );
    }

    #[test]
    fn test_full_evaluation_feedback_only() {
        let mut evaluator = PolicyEvaluator::new();
        let mut policy_file = create_test_policy_file();

        // Remove the blocking policy, keep only feedback
        policy_file.policies.pop();

        let hook_event = create_test_hook_event();
        let evaluation_context = create_test_evaluation_context();

        let policy_files = vec![policy_file];
        let result = evaluator
            .evaluate(&policy_files, &hook_event, &evaluation_context)
            .unwrap();

        // Should be blocked due to feedback (converted to block)
        match result.decision {
            PolicyDecision::Block { feedback } => {
                assert_eq!(feedback, "Consider adding tests for Rust files");
            }
            _ => panic!("Expected Block decision from feedback"),
        }

        assert_eq!(result.feedback_messages.len(), 1);
    }

    #[test]
    fn test_full_evaluation_allow() {
        let mut evaluator = PolicyEvaluator::new();
        let policy_file = PolicyFile {
            schema_version: "1.0".to_string(),
            settings: Settings::default(),
            policies: vec![], // No policies
        };

        let hook_event = create_test_hook_event();
        let evaluation_context = create_test_evaluation_context();

        let policy_files = vec![policy_file];
        let result = evaluator
            .evaluate(&policy_files, &hook_event, &evaluation_context)
            .unwrap();

        // Should allow when no policies match
        match result.decision {
            PolicyDecision::Allow => {
                // Expected
            }
            _ => panic!("Expected Allow decision"),
        }

        assert_eq!(result.feedback_messages.len(), 0);
    }

    #[test]
    fn test_template_substitution() {
        let evaluator = PolicyEvaluator::new();
        let context = create_test_evaluation_context();

        let template = "File: {{tool_input.file_path}}, Tool: {{tool_name}}";
        let result = evaluator.substitute_templates(template, &context);

        assert_eq!(result, "File: src/main.rs, Tool: Edit");
    }
}
