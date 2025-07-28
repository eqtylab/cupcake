use crate::config::types::ComposedPolicy;
use crate::engine::conditions::{ConditionEvaluator, EvaluationContext};
use crate::engine::response::EngineDecision;
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
    /// Allow operation (either no hard action found or explicit allow)
    Allow { reason: Option<String> },
    /// Block operation with feedback
    Block { feedback: String },
    /// Ask user for confirmation
    Ask { reason: String },
}

/// Complete evaluation result combining both passes
#[derive(Debug, Clone)]
pub struct EvaluationResult {
    /// Final decision (from Pass 2)
    pub decision: EngineDecision,
    /// All feedback collected (from Pass 1)
    pub feedback_messages: Vec<String>,
    /// Policies that matched and their actions
    pub matched_policies: Vec<MatchedPolicy>,
}

/// A policy that matched during evaluation
#[derive(Debug, Clone)]
pub struct MatchedPolicy {
    /// Policy name for debugging
    pub name: String,
    /// The action to execute
    pub action: crate::config::actions::Action,
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
        // Single evaluation pass: evaluate each policy once and cache results
        let ordered_policies = self.build_ordered_policy_list(policies, hook_event)?;
        let mut matched_policies = Vec::new();
        let mut evaluation_cache = std::collections::HashMap::new();
        
        // Evaluate each policy exactly once
        for policy in &ordered_policies {
            let conditions_match = self.evaluate_policy_conditions(policy, evaluation_context)?;
            evaluation_cache.insert(policy.name.clone(), conditions_match);
            
            if conditions_match {
                matched_policies.push(MatchedPolicy {
                    name: policy.name.clone(),
                    action: policy.action.clone(),
                });
            }
        }

        // Pass 1: Collect all feedback from soft actions (using cached results)
        let feedback_collection = self.execute_pass_1_cached(&ordered_policies, &evaluation_cache, evaluation_context)?;

        // Pass 2: Find first hard action decision (using cached results)
        let hard_decision = self.execute_pass_2_cached(&ordered_policies, &evaluation_cache, evaluation_context)?;

        // Combine results
        let decision = match hard_decision {
            HardDecision::Allow { reason } => {
                // Either no hard action found or explicit allow action
                EngineDecision::Allow { reason }
            }
            HardDecision::Block { feedback } => {
                // Combine hard block feedback with all collected feedback
                let mut all_feedback = vec![feedback];
                all_feedback.extend(feedback_collection.feedback_messages.clone());
                let combined_feedback = all_feedback.join("\n");
                EngineDecision::Block {
                    feedback: combined_feedback,
                }
            }
            HardDecision::Ask { reason } => EngineDecision::Ask { reason },
        };

        Ok(EvaluationResult {
            decision,
            feedback_messages: feedback_collection.feedback_messages,
            matched_policies,
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

            // Check if policy matcher applies
            if policy.matcher.is_empty() {
                // Empty string matcher: only matches non-tool events
                if tool_name.is_some() {
                    continue; // Skip this policy for tool events
                }
            } else if policy.matcher == "*" {
                // Special case: "*" matches everything (both tool and non-tool events)
                // Continue to next check
            } else {
                // Non-empty, non-wildcard matcher: only matches tool events with regex
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
                } else {
                    // Non-tool event with non-wildcard matcher: no match
                    continue;
                }
            }

            ordered_policies.push(policy);
        }

        Ok(ordered_policies)
    }

    /// Execute Pass 1: Collect all feedback from soft actions (using cached evaluation results)
    fn execute_pass_1_cached(
        &mut self,
        ordered_policies: &[&ComposedPolicy],
        evaluation_cache: &std::collections::HashMap<String, bool>,
        evaluation_context: &EvaluationContext,
    ) -> Result<FeedbackCollection> {
        let mut feedback_messages = Vec::new();

        for policy in ordered_policies {
            // Use cached evaluation result instead of re-evaluating
            let conditions_match = evaluation_cache.get(&policy.name).copied().unwrap_or(false);

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


    /// Execute Pass 2: Find first hard action decision (using cached evaluation results)
    fn execute_pass_2_cached(
        &mut self,
        ordered_policies: &[&ComposedPolicy],
        evaluation_cache: &std::collections::HashMap<String, bool>,
        evaluation_context: &EvaluationContext,
    ) -> Result<HardDecision> {
        for policy in ordered_policies {
            // Use cached evaluation result instead of re-evaluating
            let conditions_match = evaluation_cache.get(&policy.name).copied().unwrap_or(false);

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
                        crate::config::actions::Action::Allow { reason } => {
                            let substituted_reason = reason
                                .as_ref()
                                .map(|r| self.substitute_templates(r, evaluation_context));
                            Ok(HardDecision::Allow {
                                reason: substituted_reason,
                            })
                        }
                        crate::config::actions::Action::Ask { reason } => {
                            let substituted_reason = self.substitute_templates(reason, evaluation_context);
                            Ok(HardDecision::Ask {
                                reason: substituted_reason,
                            })
                        }
                        crate::config::actions::Action::RunCommand {
                            on_failure, ..
                        } => {
                            // RunCommand actions are executed in the action phase
                            // For now, we continue to find other hard actions
                            // The action phase will handle the actual blocking decision
                            if matches!(
                                on_failure,
                                crate::config::actions::OnFailureBehavior::Block
                            ) {
                                // Skip this for now - let action phase handle it
                                continue;
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
        Ok(HardDecision::Allow { reason: None })
    }


    /// Evaluate all conditions for a policy
    fn evaluate_policy_conditions(
        &mut self,
        policy: &ComposedPolicy,
        context: &EvaluationContext,
    ) -> Result<bool> {
        // Debug: Track policy condition evaluations
        eprintln!("Debug: Evaluating policy conditions for '{}'", policy.name);
        
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
            crate::config::actions::Action::InjectContext { context: ctx, .. } => {
                Some(self.substitute_templates(ctx, context))
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
