use crate::engine::actions::ActionContext;
use crate::engine::conditions::EvaluationContext;
use crate::engine::events::HookEvent;
use chrono::Utc;
use std::collections::HashMap;
use std::path::PathBuf;

/// Builds execution contexts from hook events
pub struct ExecutionContextBuilder;

impl Default for ExecutionContextBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ExecutionContextBuilder {
    pub fn new() -> Self {
        Self
    }

    /// Build evaluation context from hook event
    pub fn build_evaluation_context(&self, hook_event: &HookEvent) -> EvaluationContext {
        let (session_id, tool_name, tool_input, prompt, source) =
            self.extract_event_data(hook_event);
        let current_dir = PathBuf::from(&hook_event.common().cwd);

        EvaluationContext {
            event_type: hook_event.event_name().to_string(),
            tool_name,
            tool_input,
            session_id,
            current_dir,
            env_vars: std::env::vars().collect(),
            timestamp: Utc::now(),
            prompt,
            source,
        }
    }

    /// Build action context from hook event
    pub fn build_action_context(&self, hook_event: &HookEvent) -> ActionContext {
        let (session_id, tool_name, tool_input, prompt, source) =
            self.extract_event_data(hook_event);
        let current_dir = PathBuf::from(&hook_event.common().cwd);

        let mut context = ActionContext::new(
            tool_name,
            tool_input,
            current_dir,
            std::env::vars().collect(),
            session_id,
        );

        // Add prompt to template variables if present
        if let Some(prompt_text) = prompt {
            context
                .template_vars
                .insert("prompt".to_string(), prompt_text);
        }

        // Add source to template variables if present
        if let Some(source_text) = source {
            context
                .template_vars
                .insert("source".to_string(), source_text);
        }

        context
    }

    fn extract_event_data(
        &self,
        hook_event: &HookEvent,
    ) -> (
        String,
        String,
        HashMap<String, serde_json::Value>,
        Option<String>,
        Option<String>,
    ) {
        match hook_event {
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
                None,
            ),
            HookEvent::UserPromptSubmit { common, prompt } => (
                common.session_id.clone(),
                String::new(),
                HashMap::new(),
                Some(prompt.clone()),
                None,
            ),
            HookEvent::SessionStart { common, source } => (
                common.session_id.clone(),
                String::new(),
                HashMap::new(),
                None,
                Some(match source {
                    crate::engine::events::SessionSource::Startup => "startup".to_string(),
                    crate::engine::events::SessionSource::Resume => "resume".to_string(),
                    crate::engine::events::SessionSource::Clear => "clear".to_string(),
                }),
            ),
            HookEvent::Notification { common, .. }
            | HookEvent::Stop { common, .. }
            | HookEvent::SubagentStop { common, .. }
            | HookEvent::PreCompact { common, .. } => (
                common.session_id.clone(),
                String::new(),
                HashMap::new(),
                None,
                None,
            ),
        }
    }

    fn extract_tool_input(
        &self,
        tool_input: &serde_json::Value,
    ) -> HashMap<String, serde_json::Value> {
        match tool_input {
            serde_json::Value::Object(map) => map.clone().into_iter().collect(),
            _ => HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::events::CommonEventData;

    #[test]
    fn test_context_builder_creation() {
        let _ = ExecutionContextBuilder::new();
    }

    #[test]
    fn test_evaluation_context_from_pre_tool_use() {
        let builder = ExecutionContextBuilder::new();
        let event = HookEvent::PreToolUse {
            common: CommonEventData {
                session_id: "test-123".to_string(),
                transcript_path: "/tmp/transcript".to_string(),
                cwd: "/home/user".to_string(),
            },
            tool_name: "Bash".to_string(),
            tool_input: serde_json::json!({"command": "ls"}),
        };

        let context = builder.build_evaluation_context(&event);
        assert_eq!(context.event_type, "PreToolUse");
        assert_eq!(context.tool_name, "Bash");
        assert_eq!(context.session_id, "test-123");
        assert_eq!(context.tool_input["command"], serde_json::json!("ls"));
    }

    #[test]
    fn test_action_context_from_user_prompt() {
        let builder = ExecutionContextBuilder::new();
        let event = HookEvent::UserPromptSubmit {
            common: CommonEventData {
                session_id: "test-456".to_string(),
                transcript_path: "/tmp/transcript".to_string(),
                cwd: "/home/user".to_string(),
            },
            prompt: "Hello".to_string(),
        };

        let context = builder.build_action_context(&event);
        assert_eq!(context.session_id, "test-456");
        assert_eq!(context.tool_name, "");
        assert!(context.tool_input.is_empty());
    }

    #[test]
    fn test_evaluation_context_from_session_start() {
        let builder = ExecutionContextBuilder::new();
        let event = HookEvent::SessionStart {
            common: CommonEventData {
                session_id: "test-789".to_string(),
                transcript_path: "/tmp/transcript".to_string(),
                cwd: "/home/user/project".to_string(),
            },
            source: crate::engine::events::SessionSource::Startup,
        };

        let context = builder.build_evaluation_context(&event);
        assert_eq!(context.event_type, "SessionStart");
        assert_eq!(context.tool_name, "");
        assert_eq!(context.session_id, "test-789");
        assert!(context.tool_input.is_empty());
        assert_eq!(context.prompt, None);
    }

    #[test]
    fn test_action_context_from_session_start() {
        let builder = ExecutionContextBuilder::new();
        let event = HookEvent::SessionStart {
            common: CommonEventData {
                session_id: "test-789".to_string(),
                transcript_path: "/tmp/transcript".to_string(),
                cwd: "/home/user/project".to_string(),
            },
            source: crate::engine::events::SessionSource::Resume,
        };

        let context = builder.build_action_context(&event);
        assert_eq!(context.session_id, "test-789");
        assert_eq!(context.tool_name, "");
        assert!(context.tool_input.is_empty());
    }
}
