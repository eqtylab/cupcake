use super::types::PolicyFile;
use crate::{CupcakeError, Result};
use std::path::Path;

/// Configuration loader for policy files
pub struct PolicyLoader {
    /// Enable strict validation
    strict: bool,
    /// Cache loaded policies
    cache: std::collections::HashMap<String, (PolicyFile, std::time::SystemTime)>,
}

impl PolicyLoader {
    /// Create new policy loader
    pub fn new() -> Self {
        Self {
            strict: false,
            cache: std::collections::HashMap::new(),
        }
    }

    /// Enable strict validation mode
    pub fn with_strict_validation(mut self) -> Self {
        self.strict = true;
        self
    }

    /// Load policy file from path
    pub fn load_policy_file<P: AsRef<Path>>(&mut self, path: P) -> Result<PolicyFile> {
        let path = path.as_ref();
        let path_str = path.to_string_lossy().to_string();

        // Check cache first
        if let Some((cached_policy, cached_time)) = self.cache.get(&path_str) {
            if let Ok(metadata) = std::fs::metadata(path) {
                if let Ok(modified) = metadata.modified() {
                    if modified <= *cached_time {
                        return Ok(cached_policy.clone());
                    }
                }
            }
        }

        // Load from file
        let contents = std::fs::read_to_string(path).map_err(|e| {
            CupcakeError::Config(format!("Failed to read policy file {}: {}", path_str, e))
        })?;

        let policy: PolicyFile = toml::from_str(&contents).map_err(|e| {
            CupcakeError::Config(format!("Failed to parse policy file {}: {}", path_str, e))
        })?;

        // Validate policy
        self.validate_policy(&policy)?;

        // Cache the result
        if let Ok(metadata) = std::fs::metadata(path) {
            if let Ok(modified) = metadata.modified() {
                self.cache.insert(path_str, (policy.clone(), modified));
            }
        }

        Ok(policy)
    }

    /// Load policies from multiple files (project + user)
    pub fn load_policy_hierarchy<P: AsRef<Path>>(
        &mut self,
        project_root: P,
    ) -> Result<Vec<PolicyFile>> {
        let project_root = project_root.as_ref();
        let mut policies = Vec::new();

        // Load project policies first (higher priority)
        let project_policy_path = project_root.join("cupcake.toml");
        if project_policy_path.exists() {
            match self.load_policy_file(project_policy_path) {
                Ok(policy) => policies.push(policy),
                Err(e) => {
                    eprintln!("Warning: Failed to load project policy file: {}", e);
                    // Don't fail entirely, just continue without project policies
                }
            }
        }

        // Load user policies second (lower priority)
        // Claude Code uses ~/.claude/ directory structure
        if let Some(home_dir) = directories::BaseDirs::new() {
            let user_policy_path = home_dir.home_dir().join(".claude").join("cupcake.toml");
            if user_policy_path.exists() {
                match self.load_policy_file(user_policy_path) {
                    Ok(policy) => policies.push(policy),
                    Err(e) => {
                        eprintln!("Warning: Failed to load user policy file: {}", e);
                        // Don't fail entirely, just continue without user policies
                    }
                }
            }
        }

        Ok(policies)
    }

    /// Load policy files from specific paths (for runtime use)
    pub fn load_policies_from_paths<P: AsRef<Path>>(
        &mut self,
        paths: &[P],
    ) -> Result<Vec<PolicyFile>> {
        let mut policies = Vec::new();

        for path in paths {
            let path = path.as_ref();
            if path.exists() {
                match self.load_policy_file(path) {
                    Ok(policy) => policies.push(policy),
                    Err(e) => {
                        eprintln!(
                            "Warning: Failed to load policy file {}: {}",
                            path.display(),
                            e
                        );
                        // Continue loading other files
                    }
                }
            }
        }

        Ok(policies)
    }

    /// Validate policy file structure and content
    fn validate_policy(&self, policy: &PolicyFile) -> Result<()> {
        // Check schema version
        if policy.schema_version != "1.0" {
            return Err(CupcakeError::Config(format!(
                "Unsupported schema version: {}. Expected: 1.0",
                policy.schema_version
            )));
        }

        // Validate each policy
        for (index, policy_def) in policy.policies.iter().enumerate() {
            self.validate_policy_definition(policy_def, index)?;
        }

        Ok(())
    }

