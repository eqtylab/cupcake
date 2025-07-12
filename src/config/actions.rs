use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::conditions::Condition;

/// Action types for policy responses
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Action {
    /// Provide feedback without blocking (soft action)
    ProvideFeedback {
        message: String,
        #[serde(default)]
        include_context: bool,
    },

    /// Block operation with feedback to Claude (hard action)
    BlockWithFeedback {
        feedback_message: String,
        #[serde(default)]
        include_context: bool,
    },

    /// Auto-approve operation (hard action)
    Approve {
        #[serde(skip_serializing_if = "Option::is_none")]
        reason: Option<String>,
    },

    /// Run a command (can be soft or hard based on on_failure)
    RunCommand {
        command: String,
        #[serde(default)]
        on_failure: OnFailureBehavior,
        #[serde(skip_serializing_if = "Option::is_none")]
        on_failure_feedback: Option<String>,
        #[serde(default)]
        background: bool,
        #[serde(default)]
        timeout_seconds: Option<u32>,
    },

    /// Update session state with custom data
    UpdateState {
        #[serde(skip_serializing_if = "Option::is_none")]
        event: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        key: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        value: Option<serde_json::Value>,
        #[serde(skip_serializing_if = "Option::is_none")]
        data: Option<HashMap<String, serde_json::Value>>,
    },

    /// Conditional action based on runtime condition
    Conditional {
        #[serde(rename = "if")]
        if_condition: Condition,
        #[serde(rename = "then")]
        then_action: Box<Action>,
        #[serde(rename = "else", skip_serializing_if = "Option::is_none")]
        else_action: Option<Box<Action>>,
    },
}

/// Behavior when RunCommand fails
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum OnFailureBehavior {
    /// Continue processing (keeps action as "soft")
    Continue,
    /// Block operation (makes action "hard")
    Block,
}

impl Default for OnFailureBehavior {
    fn default() -> Self {
        Self::Continue
    }
}

/// Classification of actions for two-pass evaluation
#[derive(Debug, Clone, PartialEq)]
pub enum ActionType {
    /// Soft actions that provide feedback
    Soft,
    /// Hard actions that make decisions (block/approve)
    Hard,
}

impl Action {
    /// Determine if this action is soft (feedback) or hard (decision)
    pub fn action_type(&self) -> ActionType {
        match self {
            Action::ProvideFeedback { .. } => ActionType::Soft,
            Action::BlockWithFeedback { .. } => ActionType::Hard,
            Action::Approve { .. } => ActionType::Hard,
            Action::RunCommand { on_failure, .. } => match on_failure {
                OnFailureBehavior::Continue => ActionType::Soft,
                OnFailureBehavior::Block => ActionType::Hard,
            },
            Action::UpdateState { .. } => ActionType::Soft,
            Action::Conditional {
                then_action,
                else_action,
                ..
            } => {
                // Conditional is hard if either branch is hard
                let then_type = then_action.action_type();
                let else_type = else_action
                    .as_ref()
                    .map(|a| a.action_type())
                    .unwrap_or(ActionType::Soft);

                if then_type == ActionType::Hard || else_type == ActionType::Hard {
                    ActionType::Hard
                } else {
                    ActionType::Soft
                }
            }
        }
    }

    /// Check if this action requires command execution
    pub fn requires_execution(&self) -> bool {
        matches!(self, Action::RunCommand { .. })
    }

    /// Check if this action modifies state
    pub fn modifies_state(&self) -> bool {
        matches!(self, Action::UpdateState { .. })
    }

    /// Check if this action is a "soft" action (feedback only)
    pub fn is_soft_action(&self) -> bool {
        self.action_type() == ActionType::Soft
    }

    /// Check if this action is a "hard" action (makes decisions)
    pub fn is_hard_action(&self) -> bool {
        self.action_type() == ActionType::Hard
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_action_type_classification() {
        let soft_action = Action::ProvideFeedback {
            message: "Test feedback".to_string(),
            include_context: false,
        };
        assert_eq!(soft_action.action_type(), ActionType::Soft);

        let hard_action = Action::BlockWithFeedback {
            feedback_message: "Blocked".to_string(),
            include_context: false,
        };
        assert_eq!(hard_action.action_type(), ActionType::Hard);

        let soft_command = Action::RunCommand {
            command: "echo test".to_string(),
            on_failure: OnFailureBehavior::Continue,
            on_failure_feedback: None,
            background: false,
            timeout_seconds: None,
        };
        assert_eq!(soft_command.action_type(), ActionType::Soft);

        let hard_command = Action::RunCommand {
            command: "cargo test".to_string(),
            on_failure: OnFailureBehavior::Block,
            on_failure_feedback: Some("Tests failed".to_string()),
            background: false,
            timeout_seconds: None,
        };
        assert_eq!(hard_command.action_type(), ActionType::Hard);
    }

    #[test]
    fn test_action_serialization() {
        let action = Action::ProvideFeedback {
            message: "Use <Button> instead of <button>".to_string(),
            include_context: true,
        };

        let yaml = serde_yaml_ng::to_string(&action).unwrap();
        let deserialized: Action = serde_yaml_ng::from_str(&yaml).unwrap();

        match deserialized {
            Action::ProvideFeedback {
                message,
                include_context,
            } => {
                assert_eq!(message, "Use <Button> instead of <button>");
                assert!(include_context);
            }
            _ => panic!("Wrong action type"),
        }
    }

    #[test]
    fn test_on_failure_behavior_default() {
        let behavior = OnFailureBehavior::default();
        assert_eq!(behavior, OnFailureBehavior::Continue);
    }

    #[test]
    fn test_conditional_action_hard_classification() {
        let conditional = Action::Conditional {
            if_condition: Condition::Check {
                command: "echo 'test'".to_string(),
                expect_success: true,
            },
            then_action: Box::new(Action::BlockWithFeedback {
                feedback_message: "Blocked".to_string(),
                include_context: false,
            }),
            else_action: Some(Box::new(Action::Approve { reason: None })),
        };

        assert_eq!(conditional.action_type(), ActionType::Hard);
    }

    #[test]
    fn test_action_requirements() {
        let run_command = Action::RunCommand {
            command: "test".to_string(),
            on_failure: OnFailureBehavior::Continue,
            on_failure_feedback: None,
            background: false,
            timeout_seconds: None,
        };
        assert!(run_command.requires_execution());

        let update_state = Action::UpdateState {
            event: Some("test".to_string()),
            key: None,
            value: None,
            data: None,
        };
        assert!(update_state.modifies_state());
    }
}
