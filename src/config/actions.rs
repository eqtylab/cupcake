use serde::{Deserialize, Serialize};

use super::conditions::Condition;

/// Command specification for secure execution
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum CommandSpec {
    /// Kubernetes-style array command (secure, no shell)
    Array(Box<ArrayCommandSpec>),
    /// Shell script executed via /bin/sh (requires allow_shell setting)
    Shell(ShellCommandSpec),
}

/// Kubernetes-style command specification with composition operators
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArrayCommandSpec {
    /// Command to execute (e.g., ["/usr/bin/git"])
    pub command: Vec<String>,

    /// Arguments to pass to command (e.g., ["status", "-s"])
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<Vec<String>>,

    /// Working directory for execution
    #[serde(skip_serializing_if = "Option::is_none", rename = "workingDir")]
    pub working_dir: Option<String>,

    /// Environment variables
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env: Option<Vec<EnvVar>>,

    // Composition operators for shell-free command chaining
    /// Pipe stdout to subsequent commands
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pipe: Option<Vec<PipeCommand>>,

    /// Redirect stdout to file (truncate)
    #[serde(skip_serializing_if = "Option::is_none", rename = "redirectStdout")]
    pub redirect_stdout: Option<String>,

    /// Append stdout to file
    #[serde(skip_serializing_if = "Option::is_none", rename = "appendStdout")]
    pub append_stdout: Option<String>,

    /// Redirect stderr to file
    #[serde(skip_serializing_if = "Option::is_none", rename = "redirectStderr")]
    pub redirect_stderr: Option<String>,

    /// Merge stderr into stdout
    #[serde(skip_serializing_if = "Option::is_none", rename = "mergeStderr")]
    pub merge_stderr: Option<bool>,

    /// Commands to run on success (exit code 0)
    #[serde(skip_serializing_if = "Option::is_none", rename = "onSuccess")]
    pub on_success: Option<Vec<ArrayCommandSpec>>,

    /// Commands to run on failure (exit code != 0)
    #[serde(skip_serializing_if = "Option::is_none", rename = "onFailure")]
    pub on_failure: Option<Vec<ArrayCommandSpec>>,
}

/// Environment variable specification
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnvVar {
    pub name: String,
    pub value: String,
}

/// Pipe command specification for chaining
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PipeCommand {
    /// Command and args as array (e.g., ["grep", "-v", "WARNING"])
    pub cmd: Vec<String>,
}

/// Shell script specification for legacy/complex scripts
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShellCommandSpec {
    /// Shell script to execute via /bin/sh -c
    /// WARNING: This bypasses security protections and requires allow_shell=true
    pub script: String,
}

/// Dynamic context specification for from_command
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DynamicContextSpec {
    /// Command specification for generating context
    pub spec: CommandSpec,
    /// Behavior when command fails
    #[serde(default)]
    pub on_failure: OnFailureBehavior,
}