    /// Validate individual policy definition
    fn validate_policy_definition(
        &self,
        policy: &super::types::Policy,
        index: usize,
    ) -> Result<()> {
        // Check policy name
        if policy.name.is_empty() {
            return Err(CupcakeError::Config(format!(
                "Policy at index {} has empty name",
                index
            )));
        }

        // Check conditions
        if policy.conditions.is_empty() {
            return Err(CupcakeError::Config(format!(
                "Policy '{}' has no conditions",
                policy.name
            )));
        }

        // Validate conditions
        for condition in &policy.conditions {
            self.validate_condition(condition, &policy.name)?;
        }

        // Validate action
        self.validate_action(&policy.action, &policy.name)?;

        // Check matcher for tool events
        match policy.hook_event {
            super::types::HookEventType::PreToolUse | super::types::HookEventType::PostToolUse => {
                if policy.matcher.is_none() {
                    return Err(CupcakeError::Config(format!(
                        "Policy '{}' for tool events must have a matcher",
                        policy.name
                    )));
                }

                // Validate matcher regex
                if let Some(ref matcher) = policy.matcher {
                    regex::Regex::new(matcher).map_err(|e| {
                        CupcakeError::Config(format!(
                            "Policy '{}' has invalid matcher regex: {}",
                            policy.name, e
                        ))
                    })?;
                }
            }
            super::types::HookEventType::PreCompact => {
                if let Some(ref matcher) = policy.matcher {
                    if matcher != "manual" && matcher != "auto" {
                        return Err(CupcakeError::Config(format!(
                            "Policy '{}' for PreCompact must have matcher 'manual' or 'auto'",
                            policy.name
                        )));
                    }
                }
            }
            _ => {
                // Other events don't use matchers
            }
        }

        Ok(())
    }

    /// Validate condition
    #[allow(clippy::only_used_in_recursion)] // False positive - self is needed for method recursion
    fn validate_condition(
        &self,
        condition: &super::conditions::Condition,
        policy_name: &str,
    ) -> Result<()> {
        match condition {
            super::conditions::Condition::Pattern { regex, .. } => {
                // Validate regex pattern
                regex::Regex::new(regex).map_err(|e| {
                    CupcakeError::Config(format!(
                        "Policy '{}' has invalid regex '{}': {}",
                        policy_name, regex, e
                    ))
                })?;
            }
            super::conditions::Condition::Match { field, .. } => {
                // Validate field name (basic check for non-empty)
                if field.is_empty() {
                    return Err(CupcakeError::Config(format!(
                        "Policy '{}' has empty field name in Match condition",
                        policy_name
                    )));
                }
            }
            super::conditions::Condition::Check { command, .. } => {
                // Validate command is not empty
                if command.trim().is_empty() {
                    return Err(CupcakeError::Config(format!(
                        "Policy '{}' has empty command in Check condition",
                        policy_name
                    )));
                }
            }
            super::conditions::Condition::Not { condition } => {
                self.validate_condition(condition, policy_name)?;
            }
            super::conditions::Condition::And { conditions }
            | super::conditions::Condition::Or { conditions } => {
                for cond in conditions {
                    self.validate_condition(cond, policy_name)?;
                }
            }
        }

        Ok(())
    }

    /// Validate action
    fn validate_action(&self, action: &super::actions::Action, policy_name: &str) -> Result<()> {
        match action {
            super::actions::Action::ProvideFeedback { message, .. }
            | super::actions::Action::BlockWithFeedback {
                feedback_message: message,
                ..
            } => {
                if message.is_empty() {
                    return Err(CupcakeError::Config(format!(
                        "Policy '{}' has empty feedback message",
                        policy_name
                    )));
                }
            }
            super::actions::Action::RunCommand { command, .. } => {
                if command.is_empty() {
                    return Err(CupcakeError::Config(format!(
                        "Policy '{}' has empty command",
                        policy_name
                    )));
                }
            }
            super::actions::Action::UpdateState { event, key, .. } => {
                if event.is_none() && key.is_none() {
                    return Err(CupcakeError::Config(format!(
                        "Policy '{}' UpdateState action must have either event or key",
                        policy_name
                    )));
                }
            }
            super::actions::Action::Conditional {
                if_condition,
                then_action,
                else_action,
                ..
            } => {
                self.validate_condition(if_condition, policy_name)?;
                self.validate_action(then_action, policy_name)?;
                if let Some(else_act) = else_action {
                    self.validate_action(else_act, policy_name)?;
                }
            }
            _ => {
                // Other actions don't need special validation
            }
        }

        Ok(())
    }

