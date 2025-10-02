//! Policy Validator - Cupcake-specific Rego policy validation
//!
//! Validates policies for Cupcake-specific requirements, style, and best practices.
//! Integrates with existing metadata parser.

use anyhow::Result;
use std::path::{Path, PathBuf};
use tracing::{debug, info};

pub mod decision_event_matrix;
pub mod rules;

#[cfg(test)]
mod tests;

use crate::engine::metadata::{extract_package_name, parse_metadata};
use rules::*;

/// Severity levels for validation issues
#[derive(Debug, Clone, PartialEq)]
pub enum Severity {
    Error,   // Policy won't work correctly
    Warning, // Style or best practice issue
    Info,    // Nice to know - suggestion
}

/// A validation issue found in a policy
#[derive(Debug, Clone)]
pub struct ValidationIssue {
    /// Severity of the issue
    pub severity: Severity,
    /// Unique identifier for this rule
    pub rule_id: &'static str,
    /// Human-readable description
    pub message: String,
    /// Line number where issue occurs (1-indexed)
    pub line: Option<usize>,
}

/// Parsed policy content with metadata and structure
#[derive(Debug, Clone)]
pub struct PolicyContent {
    /// Original file path
    pub path: PathBuf,
    /// Raw content
    pub content: String,
    /// Content split into lines for line-based operations
    pub lines: Vec<String>,
    /// Package name extracted from content
    pub package_name: Option<String>,
    /// Parsed metadata (if any)
    pub metadata: Option<crate::engine::metadata::PolicyMetadata>,
}

/// Trait for validation rules
pub trait ValidationRule: Send + Sync {
    /// Check policy for issues
    fn check(&self, policy: &PolicyContent) -> Vec<ValidationIssue>;

    /// Rule identifier
    fn rule_id(&self) -> &'static str;

    /// Rule description
    fn description(&self) -> &'static str;
}

/// Main policy validator
pub struct PolicyValidator {
    rules: Vec<Box<dyn ValidationRule>>,
}

/// Validation results for a single policy
#[derive(Debug)]
pub struct PolicyValidationResult {
    pub path: PathBuf,
    pub issues: Vec<ValidationIssue>,
    pub error_count: usize,
    pub warning_count: usize,
}

/// Validation results for multiple policies
#[derive(Debug)]
pub struct ValidationResult {
    pub policies: Vec<PolicyValidationResult>,
    pub total_errors: usize,
    pub total_warnings: usize,
}

impl PolicyContent {
    /// Create PolicyContent from file path
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let content = std::fs::read_to_string(&path)?;
        Self::from_content(path, content)
    }

    /// Create PolicyContent from content string
    pub fn from_content(path: PathBuf, content: String) -> Result<Self> {
        let lines = content.lines().map(String::from).collect();

        // Parse metadata
        let metadata = parse_metadata(&content)?;

        // Extract package name
        let package_name = extract_package_name(&content).ok();

        Ok(Self {
            path,
            content,
            lines,
            package_name,
            metadata,
        })
    }

    /// Update content and reparse derived fields
    pub fn update_content(&mut self, new_content: String) -> Result<()> {
        self.content = new_content;
        self.lines = self.content.lines().map(String::from).collect();

        // Re-parse metadata and package name
        self.metadata = parse_metadata(&self.content)?;
        self.package_name = extract_package_name(&self.content).ok();

        Ok(())
    }

    /// Write content back to file
    pub fn write_to_file(&self) -> Result<()> {
        std::fs::write(&self.path, &self.content)?;
        Ok(())
    }
}

impl PolicyValidator {
    /// Create validator with default Cupcake rules
    pub fn new() -> Self {
        let rules: Vec<Box<dyn ValidationRule>> = vec![
            Box::new(MetadataPlacementRule),
            Box::new(PackageDeclarationRule),
            Box::new(ObjectKeyMembershipRule),
            Box::new(DecisionStructureRule),
            Box::new(RoutingMetadataRule),
            Box::new(IncrementalRuleGroupingRule),
            Box::new(DecisionEventCompatibilityRule),
        ];

        Self { rules }
    }

    /// Validate a single policy file
    pub fn validate_policy(&self, policy: &PolicyContent) -> PolicyValidationResult {
        debug!("Validating policy: {:?}", policy.path);

        let mut issues = Vec::new();

        // Run all validation rules
        for rule in &self.rules {
            let rule_issues = rule.check(policy);
            issues.extend(rule_issues);
        }

        // Count issues by severity
        let error_count = issues
            .iter()
            .filter(|i| i.severity == Severity::Error)
            .count();
        let warning_count = issues
            .iter()
            .filter(|i| i.severity == Severity::Warning)
            .count();

        PolicyValidationResult {
            path: policy.path.clone(),
            issues,
            error_count,
            warning_count,
        }
    }

    /// Validate multiple policy files
    pub fn validate_policies(&self, policies: &[PolicyContent]) -> ValidationResult {
        info!("Validating {} policies", policies.len());

        let mut results = Vec::new();
        let mut total_errors = 0;
        let mut total_warnings = 0;

        for policy in policies {
            let result = self.validate_policy(policy);
            total_errors += result.error_count;
            total_warnings += result.warning_count;
            results.push(result);
        }

        ValidationResult {
            policies: results,
            total_errors,
            total_warnings,
        }
    }
}

impl Default for PolicyValidator {
    fn default() -> Self {
        Self::new()
    }
}
