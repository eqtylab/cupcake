use super::context::ExecutionContextBuilder;
use crate::config::types::{ComposedPolicy, Settings};
use crate::engine::actions::{ActionContext, ActionExecutor, ActionResult};
use crate::engine::evaluation::{MatchedPolicy, PolicyEvaluator};
use crate::engine::events::AgentEvent;
use crate::engine::response::EngineDecision;
use crate::{Result, tracing::{debug, warn}};

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
    pub suppress_output: bool,
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
    /// The EngineRunner now creates its own contexts internally for single-source-of-truth
    pub fn run(
        &mut self,
        policies: &[ComposedPolicy],
        agent_event: &AgentEvent,
    ) -> Result<EngineResult> {
        // Create contexts internally - single authoritative data flow
        let context_builder = ExecutionContextBuilder::new();
        let evaluation_context = context_builder.build_evaluation_context(agent_event);
        let action_context = context_builder.build_action_context(agent_event);

        // Extract ClaudeCodeEvent for legacy interface
        let AgentEvent::ClaudeCode(hook_event) = agent_event;
        // Evaluate policies
        let evaluation_result =
            self.policy_evaluator
                .evaluate(policies, hook_event, &evaluation_context)?;

        debug!(?evaluation_result.decision, "Evaluation complete");
        if !evaluation_result.feedback_messages.is_empty() {
            debug!(?evaluation_result.feedback_messages, "Collected feedback messages");
        }

        // Execute actions for matched policies
        let action_results =
            self.execute_matched_actions(&evaluation_result.matched_policies, &action_context)?;

        // Process action results
        let mut final_decision = evaluation_result.decision.clone();
        let mut context_to_inject = Vec::new();
        let mut suppress_output = false;
        let is_context_injection_event = hook_event.event_name() == "UserPromptSubmit"
            || hook_event.event_name() == "SessionStart"
            || hook_event.event_name() == "PreCompact";

        for (_policy_name, result) in action_results.iter() {
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
                ActionResult::Allow { reason } => {
                    final_decision = EngineDecision::Allow {
                        reason: reason.clone(),
                    };
                    break;
                }
                ActionResult::Success {
                    feedback: Some(ctx),
                    ..
                } if is_context_injection_event => {
                    context_to_inject.push(ctx.clone());
                }
                _ => {}
            }
        }

        // Check suppress_output from ALL matched policies, not just the winning one
        // This ensures soft actions (provide_feedback, inject_context) can also suppress output
        for matched_policy in &evaluation_result.matched_policies {
            let policy_suppress = match &matched_policy.action {
                crate::config::actions::Action::Allow {
                    suppress_output, ..
                } => *suppress_output,
                crate::config::actions::Action::Ask {
                    suppress_output, ..
                } => *suppress_output,
                crate::config::actions::Action::BlockWithFeedback {
                    suppress_output, ..
                } => *suppress_output,
                crate::config::actions::Action::ProvideFeedback {
                    suppress_output, ..
                } => *suppress_output,
                crate::config::actions::Action::RunCommand {
                    suppress_output, ..
                } => *suppress_output,
                crate::config::actions::Action::InjectContext {
                    suppress_output, ..
                } => *suppress_output,
                crate::config::actions::Action::Conditional { .. } => false, // Conditional doesn't have suppress_output
            };

            // If any matched policy wants to suppress output, honor that
            if policy_suppress {
                suppress_output = true;
                break;
            }
        }

        Ok(EngineResult {
            final_decision,
            feedback_messages: evaluation_result.feedback_messages,
            context_to_inject,
            suppress_output,
        })
    }

    fn execute_matched_actions(
        &mut self,
        matched_policies: &[MatchedPolicy],
        action_context: &ActionContext,
    ) -> Result<Vec<(String, ActionResult)>> {
        let mut results = Vec::new();

        debug!(
            policy_count = matched_policies.len(),
            "Executing actions for matched policies"
        );

        for matched_policy in matched_policies {
            debug!(
                policy_name = %matched_policy.name,
                action = ?matched_policy.action,
                "Executing action for policy"
            );

            let result = self
                .action_executor
                .execute(&matched_policy.action, action_context);

            match &result {
                ActionResult::Success { feedback, .. } => {
                    if let Some(msg) = feedback {
                        debug!(feedback = %msg, "Action feedback");
                    }
                }
                ActionResult::Block { feedback } => {
                    debug!(feedback = %feedback, "Action execution resulted in block");
                }
                ActionResult::Allow { .. } => {
                    debug!("Action execution resulted in allow");
                }
                ActionResult::Ask { reason } => {
                    debug!(reason = %reason, "Action execution resulted in ask");
                }
                ActionResult::Error { message } => {
                    warn!(
                        policy = %matched_policy.name,
                        error = %message,
                        "Error executing action for policy"
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

    #[test]
    fn test_engine_runner_with_agent_event() {
        use crate::engine::events::{
            claude_code::SessionStartPayload, ClaudeCodeEvent, CommonEventData,
        };

        let settings = Settings::default();
        let mut engine = EngineRunner::new(settings, false);

        let event = AgentEvent::ClaudeCode(ClaudeCodeEvent::SessionStart(SessionStartPayload {
            common: CommonEventData {
                session_id: "test-123".to_string(),
                transcript_path: "/tmp/transcript".to_string(),
                cwd: "/home/user".to_string(),
            },
            source: crate::engine::events::SessionSource::Startup,
        }));

        let policies = vec![];
        let result = engine.run(&policies, &event);
        assert!(result.is_ok());
    }
}