    /// Validate time format (HH:MM)
    #[cfg(test)]
    fn validate_time_format(&self, time: &str, policy_name: &str) -> Result<()> {
        let parts: Vec<&str> = time.split(':').collect();
        if parts.len() != 2 {
            return Err(CupcakeError::Config(format!(
                "Policy '{}' has invalid time format '{}'. Expected HH:MM",
                policy_name, time
            )));
        }

        let hours: u32 = parts[0].parse().map_err(|_| {
            CupcakeError::Config(format!(
                "Policy '{}' has invalid hours in time '{}'",
                policy_name, time
            ))
        })?;

        let minutes: u32 = parts[1].parse().map_err(|_| {
            CupcakeError::Config(format!(
                "Policy '{}' has invalid minutes in time '{}'",
                policy_name, time
            ))
        })?;

        if hours > 23 || minutes > 59 {
            return Err(CupcakeError::Config(format!(
                "Policy '{}' has invalid time '{}'. Hours must be 0-23, minutes 0-59",
                policy_name, time
            )));
        }

        Ok(())
    }
}

impl Default for PolicyLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_load_valid_policy() {
        let dir = tempdir().unwrap();
        let policy_path = dir.path().join("cupcake.toml");

        let policy_content = r#"schema_version = "1.0"

[settings]
audit_logging = true

[[policies]]
name = "Test Policy"
hook_event = "PreToolUse"
matcher = "Bash"
conditions = [
  { type = "pattern", field = "tool_input.command", regex = "echo.*" }
]
action = { type = "provide_feedback", message = "Test message", include_context = false }
"#;

        let mut file = File::create(&policy_path).unwrap();
        file.write_all(policy_content.as_bytes()).unwrap();

        let mut loader = PolicyLoader::new();
        let policy = loader.load_policy_file(&policy_path).unwrap();

        assert_eq!(policy.schema_version, "1.0");
        assert!(policy.settings.audit_logging);
        assert_eq!(policy.policies.len(), 1);
        assert_eq!(policy.policies[0].name, "Test Policy");
    }

    #[test]
    fn test_validate_invalid_regex() {
        let dir = tempdir().unwrap();
        let policy_path = dir.path().join("cupcake.toml");

        let policy_content = r#"schema_version = "1.0"

[[policies]]
name = "Invalid Regex"
hook_event = "PreToolUse"
matcher = "Bash"
conditions = [
  { type = "pattern", field = "tool_input.command", regex = "[invalid-regex" }
]
action = { type = "provide_feedback", message = "Test", include_context = false }
"#;

        let mut file = File::create(&policy_path).unwrap();
        file.write_all(policy_content.as_bytes()).unwrap();

        let mut loader = PolicyLoader::new();
        let result = loader.load_policy_file(&policy_path);

        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("regex") || error_msg.contains("invalid"));
    }

    #[test]
    fn test_validate_time_format() {
        let loader = PolicyLoader::new();

        // Valid times
        assert!(loader.validate_time_format("09:00", "test").is_ok());
        assert!(loader.validate_time_format("23:59", "test").is_ok());
        assert!(loader.validate_time_format("00:00", "test").is_ok());

        // Invalid times
        assert!(loader.validate_time_format("25:00", "test").is_err());
        assert!(loader.validate_time_format("12:60", "test").is_err());
        assert!(loader.validate_time_format("invalid", "test").is_err());
        assert!(loader.validate_time_format("12", "test").is_err());
    }
}
