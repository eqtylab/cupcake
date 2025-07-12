use super::types::{PolicyFile, RootConfig, PolicyFragment, ComposedPolicy, HookEventType};
use crate::{CupcakeError, Result};
use std::path::{Path, PathBuf};
use std::collections::{HashMap, HashSet};

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

    /// Load and compose policies from YAML guardrails directory
    /// This is the main entry point for the new YAML-based policy loading
    pub fn load_and_compose_policies(&mut self, start_dir: &Path) -> Result<Vec<ComposedPolicy>> {
        // Step 1: Discover - find guardrails/cupcake.yaml
        let root_config_path = self.discover_root_config(start_dir)?;
        let root_config = self.load_root_config(&root_config_path)?;
        
        // Step 2: Resolve imports using glob patterns
        let policy_fragment_paths = self.resolve_imports(&root_config, &root_config_path)?;
        
        // Step 3: Compose - deep merge all policy fragments
        let composed_fragment = self.compose_policy_fragments(&policy_fragment_paths)?;
        
        // Step 4: Validate and flatten to Vec<ComposedPolicy>
        let composed_policies = self.validate_and_flatten(composed_fragment)?;
        
        Ok(composed_policies)
    }

    /// Step 1: Search upward from start_dir for guardrails/cupcake.yaml
    fn discover_root_config(&self, start_dir: &Path) -> Result<PathBuf> {
        let mut current_dir = start_dir;
        
        loop {
            let candidate = current_dir.join("guardrails").join("cupcake.yaml");
            if candidate.exists() {
                return Ok(candidate);
            }
            
            // Move up one directory
            match current_dir.parent() {
                Some(parent) => current_dir = parent,
                None => break,
            }
        }
        
        Err(CupcakeError::Config(
            "No guardrails/cupcake.yaml found in current directory or any parent directory".to_string()
        ))
    }

    /// Load and parse the root cupcake.yaml file
    fn load_root_config(&self, config_path: &Path) -> Result<RootConfig> {
        let contents = std::fs::read_to_string(config_path).map_err(|e| {
            CupcakeError::Config(format!(
                "Failed to read root config file {}: {}", 
                config_path.display(), 
                e
            ))
        })?;

        let root_config: RootConfig = serde_yaml_ng::from_str(&contents).map_err(|e| {
            CupcakeError::Config(format!(
                "Failed to parse root config file {}: {}", 
                config_path.display(), 
                e
            ))
        })?;

        Ok(root_config)
    }

    /// Step 2: Resolve glob patterns in imports to actual file paths
    fn resolve_imports(&self, root_config: &RootConfig, root_config_path: &Path) -> Result<Vec<PathBuf>> {
        let root_dir = root_config_path.parent().ok_or_else(|| {
            CupcakeError::Path("Root config path has no parent directory".to_string())
        })?;

        let mut policy_files = Vec::new();

        for import_pattern in &root_config.imports {
            // Resolve pattern relative to the guardrails/ directory
            let pattern_path = root_dir.join(import_pattern);
            let pattern_str = pattern_path.to_string_lossy();

            // Use glob to expand the pattern
            let paths = glob::glob(&pattern_str).map_err(|e| {
                CupcakeError::Config(format!(
                    "Invalid glob pattern '{}': {}", 
                    import_pattern, 
                    e
                ))
            })?;

            for path_result in paths {
                let path = path_result.map_err(|e| {
                    CupcakeError::Config(format!(
                        "Failed to resolve glob path in pattern '{}': {}", 
                        import_pattern, 
                        e
                    ))
                })?;

                if path.is_file() {
                    policy_files.push(path);
                }
            }
        }

        // Sort alphabetically for deterministic loading order
        policy_files.sort();
        
        Ok(policy_files)
    }

    /// Step 3: Load and compose all policy fragments via deep merge
    fn compose_policy_fragments(&self, fragment_paths: &[PathBuf]) -> Result<PolicyFragment> {
        let mut composed: PolicyFragment = HashMap::new();

        for fragment_path in fragment_paths {
            let fragment = self.load_policy_fragment(fragment_path)?;
            self.deep_merge_fragment(&mut composed, fragment);
        }

        Ok(composed)
    }

    /// Load a single policy fragment file
    fn load_policy_fragment(&self, fragment_path: &Path) -> Result<PolicyFragment> {
        let contents = std::fs::read_to_string(fragment_path).map_err(|e| {
            CupcakeError::Config(format!(
                "Failed to read policy fragment {}: {}", 
                fragment_path.display(), 
                e
            ))
        })?;

        let fragment: PolicyFragment = serde_yaml_ng::from_str(&contents).map_err(|e| {
            CupcakeError::Config(format!(
                "Failed to parse policy fragment {}: {}", 
                fragment_path.display(), 
                e
            ))
        })?;

        Ok(fragment)
    }

    /// Perform deep merge: concatenate policy lists under same hook_event/matcher
    fn deep_merge_fragment(&self, target: &mut PolicyFragment, source: PolicyFragment) {
        for (hook_event, matchers) in source {
            let target_matchers = target.entry(hook_event).or_insert_with(HashMap::new);
            
            for (matcher, policies) in matchers {
                let target_policies = target_matchers.entry(matcher).or_insert_with(Vec::new);
                target_policies.extend(policies);
            }
        }
    }

    /// Step 4: Validate unique names and flatten to Vec<ComposedPolicy>
    fn validate_and_flatten(&self, composed: PolicyFragment) -> Result<Vec<ComposedPolicy>> {
        let mut policy_names = HashSet::new();
        let mut composed_policies = Vec::new();

        for (hook_event_str, matchers) in composed {
            // Parse hook event from string
            let hook_event = self.parse_hook_event(&hook_event_str)?;
            
            for (matcher, policies) in matchers {
                for policy in policies {
                    // Check for duplicate names
                    if !policy_names.insert(policy.name.clone()) {
                        return Err(CupcakeError::Config(format!(
                            "Duplicate policy name '{}' found. Policy names must be unique across all imported files.",
                            policy.name
                        )));
                    }

                    // Create ComposedPolicy
                    let composed_policy = ComposedPolicy {
                        name: policy.name,
                        description: policy.description,
                        hook_event: hook_event.clone(),
                        matcher: matcher.clone(),
                        conditions: policy.conditions,
                        action: policy.action,
                    };

                    composed_policies.push(composed_policy);
                }
            }
        }

        Ok(composed_policies)
    }

    /// Parse hook event string to HookEventType
    fn parse_hook_event(&self, hook_event_str: &str) -> Result<HookEventType> {
        match hook_event_str {
            "PreToolUse" => Ok(HookEventType::PreToolUse),
            "PostToolUse" => Ok(HookEventType::PostToolUse),
            "Notification" => Ok(HookEventType::Notification),
            "Stop" => Ok(HookEventType::Stop),
            "SubagentStop" => Ok(HookEventType::SubagentStop),
            "PreCompact" => Ok(HookEventType::PreCompact),
            _ => Err(CupcakeError::Config(format!(
                "Unknown hook event type: {}. Valid types are: PreToolUse, PostToolUse, Notification, Stop, SubagentStop, PreCompact",
                hook_event_str
            )))
        }
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

        // TODO: Replace with YAML parsing in Phase 3
        let policy: PolicyFile = serde_json::from_str("{}").map_err(|e| {
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
    use std::fs::{File, create_dir_all};
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

    // =============================================================================
    // YAML Loader Tests (Plan 005)
    // =============================================================================

    #[test]
    fn test_discover_root_config_in_current_dir() {
        let dir = tempdir().unwrap();
        let guardrails_dir = dir.path().join("guardrails");
        create_dir_all(&guardrails_dir).unwrap();
        
        let cupcake_yaml = guardrails_dir.join("cupcake.yaml");
        let mut file = File::create(&cupcake_yaml).unwrap();
        file.write_all(b"settings:\n  audit_logging: true\nimports:\n  - \"policies/*.yaml\"").unwrap();
        
        let loader = PolicyLoader::new();
        let result = loader.discover_root_config(dir.path()).unwrap();
        assert_eq!(result, cupcake_yaml);
    }

    #[test]
    fn test_discover_root_config_upward_search() {
        let dir = tempdir().unwrap();
        let subdir = dir.path().join("src").join("components");
        create_dir_all(&subdir).unwrap();
        
        let guardrails_dir = dir.path().join("guardrails");
        create_dir_all(&guardrails_dir).unwrap();
        
        let cupcake_yaml = guardrails_dir.join("cupcake.yaml");
        let mut file = File::create(&cupcake_yaml).unwrap();
        file.write_all(b"settings:\n  audit_logging: true\nimports:\n  - \"policies/*.yaml\"").unwrap();
        
        let loader = PolicyLoader::new();
        let result = loader.discover_root_config(&subdir).unwrap();
        assert_eq!(result, cupcake_yaml);
    }

    #[test]
    fn test_discover_root_config_not_found() {
        let dir = tempdir().unwrap();
        
        let loader = PolicyLoader::new();
        let result = loader.discover_root_config(dir.path());
        
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No guardrails/cupcake.yaml found"));
    }

    #[test]
    fn test_load_root_config() {
        let dir = tempdir().unwrap();
        let guardrails_dir = dir.path().join("guardrails");
        create_dir_all(&guardrails_dir).unwrap();
        
        let cupcake_yaml = guardrails_dir.join("cupcake.yaml");
        let mut file = File::create(&cupcake_yaml).unwrap();
        file.write_all(br#"
settings:
  audit_logging: true
  debug_mode: false
imports:
  - "policies/*.yaml"
  - "policies/security/*.yaml"
"#).unwrap();
        
        let loader = PolicyLoader::new();
        let root_config = loader.load_root_config(&cupcake_yaml).unwrap();
        
        assert!(root_config.settings.audit_logging);
        assert!(!root_config.settings.debug_mode);
        assert_eq!(root_config.imports.len(), 2);
        assert_eq!(root_config.imports[0], "policies/*.yaml");
        assert_eq!(root_config.imports[1], "policies/security/*.yaml");
    }

    #[test]
    fn test_resolve_imports_with_glob() {
        let dir = tempdir().unwrap();
        let guardrails_dir = dir.path().join("guardrails");
        let policies_dir = guardrails_dir.join("policies");
        create_dir_all(&policies_dir).unwrap();
        
        // Create test policy files
        File::create(policies_dir.join("01-base.yaml")).unwrap();
        File::create(policies_dir.join("02-security.yaml")).unwrap();
        File::create(policies_dir.join("03-frontend.yaml")).unwrap();
        File::create(policies_dir.join("README.md")).unwrap(); // Should be ignored
        
        let cupcake_yaml = guardrails_dir.join("cupcake.yaml");
        
        let root_config = RootConfig {
            settings: super::super::types::Settings::default(),
            imports: vec!["policies/*.yaml".to_string()],
        };
        
        let loader = PolicyLoader::new();
        let resolved = loader.resolve_imports(&root_config, &cupcake_yaml).unwrap();
        
        assert_eq!(resolved.len(), 3);
        // Files should be sorted alphabetically
        assert!(resolved[0].ends_with("01-base.yaml"));
        assert!(resolved[1].ends_with("02-security.yaml"));
        assert!(resolved[2].ends_with("03-frontend.yaml"));
    }

    #[test]
    fn test_load_policy_fragment() {
        let dir = tempdir().unwrap();
        let fragment_path = dir.path().join("test-fragment.yaml");
        
        let mut file = File::create(&fragment_path).unwrap();
        file.write_all(br#"
PreToolUse:
  "Bash":
    - name: "Block rm commands"
      conditions:
        - type: "pattern"
          field: "tool_input.command"
          regex: "^rm\\s"
      action:
        type: "block_with_feedback"
        feedback_message: "Dangerous rm command blocked!"
        include_context: false

PostToolUse:
  "Write":
    - name: "File created notification"
      conditions:
        - type: "match"
          field: "tool_name"
          value: "Write"
      action:
        type: "provide_feedback"
        message: "File created successfully"
        include_context: false
"#).unwrap();
        
        let loader = PolicyLoader::new();
        let fragment = loader.load_policy_fragment(&fragment_path).unwrap();
        
        assert!(fragment.contains_key("PreToolUse"));
        assert!(fragment.contains_key("PostToolUse"));
        
        let pre_tool_use = fragment.get("PreToolUse").unwrap();
        assert!(pre_tool_use.contains_key("Bash"));
        
        let bash_policies = pre_tool_use.get("Bash").unwrap();
        assert_eq!(bash_policies.len(), 1);
        assert_eq!(bash_policies[0].name, "Block rm commands");
        
        let post_tool_use = fragment.get("PostToolUse").unwrap();
        assert!(post_tool_use.contains_key("Write"));
        
        let write_policies = post_tool_use.get("Write").unwrap();
        assert_eq!(write_policies.len(), 1);
        assert_eq!(write_policies[0].name, "File created notification");
    }

    #[test]
    fn test_deep_merge_fragment() {
        let loader = PolicyLoader::new();
        let mut target: PolicyFragment = HashMap::new();
        
        // Create first fragment
        let mut fragment1: PolicyFragment = HashMap::new();
        let mut pre_tool_use1 = HashMap::new();
        pre_tool_use1.insert("Bash".to_string(), vec![
            super::super::types::YamlPolicy {
                name: "Policy 1".to_string(),
                description: None,
                conditions: vec![],
                action: super::super::actions::Action::ProvideFeedback {
                    message: "Test 1".to_string(),
                    include_context: false,
                },
            }
        ]);
        fragment1.insert("PreToolUse".to_string(), pre_tool_use1);
        
        // Create second fragment with overlapping hook/matcher
        let mut fragment2: PolicyFragment = HashMap::new();
        let mut pre_tool_use2 = HashMap::new();
        pre_tool_use2.insert("Bash".to_string(), vec![
            super::super::types::YamlPolicy {
                name: "Policy 2".to_string(),
                description: None,
                conditions: vec![],
                action: super::super::actions::Action::ProvideFeedback {
                    message: "Test 2".to_string(),
                    include_context: false,
                },
            }
        ]);
        fragment2.insert("PreToolUse".to_string(), pre_tool_use2);
        
        // Merge fragments
        loader.deep_merge_fragment(&mut target, fragment1);
        loader.deep_merge_fragment(&mut target, fragment2);
        
        // Verify merge
        assert!(target.contains_key("PreToolUse"));
        let pre_tool_use = target.get("PreToolUse").unwrap();
        assert!(pre_tool_use.contains_key("Bash"));
        let bash_policies = pre_tool_use.get("Bash").unwrap();
        
        // Should have both policies concatenated
        assert_eq!(bash_policies.len(), 2);
        assert_eq!(bash_policies[0].name, "Policy 1");
        assert_eq!(bash_policies[1].name, "Policy 2");
    }

    #[test]
    fn test_validate_and_flatten_success() {
        let loader = PolicyLoader::new();
        let mut composed: PolicyFragment = HashMap::new();
        
        // Create test fragment
        let mut pre_tool_use = HashMap::new();
        pre_tool_use.insert("Bash".to_string(), vec![
            super::super::types::YamlPolicy {
                name: "Unique Policy 1".to_string(),
                description: Some("Test policy".to_string()),
                conditions: vec![],
                action: super::super::actions::Action::ProvideFeedback {
                    message: "Test message".to_string(),
                    include_context: false,
                },
            }
        ]);
        
        let mut post_tool_use = HashMap::new();
        post_tool_use.insert("Write".to_string(), vec![
            super::super::types::YamlPolicy {
                name: "Unique Policy 2".to_string(),
                description: None,
                conditions: vec![],
                action: super::super::actions::Action::ProvideFeedback {
                    message: "Test message 2".to_string(),
                    include_context: false,
                },
            }
        ]);
        
        composed.insert("PreToolUse".to_string(), pre_tool_use);
        composed.insert("PostToolUse".to_string(), post_tool_use);
        
        let result = loader.validate_and_flatten(composed).unwrap();
        
        assert_eq!(result.len(), 2);
        
        // Find policies by name since order might vary
        let policy1 = result.iter().find(|p| p.name == "Unique Policy 1").unwrap();
        assert_eq!(policy1.hook_event, HookEventType::PreToolUse);
        assert_eq!(policy1.matcher, "Bash");
        assert!(policy1.description.is_some());
        
        let policy2 = result.iter().find(|p| p.name == "Unique Policy 2").unwrap();
        assert_eq!(policy2.hook_event, HookEventType::PostToolUse);
        assert_eq!(policy2.matcher, "Write");
        assert!(policy2.description.is_none());
    }

    #[test]
    fn test_validate_and_flatten_duplicate_names() {
        let loader = PolicyLoader::new();
        let mut composed: PolicyFragment = HashMap::new();
        
        // Create fragment with duplicate policy names
        let mut pre_tool_use = HashMap::new();
        pre_tool_use.insert("Bash".to_string(), vec![
            super::super::types::YamlPolicy {
                name: "Duplicate Name".to_string(),
                description: None,
                conditions: vec![],
                action: super::super::actions::Action::ProvideFeedback {
                    message: "Test 1".to_string(),
                    include_context: false,
                },
            },
            super::super::types::YamlPolicy {
                name: "Duplicate Name".to_string(),
                description: None,
                conditions: vec![],
                action: super::super::actions::Action::ProvideFeedback {
                    message: "Test 2".to_string(),
                    include_context: false,
                },
            }
        ]);
        
        composed.insert("PreToolUse".to_string(), pre_tool_use);
        
        let result = loader.validate_and_flatten(composed);
        
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("Duplicate policy name"));
        assert!(error_msg.contains("Duplicate Name"));
    }

    #[test]
    fn test_parse_hook_event_valid() {
        let loader = PolicyLoader::new();
        
        assert_eq!(loader.parse_hook_event("PreToolUse").unwrap(), HookEventType::PreToolUse);
        assert_eq!(loader.parse_hook_event("PostToolUse").unwrap(), HookEventType::PostToolUse);
        assert_eq!(loader.parse_hook_event("Notification").unwrap(), HookEventType::Notification);
        assert_eq!(loader.parse_hook_event("Stop").unwrap(), HookEventType::Stop);
        assert_eq!(loader.parse_hook_event("SubagentStop").unwrap(), HookEventType::SubagentStop);
        assert_eq!(loader.parse_hook_event("PreCompact").unwrap(), HookEventType::PreCompact);
    }

    #[test]
    fn test_parse_hook_event_invalid() {
        let loader = PolicyLoader::new();
        
        let result = loader.parse_hook_event("InvalidEvent");
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("Unknown hook event type: InvalidEvent"));
    }

    #[test]
    fn test_load_and_compose_policies_integration() {
        let dir = tempdir().unwrap();
        let guardrails_dir = dir.path().join("guardrails");
        let policies_dir = guardrails_dir.join("policies");
        create_dir_all(&policies_dir).unwrap();
        
        // Create root config
        let cupcake_yaml = guardrails_dir.join("cupcake.yaml");
        let mut file = File::create(&cupcake_yaml).unwrap();
        file.write_all(br#"
settings:
  audit_logging: true
  debug_mode: false
imports:
  - "policies/*.yaml"
"#).unwrap();
        
        // Create first policy fragment
        let fragment1 = policies_dir.join("01-security.yaml");
        let mut file = File::create(&fragment1).unwrap();
        file.write_all(br#"
PreToolUse:
  "Bash":
    - name: "Block dangerous commands"
      conditions:
        - type: "pattern"
          field: "tool_input.command"
          regex: "^(rm|dd)\\s"
      action:
        type: "block_with_feedback"
        feedback_message: "Dangerous command blocked!"
        include_context: false
"#).unwrap();
        
        // Create second policy fragment
        let fragment2 = policies_dir.join("02-git.yaml");
        let mut file = File::create(&fragment2).unwrap();
        file.write_all(br#"
PreToolUse:
  "Bash":
    - name: "Git commit reminder"
      conditions:
        - type: "pattern"
          field: "tool_input.command"
          regex: "git\\s+commit"
      action:
        type: "provide_feedback"
        message: "Remember to run tests!"
        include_context: false

PostToolUse:
  "Write":
    - name: "File modified notification"
      conditions:
        - type: "match"
          field: "tool_name"
          value: "Write"
      action:
        type: "provide_feedback"
        message: "File has been modified"
        include_context: false
"#).unwrap();
        
        let mut loader = PolicyLoader::new();
        let composed_policies = loader.load_and_compose_policies(dir.path()).unwrap();
        
        assert_eq!(composed_policies.len(), 3);
        
        // Verify all policies are present
        let policy_names: Vec<&str> = composed_policies.iter().map(|p| p.name.as_str()).collect();
        assert!(policy_names.contains(&"Block dangerous commands"));
        assert!(policy_names.contains(&"Git commit reminder"));
        assert!(policy_names.contains(&"File modified notification"));
        
        // Verify policies under same hook/matcher are both present
        let bash_policies: Vec<_> = composed_policies.iter()
            .filter(|p| p.hook_event == HookEventType::PreToolUse && p.matcher == "Bash")
            .collect();
        assert_eq!(bash_policies.len(), 2);
        
        // Verify different hook events
        let pre_policies: Vec<_> = composed_policies.iter()
            .filter(|p| p.hook_event == HookEventType::PreToolUse)
            .collect();
        let post_policies: Vec<_> = composed_policies.iter()
            .filter(|p| p.hook_event == HookEventType::PostToolUse)
            .collect();
        
        assert_eq!(pre_policies.len(), 2);
        assert_eq!(post_policies.len(), 1);
    }
}
