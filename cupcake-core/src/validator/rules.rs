//! Validation rules for Cupcake policies

use super::{PolicyContent, Severity, ValidationIssue, ValidationRule};
use regex::Regex;
use std::collections::HashMap;

/// Rule: Metadata with scope: package must be first in file
pub struct MetadataPlacementRule;

impl ValidationRule for MetadataPlacementRule {
    fn rule_id(&self) -> &'static str {
        "metadata-placement"
    }

    fn description(&self) -> &'static str {
        "Metadata with scope: package must be first in file"
    }

    fn check(&self, policy: &PolicyContent) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();

        // Look for METADATA blocks and package declarations in the raw content
        let mut metadata_line = None;
        let mut package_line = None;
        let mut has_package_scope = false;

        for (i, line) in policy.lines.iter().enumerate() {
            let trimmed = line.trim();
            if trimmed == "# METADATA" {
                metadata_line = Some(i);
            } else if trimmed.starts_with("package ") {
                package_line = Some(i);
            } else if metadata_line.is_some() && trimmed == "# scope: package" {
                has_package_scope = true;
            }
        }

        // If we have package-scoped metadata and a package declaration, check ordering
        if let (Some(meta_idx), Some(pkg_idx)) = (metadata_line, package_line) {
            if has_package_scope && meta_idx > pkg_idx {
                // Package-scoped metadata comes after package - this is wrong
                issues.push(ValidationIssue {
                    severity: Severity::Error,
                    rule_id: self.rule_id(),
                    message: "Package-scoped metadata must be the first thing in the file, before the package declaration".to_string(),
                    line: Some(pkg_idx + 1),
                });
            }
        }

        issues
    }
}

/// Rule: Must have valid package declaration
pub struct PackageDeclarationRule;

impl ValidationRule for PackageDeclarationRule {
    fn rule_id(&self) -> &'static str {
        "package-declaration"
    }

    fn description(&self) -> &'static str {
        "Policy must have a valid package declaration"
    }

    fn check(&self, policy: &PolicyContent) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();

        if policy.package_name.is_none() {
            issues.push(ValidationIssue {
                severity: Severity::Error,
                rule_id: self.rule_id(),
                message: "No valid package declaration found".to_string(),
                line: None,
            });
        }

        issues
    }
}

/// Rule: Check for Rego v1 object key membership issues
pub struct ObjectKeyMembershipRule;

impl ValidationRule for ObjectKeyMembershipRule {
    fn rule_id(&self) -> &'static str {
        "object-key-membership"
    }

    fn description(&self) -> &'static str {
        "Use object.keys() for object key membership in Rego v1"
    }

    fn check(&self, policy: &PolicyContent) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();

        // Pattern to detect "key" in object_name (simplified - will have some false positives)
        let object_membership_pattern =
            Regex::new(r#""[^"]+"\s+in\s+[a-zA-Z_][a-zA-Z0-9_\.]*"#).unwrap();

        for (i, line) in policy.lines.iter().enumerate() {
            if object_membership_pattern.is_match(line) {
                issues.push(ValidationIssue {
                    severity: Severity::Warning,
                    rule_id: self.rule_id(),
                    message: "Consider using object.keys() for object key membership in Rego v1"
                        .to_string(),
                    line: Some(i + 1),
                });
            }
        }

        issues
    }
}

/// Rule: Decision objects should have required structure
pub struct DecisionStructureRule;

impl ValidationRule for DecisionStructureRule {
    fn rule_id(&self) -> &'static str {
        "decision-structure"
    }

    fn description(&self) -> &'static str {
        "Decision objects should contain required fields (reason, severity, rule_id)"
    }

    fn check(&self, policy: &PolicyContent) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();

        // Look for decision := { patterns and check structure
        let decision_pattern = Regex::new(r"decision\s*:=\s*\{").unwrap();

        for (i, line) in policy.lines.iter().enumerate() {
            if decision_pattern.is_match(line) {
                // Check next few lines for required fields
                let mut has_reason = false;
                let mut has_rule_id = false;

                // Look ahead up to 10 lines for the closing brace
                for j in i..std::cmp::min(i + 10, policy.lines.len()) {
                    let check_line = &policy.lines[j];
                    if check_line.contains("\"reason\"") {
                        has_reason = true;
                    }
                    if check_line.contains("\"rule_id\"") {
                        has_rule_id = true;
                    }
                    if check_line.contains('}') {
                        break;
                    }
                }

                if !has_reason {
                    issues.push(ValidationIssue {
                        severity: Severity::Warning,
                        rule_id: self.rule_id(),
                        message: "Decision object should include 'reason' field".to_string(),
                        line: Some(i + 1),
                    });
                }

                if !has_rule_id {
                    issues.push(ValidationIssue {
                        severity: Severity::Warning,
                        rule_id: self.rule_id(),
                        message: "Decision object should include 'rule_id' field".to_string(),
                        line: Some(i + 1),
                    });
                }
            }
        }

        issues
    }
}

