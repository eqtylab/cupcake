use crate::engine::actions::ActionContext;
use crate::engine::conditions::EvaluationContext;
use crate::engine::events::{AgentEvent, claude_code::ClaudeCodeEvent};
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

    /// Build evaluation context from agent event
    pub fn build_evaluation_context(&self, agent_event: &AgentEvent) -> EvaluationContext {
        match agent_event {
            AgentEvent::ClaudeCode(claude_event) => self.build_claude_code_context(claude_event),
            // Future: Add other agent types here
        }
    }
    
    /// Build evaluation context from Claude Code event
    fn build_claude_code_context(&self, event: &ClaudeCodeEvent) -> EvaluationContext {
        let common = event.common();
        let current_dir = PathBuf::from(&common.cwd);
        
        // Start with base context
        let mut context = EvaluationContext {
            event_type: event.event_name().to_string(),
            session_id: common.session_id.clone(),
            current_dir,
            env_vars: std::env::vars().collect(),
            timestamp: Utc::now(),
            // All optional fields default to None
            tool_name: String::new(),
            tool_input: HashMap::new(),
            prompt: None,
            source: None,
            tool_response: None,
            stop_hook_active: None,
            trigger: None,
            custom_instructions: None,
        };
        
        // Fill in event-specific fields using clean pattern matching
        match event {
            ClaudeCodeEvent::PreToolUse(payload) => {
                context.tool_name = payload.tool_name.clone();
                context.tool_input = self.extract_tool_input(&payload.tool_input);
            }
            ClaudeCodeEvent::PostToolUse(payload) => {
                context.tool_name = payload.tool_name.clone();
                context.tool_input = self.extract_tool_input(&payload.tool_input);
                context.tool_response = Some(payload.tool_response.clone());
            }
            ClaudeCodeEvent::UserPromptSubmit(payload) => {
                context.prompt = Some(payload.prompt.clone());
            }
            ClaudeCodeEvent::SessionStart(payload) => {
                context.source = Some(match payload.source {
                    crate::engine::events::SessionSource::Startup => "startup".to_string(),
                    crate::engine::events::SessionSource::Resume => "resume".to_string(),
                    crate::engine::events::SessionSource::Clear => "clear".to_string(),
                });
            }
            ClaudeCodeEvent::Stop(payload) => {
                context.stop_hook_active = Some(payload.stop_hook_active);
            }
            ClaudeCodeEvent::SubagentStop(payload) => {
                context.stop_hook_active = Some(payload.stop_hook_active);
            }
            ClaudeCodeEvent::PreCompact(payload) => {
                context.trigger = Some(match payload.trigger {
                    crate::engine::events::CompactTrigger::Manual => "manual".to_string(),
                    crate::engine::events::CompactTrigger::Auto => "auto".to_string(),
                });
                context.custom_instructions = payload.custom_instructions.clone();
            }
            ClaudeCodeEvent::Notification { .. } => {
                // No special fields for notification
            }
        }
        
        context
    }

    /// Build action context from agent event
    pub fn build_action_context(&self, agent_event: &AgentEvent) -> ActionContext {
        // Single source of truth: derive from EvaluationContext
        let evaluation_context = self.build_evaluation_context(agent_event);
        ActionContext::from_evaluation_context(&evaluation_context)
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
    use crate::engine::events::{CommonEventData, HookEvent};

    #[test]
    fn test_context_builder_creation() {
        let _ = ExecutionContextBuilder::new();
    }

    #[test]
    fn test_evaluation_context_from_pre_tool_use() {
        let builder = ExecutionContextBuilder::new();
        let event = HookEvent::PreToolUse(crate::engine::events::claude_code::PreToolUsePayload {
            common: CommonEventData {
                session_id: "test-123".to_string(),
                transcript_path: "/tmp/transcript".to_string(),
                cwd: "/home/user".to_string(),
            },
            tool_name: "Bash".to_string(),
            tool_input: serde_json::json!({"command": "ls"}),
        });

        let agent_event = AgentEvent::ClaudeCode(event);
        let context = builder.build_evaluation_context(&agent_event);
        assert_eq!(context.event_type, "PreToolUse");
        assert_eq!(context.tool_name, "Bash");
        assert_eq!(context.session_id, "test-123");
        assert_eq!(context.tool_input["command"], serde_json::json!("ls"));
    }

    #[test]
    fn test_action_context_from_user_prompt() {
        let builder = ExecutionContextBuilder::new();
        let event = HookEvent::UserPromptSubmit(crate::engine::events::claude_code::UserPromptSubmitPayload {
            common: CommonEventData {
                session_id: "test-456".to_string(),
                transcript_path: "/tmp/transcript".to_string(),
                cwd: "/home/user".to_string(),
            },
            prompt: "Hello".to_string(),
        });

        let agent_event = AgentEvent::ClaudeCode(event);
        let context = builder.build_action_context(&agent_event);
        assert_eq!(context.session_id, "test-456");
        assert_eq!(context.tool_name, "");
        assert!(context.tool_input.is_empty());
    }

    #[test]
    fn test_evaluation_context_from_session_start() {
        let builder = ExecutionContextBuilder::new();
        let event = HookEvent::SessionStart(crate::engine::events::claude_code::SessionStartPayload {
            common: CommonEventData {
                session_id: "test-789".to_string(),
                transcript_path: "/tmp/transcript".to_string(),
                cwd: "/home/user/project".to_string(),
            },
            source: crate::engine::events::SessionSource::Startup,
        });

        let agent_event = AgentEvent::ClaudeCode(event);
        let context = builder.build_evaluation_context(&agent_event);
        assert_eq!(context.event_type, "SessionStart");
        assert_eq!(context.tool_name, "");
        assert_eq!(context.session_id, "test-789");
        assert!(context.tool_input.is_empty());
        assert_eq!(context.prompt, None);
    }

    #[test]
    fn test_action_context_from_session_start() {
        let builder = ExecutionContextBuilder::new();
        let event = HookEvent::SessionStart(crate::engine::events::claude_code::SessionStartPayload {
            common: CommonEventData {
                session_id: "test-789".to_string(),
                transcript_path: "/tmp/transcript".to_string(),
                cwd: "/home/user/project".to_string(),
            },
            source: crate::engine::events::SessionSource::Resume,
        });

        let agent_event = AgentEvent::ClaudeCode(event);
        let context = builder.build_action_context(&agent_event);
        assert_eq!(context.session_id, "test-789");
        assert_eq!(context.tool_name, "");
        assert!(context.tool_input.is_empty());
    }
}
