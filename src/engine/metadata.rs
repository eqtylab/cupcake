//! OPA Metadata Parser - Standard metadata-driven routing system
//! 
//! Implements the NEW_GUIDING_FINAL.md metadata parsing specification.
//! Replaces the deprecated custom `selector` parser with standard OPA metadata.
//! 
//! This module enables Host-Side Indexing by parsing `# METADATA` blocks
//! and extracting routing directives for O(1) policy lookups.

use anyhow::{anyhow, bail, Context, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

/// Routing directive extracted from OPA metadata
/// Maps 1:1 to the NEW_GUIDING_FINAL.md specification
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RoutingDirective {
    /// Events this policy applies to (e.g., ["PreToolUse", "PostToolUse"])
    #[serde(default)]
    pub required_events: Vec<String>,
    
    /// Tools this policy applies to (e.g., ["Bash", "WebFetch"])
    #[serde(default)]
    pub required_tools: Vec<String>,
    
    /// External signals required by this policy
    #[serde(default)]
    pub required_signals: Vec<String>,
}

impl Default for RoutingDirective {
    fn default() -> Self {
        Self {
            required_events: Vec::new(),
            required_tools: Vec::new(),
            required_signals: Vec::new(),
        }
    }
}

/// Complete metadata block structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyMetadata {
    /// Optional scope (rule, document, package, subpackages)
    #[serde(default)]
    pub scope: Option<String>,
    
    /// Policy title for documentation
    #[serde(default)]
    pub title: Option<String>,
    
    /// Authors/owners of this policy
    #[serde(default)]
    pub authors: Vec<String>,
    
    /// Organizations responsible
    #[serde(default)]
    pub organizations: Vec<String>,
    
    /// Custom fields including routing directives
    #[serde(default)]
    pub custom: CustomMetadata,
}

/// Custom metadata fields specific to Cupcake
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CustomMetadata {
    /// Severity level (HIGH, MEDIUM, LOW)
    #[serde(default)]
    pub severity: Option<String>,
    
    /// Unique policy ID
    #[serde(default)]
    pub id: Option<String>,
    
    /// Routing directive for Host-Side Indexing
    #[serde(default)]
    pub routing: Option<RoutingDirective>,
}

/// Extract and parse OPA metadata from a .rego file
pub fn parse_metadata(content: &str) -> Result<Option<PolicyMetadata>> {
    debug!("Parsing OPA metadata from policy content");
    
    // Find METADATA comment blocks
    let metadata_yaml = extract_metadata_yaml(content)?;
    
    if metadata_yaml.is_empty() {
        debug!("No metadata found in policy");
        return Ok(None);
    }
    
    debug!("Extracted YAML content: {}", metadata_yaml);
    
    // Parse YAML into metadata struct
    let metadata: PolicyMetadata = serde_yaml_ng::from_str(&metadata_yaml)
        .with_context(|| format!("Failed to parse metadata YAML: {}", metadata_yaml))?;
    
    debug!("Successfully parsed metadata: {:?}", metadata);
    Ok(Some(metadata))
}

/// Extract YAML content from # METADATA comment blocks
fn extract_metadata_yaml(content: &str) -> Result<String> {
    let mut yaml_lines = Vec::new();
    let mut in_metadata_block = false;
    
    for line in content.lines() {
        let trimmed = line.trim();
        
        if trimmed == "# METADATA" {
            in_metadata_block = true;
            continue;
        }
        
        if in_metadata_block {
            if trimmed.starts_with('#') {
                // Remove leading '# ' from comment lines
                let yaml_line = if trimmed.len() > 2 && trimmed.starts_with("# ") {
                    &trimmed[2..]
                } else if trimmed == "#" {
                    ""
                } else {
                    // Line like "#   key: value" - remove '#' and keep indentation
                    &trimmed[1..]
                };
                yaml_lines.push(yaml_line);
            } else {
                // Any non-comment line (empty or otherwise) ends the metadata block
                break;
            }
        }
    }
    
    Ok(yaml_lines.join("\n"))
}


/// Validate a routing directive for completeness
pub fn validate_routing_directive(directive: &RoutingDirective, package_name: &str) -> Result<()> {
    let is_system_policy = package_name.starts_with("cupcake.system");
    
    if is_system_policy {
        // System policies: aggregation functions that don't need event-specific routing
        if !directive.required_events.is_empty() {
            return Err(anyhow!("System policy '{}' should not specify required_events - system policies are aggregation functions", package_name));
        }
        if !directive.required_tools.is_empty() {
            return Err(anyhow!("System policy '{}' should not specify required_tools - system policies don't operate on specific tools", package_name));
        }
        if !directive.required_signals.is_empty() {
            return Err(anyhow!("System policy '{}' should not specify required_signals - system policies work with pre-aggregated data", package_name));
        }
        debug!("System policy {} has valid empty routing directive", package_name);
        return Ok(());
    }
    
    // Regular policies: business logic that responds to specific events
    if directive.required_events.is_empty() && !directive.required_tools.is_empty() {
        return Err(anyhow!(
            "Policy '{}' specifies required_tools {:?} but no required_events. Tools are only meaningful in the context of specific events (e.g., PreToolUse:Bash)", 
            package_name, 
            directive.required_tools
        ));
    }
    
    // Validate known Claude Code event types
    let valid_events = [
        "PreToolUse",
        "PostToolUse", 
        "UserPromptSubmit",
        "Stop",
        "SubagentStop",
        "Notification",
        "PreCompact",
        "SessionStart",
    ];
    
    for event in &directive.required_events {
        if !valid_events.contains(&event.as_str()) {
            warn!("Unknown event type '{}' in routing directive for policy '{}'. Known events: {:?}", 
                  event, package_name, valid_events);
        }
    }
    
    // Tools and signals are optional for regular policies
    debug!("Regular policy {} has valid routing directive", package_name);
    Ok(())
}

