use crate::config::types::{ComposedPolicy, Settings};
use crate::engine::actions::{ActionContext, ActionExecutor, ActionResult};
use crate::engine::conditions::EvaluationContext;
use crate::engine::evaluation::{MatchedPolicy, PolicyEvaluator};
use crate::engine::events::HookEvent;
use crate::engine::response::EngineDecision;
use crate::Result;

/// Orchestrates policy evaluation and action execution
pub struct EngineRunner {
    policy_evaluator: PolicyEvaluator,
    action_executor: ActionExecutor,
    debug: bool,
}

/// Result of engine execution
pub struct EngineResult {
    pub final_decision: EngineDecision,
    pub feedback_messages: Vec<String>,
    pub context_to_inject: Vec<String>,
}

impl EngineRunner {
    pub fn new(settings: Settings, debug: bool) -> Self {
        Self {
            policy_evaluator: PolicyEvaluator::new(),
            action_executor: ActionExecutor::with_settings(settings),
            debug,
        }
    }

    /// Run policy evaluation and action execution
    pub fn run(
        &mut self,
        policies: &[ComposedPolicy],
        hook_event: &HookEvent,
        evaluation_context: &EvaluationContext,
        action_context: &ActionContext,
    ) -> Result<EngineResult> {
        // Evaluate policies
        let evaluation_result = self.policy_evaluator.evaluate(
            policies,
            hook_event,
            evaluation_context,
        )?;

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

        // Execute actions for matched policies
        let action_results = self.execute_matched_actions(
            &evaluation_result.matched_policies,
            action_context,
        )?;

        // Process action results
        let mut final_decision = evaluation_result.decision.clone();
        let mut context_to_inject = Vec::new();
        let is_user_prompt_submit = hook_event.event_name() == "UserPromptSubmit";

        for (_policy_name, result) in &action_results {
            match result {
                ActionResult::Block { feedback } => {
                    final_decision = EngineDecision::Block {
                        feedback: feedback.clone(),
                    };
                    break;
                }
                ActionResult::Ask { reason } => {
                    final_decision = EngineDecision::Ask {
                        reason: reason.clone(),
                    };
                    break;
                }
                ActionResult::Success {
                    feedback: Some(ctx),
                    ..
                } if is_user_prompt_submit => {
                    context_to_inject.push(ctx.clone());
                }
                _ => {}
            }
        }

        Ok(EngineResult {
            final_decision,
            feedback_messages: evaluation_result.feedback_messages,
            context_to_inject,
        })
    }

    fn execute_matched_actions(
        &mut self,
        matched_policies: &[MatchedPolicy],
        action_context: &ActionContext,
    ) -> Result<Vec<(String, ActionResult)>> {
        let mut results = Vec::new();

        if self.debug {
            eprintln!(
                "Debug: Executing actions for {} matched policies",
                matched_policies.len()
            );
        }

        for matched_policy in matched_policies {
            if self.debug {
                eprintln!(
                    "Debug: Executing action for policy '{}': {:?}",
                    matched_policy.name, matched_policy.action
                );
            }

            let result = self.action_executor.execute(&matched_policy.action, action_context);

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
                }
            }

            results.push((matched_policy.name.clone(), result));
        }

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::types::Settings;

    #[test]
    fn test_engine_runner_creation() {
        let settings = Settings::default();
        let engine = EngineRunner::new(settings, true);
        assert!(engine.debug);
    }
}