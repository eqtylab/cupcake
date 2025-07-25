#[cfg(test)]
mod tests {
    use cupcake::cli::tui::init::state::*;
    use std::path::PathBuf;

    #[test]
    fn test_review_state_initialization() {
        let state = ReviewState::default();
        assert!(state.rules.is_empty());
        assert!(state.selected.is_empty());
        assert!(state.filtered_indices.is_empty());
        assert_eq!(state.selected_index, 0);
        assert!(!state.search_active);
    }

    #[test]
    fn test_extracted_rule_structure() {
        let rule = ExtractedRule {
            id: 1,
            source_file: PathBuf::from("test.md"),
            description: "Test rule".to_string(),
            hook_description: "Test hook description".to_string(),
            severity: Severity::High,
            category: "testing".to_string(),
            when: "pre-commit".to_string(),
            block_on_violation: true,
            policy_decision: PolicyDecision {
                to_policy: true,
                rationale: "Important test rule".to_string(),
            },
        };
        
        assert_eq!(rule.id, 1);
        assert_eq!(rule.description, "Test rule");
        assert!(matches!(rule.severity, Severity::High));
        assert!(rule.block_on_violation);
    }

    #[test]
    fn test_severity_levels() {
        let high = Severity::High;
        let medium = Severity::Medium;
        let low = Severity::Low;
        
        assert!(matches!(high, Severity::High));
        assert!(matches!(medium, Severity::Medium));
        assert!(matches!(low, Severity::Low));
    }

    #[test]
    fn test_rule_edit_form() {
        let mut form = RuleEditForm::default();
        assert_eq!(form.current_field, FormField::Description);
        assert!(form.description.value().is_empty());
        assert_eq!(form.severity, Severity::Medium);
        assert!(!form.block_on_violation);
        
        // Test field navigation
        form.current_field = FormField::Severity;
        assert!(matches!(form.current_field, FormField::Severity));
    }

    #[test]
    fn test_review_state_with_rules() {
        let mut state = ReviewState::default();
        
        // Add some rules
        state.rules.push(ExtractedRule {
            id: 0,
            source_file: PathBuf::from("CLAUDE.md"),
            description: "Test rule 1".to_string(),
            hook_description: "Block dangerous operations".to_string(),
            severity: Severity::High,
            category: "test".to_string(),
            when: "always".to_string(),
            block_on_violation: true,
            policy_decision: PolicyDecision {
                to_policy: true,
                rationale: "High severity rule should be enforced".to_string(),
            },
        });
        
        state.rules.push(ExtractedRule {
            id: 1,
            source_file: PathBuf::from("CLAUDE.md"),
            description: "Test rule 2".to_string(),
            hook_description: "Provide helpful warning".to_string(),
            severity: Severity::Medium,
            category: "test".to_string(),
            when: "always".to_string(),
            block_on_violation: false,
            policy_decision: PolicyDecision {
                to_policy: true,
                rationale: "Medium severity rule for guidance".to_string(),
            },
        });
        
        assert_eq!(state.rules.len(), 2);
        
        // Test selection
        state.selected.insert(0);
        assert!(state.selected.contains(&0));
        assert!(!state.selected.contains(&1));
        
        // Test expanded sections
        state.expanded_sections.insert("CLAUDE.md".to_string());
        assert!(state.expanded_sections.contains("CLAUDE.md"));
    }

    #[test]
    fn test_form_fields() {
        // Test all form field variants
        assert!(matches!(FormField::Description, FormField::Description));
        assert!(matches!(FormField::Severity, FormField::Severity));
        assert!(matches!(FormField::Category, FormField::Category));
        assert!(matches!(FormField::When, FormField::When));
        assert!(matches!(FormField::BlockOnViolation, FormField::BlockOnViolation));
    }
}