/// Rule: Routing metadata should be present
pub struct RoutingMetadataRule;

impl ValidationRule for RoutingMetadataRule {
    fn rule_id(&self) -> &'static str {
        "routing-metadata"
    }

    fn description(&self) -> &'static str {
        "Policy should have routing metadata for proper event handling"
    }

    fn check(&self, policy: &PolicyContent) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();

        // Skip system policies
        if let Some(pkg_name) = &policy.package_name {
            if pkg_name.starts_with("cupcake.system") {
                return issues;
            }
        }

        if let Some(metadata) = &policy.metadata {
            if metadata.custom.routing.is_none() {
                issues.push(ValidationIssue {
                    severity: Severity::Warning,
                    rule_id: self.rule_id(),
                    message:
                        "Policy should have routing metadata (required_events, required_tools)"
                            .to_string(),
                    line: None,
                });
            }
        } else {
            issues.push(ValidationIssue {
                severity: Severity::Warning,
                rule_id: self.rule_id(),
                message: "Policy should have metadata with routing information".to_string(),
                line: None,
            });
        }

        issues
    }
}

/// Rule: Incremental rules should be grouped together
pub struct IncrementalRuleGroupingRule;

impl ValidationRule for IncrementalRuleGroupingRule {
    fn rule_id(&self) -> &'static str {
        "incremental-rule-grouping"
    }

    fn description(&self) -> &'static str {
        "Rules with the same name should be grouped together"
    }

    fn check(&self, policy: &PolicyContent) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();

        // Find all rule declarations (decision verbs)
        let rule_pattern = Regex::new(r"^([a-zA-Z_][a-zA-Z0-9_]*)\s+contains\s+").unwrap();
        let mut rule_locations: HashMap<String, Vec<usize>> = HashMap::new();

        for (i, line) in policy.lines.iter().enumerate() {
            let trimmed = line.trim();
            if let Some(captures) = rule_pattern.captures(trimmed) {
                if let Some(rule_name) = captures.get(1) {
                    let name = rule_name.as_str().to_string();
                    rule_locations.entry(name).or_default().push(i);
                }
            }
        }

        // Check if rules with same name are grouped
        for (rule_name, locations) in rule_locations {
            if locations.len() > 1 {
                // Check if they're consecutive (allowing comments/empty lines between)
                let mut is_grouped = true;

                // Sort locations
                let mut sorted_locs = locations.clone();
                sorted_locs.sort();

                // Check for significant gaps (more than just comments/whitespace)
                for window in sorted_locs.windows(2) {
                    let gap_start = window[0] + 1;
                    let gap_end = window[1];

                    // Check if there are other rules in between
                    for i in gap_start..gap_end {
                        let line = &policy.lines[i];
                        let trimmed = line.trim();

                        // If there's a different rule type in between, it's not grouped
                        if rule_pattern.is_match(trimmed) {
                            if let Some(captures) = rule_pattern.captures(trimmed) {
                                if let Some(other_rule) = captures.get(1) {
                                    if other_rule.as_str() != rule_name {
                                        is_grouped = false;
                                        break;
                                    }
                                }
                            }
                        }
                    }

                    if !is_grouped {
                        break;
                    }
                }

                if !is_grouped {
                    issues.push(ValidationIssue {
                        severity: Severity::Warning,
                        rule_id: self.rule_id(),
                        message: format!("Multiple '{rule_name}' rules should be grouped together"),
                        line: Some(sorted_locs[0] + 1),
                    });
                }
            }
        }

        issues
    }
}
