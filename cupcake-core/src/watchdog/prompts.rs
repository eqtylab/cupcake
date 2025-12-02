//! Prompt loading for Watchdog
//!
//! Loads system and user prompts from `.cupcake/watchdog/` directory.
//! Falls back to platform-specific global config directory:
//! - Linux: `~/.config/cupcake/watchdog/`
//! - macOS: `~/Library/Application Support/cupcake/watchdog/`
//! - Windows: `%APPDATA%\cupcake\watchdog\`

use std::path::Path;
use tracing::debug;

/// Default system prompt for security evaluation
pub const DEFAULT_SYSTEM_PROMPT: &str = r#"You are a security reviewer for an AI coding agent. Your job is to evaluate tool calls before they execute and determine if they should be allowed.

Analyze the event and respond with a JSON object containing:
- "allow": boolean - whether the action should proceed
- "confidence": number 0-1 - how confident you are in this assessment
- "reasoning": string - brief explanation of your decision
- "concerns": array of strings - specific security concerns identified (empty if none)
- "suggestions": array of strings - alternative approaches if blocking (empty if allowing)

Focus on:
- Data exfiltration risks (reading secrets, credentials, private keys)
- Destructive operations (rm -rf, DROP TABLE, etc.)
- Privilege escalation attempts
- Actions that seem misaligned with typical development workflows

Be pragmatic. Most legitimate development commands should be allowed. Only flag genuinely concerning patterns.

Respond with ONLY the JSON object, no markdown formatting."#;

/// Default user template - just the raw event
pub const DEFAULT_USER_TEMPLATE: &str = "{{event}}";

/// Loaded prompts for Watchdog
#[derive(Debug, Clone)]
pub struct WatchdogPrompts {
    /// System prompt for the LLM
    pub system_prompt: String,
    /// User message template with {{event}} placeholder
    pub user_template: String,
}

impl Default for WatchdogPrompts {
    fn default() -> Self {
        Self {
            system_prompt: DEFAULT_SYSTEM_PROMPT.to_string(),
            user_template: DEFAULT_USER_TEMPLATE.to_string(),
        }
    }
}

impl WatchdogPrompts {
    /// Load prompts from watchdog directories
    ///
    /// Lookup order for each file:
    /// 1. Project: `.cupcake/watchdog/{system,user}.txt`
    /// 2. Global: `~/.config/cupcake/watchdog/{system,user}.txt`
    /// 3. Built-in default
    pub fn load(
        project_watchdog_dir: Option<&Path>,
        global_watchdog_dir: Option<&Path>,
    ) -> Self {
        let system_prompt = Self::load_file("system.txt", project_watchdog_dir, global_watchdog_dir)
            .unwrap_or_else(|| DEFAULT_SYSTEM_PROMPT.to_string());

        let user_template = Self::load_file("user.txt", project_watchdog_dir, global_watchdog_dir)
            .unwrap_or_else(|| DEFAULT_USER_TEMPLATE.to_string());

        Self {
            system_prompt,
            user_template,
        }
    }

    /// Load a specific file from watchdog directories
    fn load_file(
        filename: &str,
        project_dir: Option<&Path>,
        global_dir: Option<&Path>,
    ) -> Option<String> {
        // Try project first
        if let Some(dir) = project_dir {
            let path = dir.join(filename);
            if path.exists() {
                match std::fs::read_to_string(&path) {
                    Ok(content) => {
                        debug!("Loaded {} from {}", filename, path.display());
                        return Some(content);
                    }
                    Err(e) => {
                        tracing::warn!("Failed to read {}: {}", path.display(), e);
                    }
                }
            }
        }

        // Try global
        if let Some(dir) = global_dir {
            let path = dir.join(filename);
            if path.exists() {
                match std::fs::read_to_string(&path) {
                    Ok(content) => {
                        debug!("Loaded {} from {}", filename, path.display());
                        return Some(content);
                    }
                    Err(e) => {
                        tracing::warn!("Failed to read {}: {}", path.display(), e);
                    }
                }
            }
        }

        None
    }

    /// Render the user message by replacing {{event}} with the event JSON
    pub fn render_user_message(&self, event: &serde_json::Value) -> String {
        let event_json = serde_json::to_string_pretty(event).unwrap_or_else(|_| "{}".to_string());
        self.user_template.replace("{{event}}", &event_json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_default_prompts() {
        let prompts = WatchdogPrompts::default();
        assert!(prompts.system_prompt.contains("security reviewer"));
        assert_eq!(prompts.user_template, "{{event}}");
    }

    #[test]
    fn test_load_from_project_dir() {
        let temp = TempDir::new().unwrap();
        let watchdog_dir = temp.path().join("watchdog");
        fs::create_dir_all(&watchdog_dir).unwrap();

        fs::write(watchdog_dir.join("system.txt"), "Custom system prompt").unwrap();
        fs::write(watchdog_dir.join("user.txt"), "Event: {{event}}").unwrap();

        let prompts = WatchdogPrompts::load(Some(&watchdog_dir), None);
        assert_eq!(prompts.system_prompt, "Custom system prompt");
        assert_eq!(prompts.user_template, "Event: {{event}}");
    }

    #[test]
    fn test_load_fallback_to_global() {
        let project_temp = TempDir::new().unwrap();
        let global_temp = TempDir::new().unwrap();

        let project_dir = project_temp.path().join("watchdog");
        let global_dir = global_temp.path().join("watchdog");

        fs::create_dir_all(&project_dir).unwrap();
        fs::create_dir_all(&global_dir).unwrap();

        // Only system.txt in project
        fs::write(project_dir.join("system.txt"), "Project system").unwrap();
        // Only user.txt in global
        fs::write(global_dir.join("user.txt"), "Global user: {{event}}").unwrap();

        let prompts = WatchdogPrompts::load(Some(&project_dir), Some(&global_dir));
        assert_eq!(prompts.system_prompt, "Project system");
        assert_eq!(prompts.user_template, "Global user: {{event}}");
    }

    #[test]
    fn test_load_fallback_to_defaults() {
        let prompts = WatchdogPrompts::load(None, None);
        assert!(prompts.system_prompt.contains("security reviewer"));
        assert_eq!(prompts.user_template, "{{event}}");
    }

    #[test]
    fn test_render_user_message() {
        let prompts = WatchdogPrompts {
            system_prompt: "".to_string(),
            user_template: "Evaluate this:\n{{event}}\n\nBe careful!".to_string(),
        };

        let event = serde_json::json!({
            "tool_name": "Bash",
            "command": "ls"
        });

        let rendered = prompts.render_user_message(&event);
        assert!(rendered.contains("Evaluate this:"));
        assert!(rendered.contains("\"tool_name\": \"Bash\""));
        assert!(rendered.contains("Be careful!"));
    }

    #[test]
    fn test_render_default_template() {
        let prompts = WatchdogPrompts::default();
        let event = serde_json::json!({"test": "value"});

        let rendered = prompts.render_user_message(&event);
        assert!(rendered.contains("\"test\": \"value\""));
        // Should be just the JSON, no extra text
        assert!(!rendered.contains("{{event}}"));
    }
}
