use crate::config::types::ComposedPolicy;
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
        policies: &[ComposedPolicy],
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
        policies: &'a [ComposedPolicy],
        hook_event: &crate::engine::events::HookEvent,
    ) -> Result<Vec<&'a ComposedPolicy>> {
        let mut ordered_policies = Vec::new();
        let hook_event_name = hook_event.event_name();
        let tool_name = hook_event.tool_name();

        for policy in policies {
            // Check if policy applies to this hook event
            let policy_event_name = policy.hook_event.to_string();

            if policy_event_name != hook_event_name {
                continue;
            }

            // Check if policy matcher applies to this tool (for PreToolUse/PostToolUse)
            if let Some(tool) = tool_name {
                let matcher_regex = regex::Regex::new(&policy.matcher).map_err(|e| {
                    crate::CupcakeError::Config(format!(
                        "Invalid matcher regex '{}': {}",
                        policy.matcher, e
                    ))
                })?;

                if !matcher_regex.is_match(tool) {
                    continue;
                }
            }

            ordered_policies.push(policy);
        }

        Ok(ordered_policies)
    }

    /// Execute Pass 1: Collect all feedback from soft actions
    fn execute_pass_1(
        &mut self,
        policies: &[ComposedPolicy],
        hook_event: &crate::engine::events::HookEvent,
        evaluation_context: &EvaluationContext,
    ) -> Result<FeedbackCollection> {
        let mut feedback_messages = Vec::new();
        let ordered_policies = self.build_ordered_policy_list(policies, hook_event)?;

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
        policies: &[ComposedPolicy],
        hook_event: &crate::engine::events::HookEvent,
        evaluation_context: &EvaluationContext,
    ) -> Result<HardDecision> {
        let ordered_policies = self.build_ordered_policy_list(policies, hook_event)?;

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
        policy: &ComposedPolicy,
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

    #[test]
    fn test_policy_evaluator_creation() {
        let _evaluator = PolicyEvaluator::new();
        // Test passes if no panic - main functionality tested in integration tests
    }
}
