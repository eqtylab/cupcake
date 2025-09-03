//! Tests for the policy validator

use super::*;
use super::rules::*;
use std::path::PathBuf;

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_policy(content: &str) -> PolicyContent {
        PolicyContent::from_content(
            PathBuf::from("test.rego"),
            content.to_string(),
        ).unwrap()
    }

    #[test]
    fn test_metadata_placement_rule_correct() {
        let content = r#"# METADATA
# scope: package
# title: Test Policy
package cupcake.policies.test

import rego.v1

deny contains decision if {
    true
    decision := {"reason": "test", "rule_id": "TEST-001"}
}"#;

        let policy = create_test_policy(content);
        let rule = MetadataPlacementRule;
        let issues = rule.check(&policy);
        
        assert!(issues.is_empty(), "Should not have issues with correct placement");
    }

    #[test]
    fn test_metadata_placement_rule_incorrect() {
        let content = r#"package cupcake.policies.test

import rego.v1

# METADATA
# scope: package
# title: Test Policy

deny contains decision if {
    true
    decision := {"reason": "test", "rule_id": "TEST-001"}
}"#;

        let policy = create_test_policy(content);
        let rule = MetadataPlacementRule;
        let issues = rule.check(&policy);
        
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].severity, Severity::Error);
        assert_eq!(issues[0].rule_id, "metadata-placement");
    }

    #[test]
    fn test_package_declaration_rule() {
        let content = r#"# No package declaration
import rego.v1

deny contains decision if {
    true
    decision := {"reason": "test", "rule_id": "TEST-001"}
}"#;

        let policy = create_test_policy(content);
        let rule = PackageDeclarationRule;
        let issues = rule.check(&policy);
        
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].severity, Severity::Error);
        assert_eq!(issues[0].rule_id, "package-declaration");
    }

    #[test]
    fn test_object_key_membership_rule() {
        let content = r#"package cupcake.policies.test

import rego.v1

deny contains decision if {
    "key" in my_object
    decision := {"reason": "test", "rule_id": "TEST-001"}
}"#;

        let policy = create_test_policy(content);
        let rule = ObjectKeyMembershipRule;
        let issues = rule.check(&policy);
        
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].severity, Severity::Warning);
        assert_eq!(issues[0].rule_id, "object-key-membership");
    }


    #[test]
    fn test_decision_structure_rule() {
        let content = r#"package cupcake.policies.test

import rego.v1

deny contains decision if {
    true
    decision := {"severity": "HIGH"}
}"#;

        let policy = create_test_policy(content);
        let rule = DecisionStructureRule;
        let issues = rule.check(&policy);
        
        assert_eq!(issues.len(), 2); // Missing both reason and rule_id
        assert!(issues.iter().any(|i| i.message.contains("reason")));
        assert!(issues.iter().any(|i| i.message.contains("rule_id")));
    }

    #[test]
    fn test_incremental_rule_grouping_rule() {
        let content = r#"package cupcake.policies.test

import rego.v1

deny contains decision if {
    input.x == 1
    decision := {"reason": "test1", "rule_id": "TEST-001"}
}

halt contains decision if {
    input.dangerous
    decision := {"reason": "danger", "rule_id": "TEST-002"}  
}

deny contains decision if {
    input.x == 2
    decision := {"reason": "test2", "rule_id": "TEST-003"}
}"#;

        let policy = create_test_policy(content);
        let rule = IncrementalRuleGroupingRule;
        let issues = rule.check(&policy);
        
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].severity, Severity::Warning);
        assert_eq!(issues[0].rule_id, "incremental-rule-grouping");
        assert!(issues[0].message.contains("deny"));
    }

    #[test]
    fn test_routing_metadata_rule() {
        let content = r#"package cupcake.policies.test

import rego.v1

deny contains decision if {
    true
    decision := {"reason": "test", "rule_id": "TEST-001"}
}"#;

        let policy = create_test_policy(content);
        let rule = RoutingMetadataRule;
        let issues = rule.check(&policy);
        
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].severity, Severity::Warning);
        assert_eq!(issues[0].rule_id, "routing-metadata");
    }

    #[test]
    fn test_routing_metadata_rule_system_policy_skip() {
        let content = r#"package cupcake.system.test

import rego.v1

evaluate := {
    "halts": []
}"#;

        let policy = create_test_policy(content);
        let rule = RoutingMetadataRule;
        let issues = rule.check(&policy);
        
        assert!(issues.is_empty(), "System policies should not need routing metadata");
    }

    #[test]
    fn test_full_validator() {
        let content = r#"# METADATA
# scope: package
# title: Test Policy
# custom:
#   routing:
#     required_events: ["PreToolUse"]
#     required_tools: ["Bash"]
package cupcake.policies.test

import rego.v1

deny contains decision if {
    true
    decision := {
        "reason": "Test policy",
        "severity": "HIGH", 
        "rule_id": "TEST-001"
    }
}"#;

        let policy = create_test_policy(content);
        let validator = PolicyValidator::new();
        let result = validator.validate_policy(&policy);
        
        assert_eq!(result.error_count, 0);
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn test_validator_with_issues() {
        let content = r#"package cupcake.policies.test

import rego.v1

# METADATA
# scope: package

deny contains decision if {
    "key" in my_object
    decision := {"severity": "HIGH"}
}"#;

        let policy = create_test_policy(content);
        let validator = PolicyValidator::new();
        let result = validator.validate_policy(&policy);
        
        assert!(result.error_count > 0 || result.warning_count > 0);
        assert!(result.issues.iter().any(|i| i.rule_id == "metadata-placement"));
        assert!(result.issues.iter().any(|i| i.rule_id == "object-key-membership"));
    }
}