/// Action types for policy responses
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Action {
    /// Provide feedback without blocking (soft action)
    ProvideFeedback {
        message: String,
        #[serde(default)]
        include_context: bool,
        #[serde(default, skip_serializing_if = "is_false")]
        suppress_output: bool,
    },

    /// Block operation with feedback to Claude (hard action)
    BlockWithFeedback {
        feedback_message: String,
        #[serde(default)]
        include_context: bool,
        #[serde(default, skip_serializing_if = "is_false")]
        suppress_output: bool,
    },

    /// Auto-allow operation bypassing permission system (hard action)
    Allow {
        #[serde(skip_serializing_if = "Option::is_none")]
        reason: Option<String>,
        #[serde(default, skip_serializing_if = "is_false")]
        suppress_output: bool,
    },

    /// Request user confirmation for operation (hard action)
    Ask {
        reason: String,
        #[serde(default, skip_serializing_if = "is_false")]
        suppress_output: bool,
    },

    /// Run a command (can be soft or hard based on on_failure)
    RunCommand {
        /// Command specification for secure execution
        spec: CommandSpec,
        #[serde(default)]
        on_failure: OnFailureBehavior,
        #[serde(skip_serializing_if = "Option::is_none")]
        on_failure_feedback: Option<String>,
        #[serde(default)]
        background: bool,
        #[serde(default)]
        timeout_seconds: Option<u32>,
        #[serde(default, skip_serializing_if = "is_false")]
        suppress_output: bool,
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

    /// Inject context into Claude's awareness (UserPromptSubmit and SessionStart only)
    InjectContext {
        /// Static context to inject (mutually exclusive with from_command)
        #[serde(skip_serializing_if = "Option::is_none")]
        context: Option<String>,
        /// Dynamic context from command execution (mutually exclusive with context)
        #[serde(skip_serializing_if = "Option::is_none")]
        from_command: Option<Box<DynamicContextSpec>>,
        /// Whether to use stdout method (true) or JSON method (false)
        #[serde(default = "default_use_stdout")]
        use_stdout: bool,
        #[serde(default, skip_serializing_if = "is_false")]
        suppress_output: bool,
    },
}

/// Helper function for serde skip_serializing_if
fn is_false(b: &bool) -> bool {
    !*b
}

/// Returns the default method for context injection.
///
/// The `stdout` method is chosen as the default because it is simple and
/// does not require additional parsing or processing. This makes it
/// suitable for most use cases where the injected context is directly
/// consumed by a process or script.
///
/// The JSON method, on the other hand, should be used when the context
/// needs to be structured or when it will be consumed by a system that
/// expects JSON-formatted input. This method provides more flexibility
/// but requires additional handling to parse the JSON data.
fn default_use_stdout() -> bool {
    true // Default to simple stdout method
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
    /// Create a ProvideFeedback action with sensible defaults
    pub fn provide_feedback(message: impl Into<String>) -> Self {
        Action::ProvideFeedback {
            message: message.into(),
            include_context: false,
            suppress_output: false,
        }
    }

    /// Create a BlockWithFeedback action with sensible defaults
    pub fn block_with_feedback(feedback_message: impl Into<String>) -> Self {
        Action::BlockWithFeedback {
            feedback_message: feedback_message.into(),
            include_context: false,
            suppress_output: false,
        }
    }

    /// Create an Allow action with sensible defaults
    pub fn allow() -> Self {
        Action::Allow {
            reason: None,
            suppress_output: false,
        }
    }

    /// Create an Allow action with a reason
    pub fn allow_with_reason(reason: impl Into<String>) -> Self {
        Action::Allow {
            reason: Some(reason.into()),
            suppress_output: false,
        }
    }

    /// Create an Ask action
    pub fn ask(reason: impl Into<String>) -> Self {
        Action::Ask {
            reason: reason.into(),
            suppress_output: false,
        }
    }

    /// Create an InjectContext action with static content
    pub fn inject_context(context: impl Into<String>) -> Self {
        Action::InjectContext {
            context: Some(context.into()),
            from_command: None,
            use_stdout: true,
            suppress_output: false,
        }
    }

    /// Create an InjectContext action with dynamic content from command
    pub fn inject_context_from_command(spec: CommandSpec, on_failure: OnFailureBehavior) -> Self {
        Action::InjectContext {
            context: None,
            from_command: Some(Box::new(DynamicContextSpec { spec, on_failure })),
            use_stdout: true,
            suppress_output: false,
        }
    }

    /// Create a RunCommand action with basic array command
    pub fn run_command(command: Vec<String>) -> Self {
        Action::RunCommand {
            spec: CommandSpec::Array(Box::new(ArrayCommandSpec {
                command,
                args: None,
                working_dir: None,
                env: None,
                pipe: None,
                redirect_stdout: None,
                append_stdout: None,
                redirect_stderr: None,
                merge_stderr: None,
                on_success: None,
                on_failure: None,
            })),
            on_failure: OnFailureBehavior::Continue,
            on_failure_feedback: None,
            background: false,
            timeout_seconds: None,
            suppress_output: false,
        }
    }

    /// Create a RunCommand action with shell script
    pub fn run_shell(script: impl Into<String>) -> Self {
        Action::RunCommand {
            spec: CommandSpec::Shell(ShellCommandSpec {
                script: script.into(),
            }),
            on_failure: OnFailureBehavior::Continue,
            on_failure_feedback: None,
            background: false,
            timeout_seconds: None,
            suppress_output: false,
        }
    }

    /// Set suppress_output to true for any action
    pub fn with_suppress_output(mut self) -> Self {
        match &mut self {
            Action::ProvideFeedback { suppress_output, .. } => *suppress_output = true,
            Action::BlockWithFeedback { suppress_output, .. } => *suppress_output = true,
            Action::Allow { suppress_output, .. } => *suppress_output = true,
            Action::Ask { suppress_output, .. } => *suppress_output = true,
            Action::RunCommand { suppress_output, .. } => *suppress_output = true,
            Action::InjectContext { suppress_output, .. } => *suppress_output = true,
            Action::Conditional { .. } => {} // Conditional doesn't have suppress_output
        }
        self
    }

    /// Set include_context to true for feedback actions
    pub fn with_context(mut self) -> Self {
        match &mut self {
            Action::ProvideFeedback { include_context, .. } => *include_context = true,
            Action::BlockWithFeedback { include_context, .. } => *include_context = true,
            _ => {} // Other actions don't have include_context
        }
        self
    }

    /// Set on_failure to Block for RunCommand actions (makes them hard)
    pub fn with_blocking_failure(mut self) -> Self {
        if let Action::RunCommand { on_failure, .. } = &mut self {
            *on_failure = OnFailureBehavior::Block;
        }
        self
    }

    /// Set failure feedback message for RunCommand actions
    pub fn with_failure_feedback(mut self, feedback: impl Into<String>) -> Self {
        if let Action::RunCommand { on_failure_feedback, .. } = &mut self {
            *on_failure_feedback = Some(feedback.into());
        }
        self
    }

    /// Determine if this action is soft (feedback) or hard (decision)
    pub fn action_type(&self) -> ActionType {
        match self {
            Action::ProvideFeedback { .. } => ActionType::Soft,
            Action::BlockWithFeedback { .. } => ActionType::Hard,
            Action::Allow { .. } => ActionType::Hard,
            Action::Ask { .. } => ActionType::Hard,
            Action::RunCommand { on_failure, .. } => match on_failure {
                OnFailureBehavior::Continue => ActionType::Soft,
                OnFailureBehavior::Block => ActionType::Hard,
            },
            Action::InjectContext { .. } => ActionType::Soft,
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
        let soft_action = Action::provide_feedback("Test feedback");
        assert_eq!(soft_action.action_type(), ActionType::Soft);

        let hard_action = Action::block_with_feedback("Blocked");
        assert_eq!(hard_action.action_type(), ActionType::Hard);

        let soft_command = Action::run_command(vec!["echo".to_string()]);
        assert_eq!(soft_command.action_type(), ActionType::Soft);

        let hard_command = Action::run_command(vec!["cargo".to_string()])
            .with_blocking_failure()
            .with_failure_feedback("Tests failed");
        assert_eq!(hard_command.action_type(), ActionType::Hard);

        let ask_action = Action::ask("Please confirm this operation");
        assert_eq!(ask_action.action_type(), ActionType::Hard);
        assert!(ask_action.is_hard_action());
    }

    #[test]
    fn test_builder_pattern() {
        // Test the new builder pattern that eliminates brittle field initialization
        let silent_feedback = Action::provide_feedback("Test message")
            .with_suppress_output()
            .with_context();
        
        match silent_feedback {
            Action::ProvideFeedback { message, include_context, suppress_output } => {
                assert_eq!(message, "Test message");
                assert!(include_context);
                assert!(suppress_output);
            }
            _ => panic!("Expected ProvideFeedback action"),
        }

        let silent_approval = Action::allow_with_reason("Auto-approved")
            .with_suppress_output();
        
        match silent_approval {
            Action::Allow { reason, suppress_output } => {
                assert_eq!(reason, Some("Auto-approved".to_string()));
                assert!(suppress_output);
            }
            _ => panic!("Expected Allow action"),
        }

        let hard_command = Action::run_command(vec!["test".to_string()])
            .with_blocking_failure()
            .with_failure_feedback("Command failed")
            .with_suppress_output();
        
        match hard_command {
            Action::RunCommand { on_failure, on_failure_feedback, suppress_output, .. } => {
                assert_eq!(on_failure, OnFailureBehavior::Block);
                assert_eq!(on_failure_feedback, Some("Command failed".to_string()));
                assert!(suppress_output);
            }
            _ => panic!("Expected RunCommand action"),
        }

        let context_injection = Action::inject_context("Security reminder")
            .with_suppress_output();
        
        match context_injection {
            Action::InjectContext { context, use_stdout, suppress_output, .. } => {
                assert_eq!(context, Some("Security reminder".to_string()));
                assert!(use_stdout); // Default
                assert!(suppress_output);
            }
            _ => panic!("Expected InjectContext action"),
        }
    }

    #[test]
    fn test_action_serialization() {
        let action = Action::ProvideFeedback {
            message: "Use <Button> instead of <button>".to_string(),
            include_context: true,
            suppress_output: false,
        };

        let yaml = serde_yaml_ng::to_string(&action).unwrap();
        let deserialized: Action = serde_yaml_ng::from_str(&yaml).unwrap();

        match deserialized {
            Action::ProvideFeedback {
                message,
                include_context,
                ..
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
                spec: Box::new(CommandSpec::Array(Box::new(ArrayCommandSpec {
                    command: vec!["echo".to_string()],
                    args: Some(vec!["test".to_string()]),
                    working_dir: None,
                    env: None,
                    pipe: None,
                    redirect_stdout: None,
                    append_stdout: None,
                    redirect_stderr: None,
                    merge_stderr: None,
                    on_success: None,
                    on_failure: None,
                }))),
                expect_success: true,
            },
            then_action: Box::new(Action::BlockWithFeedback {
                feedback_message: "Blocked".to_string(),
                include_context: false,
                suppress_output: false,
            }),
            else_action: Some(Box::new(Action::Allow { reason: None, suppress_output: false })),
        };

        assert_eq!(conditional.action_type(), ActionType::Hard);
    }

    #[test]
    fn test_action_requirements() {
        let run_command = Action::RunCommand {
            spec: CommandSpec::Array(Box::new(ArrayCommandSpec {
                command: vec!["test".to_string()],
                args: None,
                working_dir: None,
                env: None,
                pipe: None,
                redirect_stdout: None,
                append_stdout: None,
                redirect_stderr: None,
                merge_stderr: None,
                on_success: None,
                on_failure: None,
            })),
            on_failure: OnFailureBehavior::Continue,
            on_failure_feedback: None,
            background: false,
            timeout_seconds: None,
            suppress_output: false,
        };
        assert!(run_command.requires_execution());
    }

    #[test]
    fn test_inject_context_action() {
        // Test creation and classification
        let inject_context = Action::InjectContext {
            context: Some("Remember to validate user input".to_string()),
            from_command: None,
            use_stdout: true,
            suppress_output: false,
        };

        assert!(inject_context.is_soft_action());
        assert!(!inject_context.is_hard_action());
        assert_eq!(inject_context.action_type(), ActionType::Soft);

        // Test serialization
        let yaml = serde_yaml_ng::to_string(&inject_context).unwrap();
        assert!(yaml.contains("inject_context"));
        assert!(yaml.contains("Remember to validate user input"));
        assert!(yaml.contains("use_stdout: true"));

        // Test deserialization
        let deserialized: Action = serde_yaml_ng::from_str(&yaml).unwrap();
        assert_eq!(inject_context, deserialized);

        // Test with use_stdout = false
        let inject_json = Action::InjectContext {
            context: Some("Use JSON method".to_string()),
            from_command: None,
            use_stdout: false,
            suppress_output: false,
        };

        let yaml2 = serde_yaml_ng::to_string(&inject_json).unwrap();
        assert!(yaml2.contains("use_stdout: false"));
    }

    #[test]
    fn test_inject_context_from_command() {
        // Test creation using builder
        let inject_from_cmd = Action::inject_context_from_command(
            CommandSpec::Array(Box::new(ArrayCommandSpec {
                command: vec!["./scripts/get-context.sh".to_string()],
                args: Some(vec!["{{prompt}}".to_string()]),
                working_dir: None,
                env: None,
                pipe: None,
                redirect_stdout: None,
                append_stdout: None,
                redirect_stderr: None,
                merge_stderr: None,
                on_success: None,
                on_failure: None,
            })),
            OnFailureBehavior::Continue,
        );

        assert!(inject_from_cmd.is_soft_action());
        assert_eq!(inject_from_cmd.action_type(), ActionType::Soft);

        // Test serialization
        let yaml = serde_yaml_ng::to_string(&inject_from_cmd).unwrap();
        assert!(yaml.contains("from_command"));
        assert!(yaml.contains("./scripts/get-context.sh"));
        assert!(yaml.contains("on_failure: continue"));

        // Test deserialization
        let deserialized: Action = serde_yaml_ng::from_str(&yaml).unwrap();
        assert_eq!(inject_from_cmd, deserialized);

        // Test that either context or from_command is required (not both)
        match inject_from_cmd {
            Action::InjectContext { context, from_command, .. } => {
                assert!(context.is_none());
                assert!(from_command.is_some());
            }
            _ => panic!("Expected InjectContext action"),
        }
    }

    #[test]
    fn test_ask_action_serialization() {
        let ask_action = Action::Ask {
            reason: "Please confirm this operation".to_string(),
            suppress_output: false,
        };

        // Test classification
        assert!(ask_action.is_hard_action());
        assert!(!ask_action.is_soft_action());
        assert_eq!(ask_action.action_type(), ActionType::Hard);

        // Test YAML serialization
        let yaml = serde_yaml_ng::to_string(&ask_action).unwrap();
        assert!(yaml.contains("ask"));
        assert!(yaml.contains("Please confirm this operation"));

        // Test deserialization
        let deserialized: Action = serde_yaml_ng::from_str(&yaml).unwrap();
        assert_eq!(ask_action, deserialized);

        // Verify the deserialized action maintains correct properties
        match deserialized {
            Action::Ask { reason, .. } => {
                assert_eq!(reason, "Please confirm this operation");
            }
            _ => panic!("Expected Ask action after deserialization"),
        }
    }
}