/// Extract package name from Rego content (reused from parser.rs)
pub fn extract_package_name(content: &str) -> Result<String> {
    let package_regex = Regex::new(r"(?m)^package\s+([a-zA-Z0-9._]+)")?;
    
    if let Some(captures) = package_regex.captures(content) {
        if let Some(name) = captures.get(1) {
            return Ok(name.as_str().to_string());
        }
    }
    
    bail!("No package declaration found in policy")
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_metadata_basic() {
        let content = r#"
package cupcake.policies.test

# METADATA
# scope: rule
# title: Test Policy
# authors: ["Security Team"]
# custom:
#   severity: HIGH
#   id: TEST-001
#   routing:
#     required_events: ["PreToolUse"]
#     required_tools: ["Bash"]

deny contains decision if {
    input.tool_name == "Bash"
}
"#;
        
        let metadata = parse_metadata(content).unwrap().unwrap();
        assert_eq!(metadata.title, Some("Test Policy".to_string()));
        assert_eq!(metadata.authors, vec!["Security Team"]);
        
        let routing = metadata.custom.routing.unwrap();
        assert_eq!(routing.required_events, vec!["PreToolUse"]);
        assert_eq!(routing.required_tools, vec!["Bash"]);
    }
    
    #[test]
    fn test_parse_metadata_minimal() {
        let content = r#"
package cupcake.policies.test

# METADATA
# custom:
#   routing:
#     required_events: ["UserPromptSubmit"]

allow if { true }
"#;
        
        let metadata = parse_metadata(content).unwrap().unwrap();
        let routing = metadata.custom.routing.unwrap();
        assert_eq!(routing.required_events, vec!["UserPromptSubmit"]);
        assert!(routing.required_tools.is_empty());
    }
    
    #[test] 
    fn test_no_metadata() {
        let content = r#"
package cupcake.policies.test

deny if {
    input.tool_name == "Bash"
}
"#;
        
        let result = parse_metadata(content).unwrap();
        assert!(result.is_none());
    }
    
    #[test]
    fn test_validate_routing_directive() {
        // Test valid regular policy
        let valid_regular = RoutingDirective {
            required_events: vec!["PreToolUse".to_string()],
            required_tools: vec!["Bash".to_string()],
            required_signals: vec![],
        };
        assert!(validate_routing_directive(&valid_regular, "cupcake.policies.bash_guard").is_ok());
        
        // Test invalid regular policy - tools without events
        let invalid_regular = RoutingDirective {
            required_events: vec![], // Empty events but has tools - should fail for regular policies
            required_tools: vec!["Bash".to_string()],
            required_signals: vec![],
        };
        assert!(validate_routing_directive(&invalid_regular, "cupcake.policies.bash_guard").is_err());
        
        // Test valid system policy - empty routing
        let valid_system = RoutingDirective {
            required_events: vec![],
            required_tools: vec![],
            required_signals: vec![],
        };
        assert!(validate_routing_directive(&valid_system, "cupcake.system.evaluate").is_ok());
        
        // Test invalid system policy - system policies shouldn't have routing requirements
        let invalid_system = RoutingDirective {
            required_events: vec!["PreToolUse".to_string()],
            required_tools: vec![],
            required_signals: vec![],
        };
        assert!(validate_routing_directive(&invalid_system, "cupcake.system.evaluate").is_err());
        
        // Test valid regular policy with empty events and empty tools - event-agnostic policies are OK
        let valid_empty_regular = RoutingDirective {
            required_events: vec![],
            required_tools: vec![],
            required_signals: vec!["global_setting".to_string()],
        };
        assert!(validate_routing_directive(&valid_empty_regular, "cupcake.policies.global_rules").is_ok());
    }
    
    #[test]
    fn test_extract_package_name() {
        let content = r#"
# Comment
package cupcake.policies.security.network

import rego.v1
"#;
        
        let package = extract_package_name(content).unwrap();
        assert_eq!(package, "cupcake.policies.security.network");
    }
}

// Aligns with NEW_GUIDING_FINAL.md:
// - Standard OPA metadata parsing replaces custom selector blocks
// - RoutingDirective enables Host-Side Indexing for O(1) lookups  
// - Supports required_events, required_tools, and required_signals
// - Validates against known Claude Code event types
// - Foundation for metadata-driven routing in the Hybrid Model