#[cfg(test)]
mod tests {
    use cupcake::cli::tui::init::state::*;

    #[test]
    fn test_compilation_state_initialization() {
        let state = CompilationState::default();
        assert!(state.phases.is_empty());
        assert_eq!(state.current_phase, 0);
        assert_eq!(state.overall_progress, 0.0);
        assert!(!state.show_logs);
        assert!(state.logs.is_empty());
    }

    #[test]
    fn test_compilation_phase_structure() {
        let phase = CompilationPhase {
            name: "Test Phase".to_string(),
            status: PhaseStatus::Pending,
            details: vec!["Detail 1".to_string()],
            elapsed_ms: 100,
        };
        
        assert_eq!(phase.name, "Test Phase");
        assert!(matches!(phase.status, PhaseStatus::Pending));
        assert_eq!(phase.details.len(), 1);
        assert_eq!(phase.elapsed_ms, 100);
    }

    #[test]
    fn test_phase_status_variants() {
        // Test all phase status variants
        let pending = PhaseStatus::Pending;
        let in_progress = PhaseStatus::InProgress;
        let complete = PhaseStatus::Complete;
        let failed = PhaseStatus::Failed("Error message".to_string());
        
        assert!(matches!(pending, PhaseStatus::Pending));
        assert!(matches!(in_progress, PhaseStatus::InProgress));
        assert!(matches!(complete, PhaseStatus::Complete));
        
        if let PhaseStatus::Failed(err) = failed {
            assert_eq!(err, "Error message");
        } else {
            panic!("Expected Failed status");
        }
    }

    #[test]
    fn test_compilation_state_with_phases() {
        let state = CompilationState {
            phases: vec![
                CompilationPhase {
                    name: "Phase 1".to_string(),
                    status: PhaseStatus::Complete,
                    details: vec!["✓ Done".to_string()],
                    elapsed_ms: 100,
                },
                CompilationPhase {
                    name: "Phase 2".to_string(),
                    status: PhaseStatus::InProgress,
                    details: vec!["⟳ Working...".to_string()],
                    elapsed_ms: 0,
                },
                CompilationPhase {
                    name: "Phase 3".to_string(),
                    status: PhaseStatus::Pending,
                    details: vec![],
                    elapsed_ms: 0,
                },
            ],
            current_phase: 1,
            overall_progress: 0.5,
            show_logs: true,
            logs: vec!["Log entry 1".to_string(), "Log entry 2".to_string()],
            high_count: 5,
            medium_count: 10,
            low_count: 3,
        };
        
        assert_eq!(state.phases.len(), 3);
        assert_eq!(state.current_phase, 1);
        assert_eq!(state.overall_progress, 0.5);
        assert!(state.show_logs);
        assert_eq!(state.logs.len(), 2);
        
        // Test phase status checks
        assert!(matches!(state.phases[0].status, PhaseStatus::Complete));
        assert!(matches!(state.phases[1].status, PhaseStatus::InProgress));
        assert!(matches!(state.phases[2].status, PhaseStatus::Pending));
    }

    #[test]
    fn test_success_state() {
        let state = SuccessState {
            total_rules: 42,
            high_count: 10,
            medium_count: 20,
            low_count: 12,
            config_location: std::path::PathBuf::from("guardrails/cupcake.yaml"),
        };
        
        assert_eq!(state.total_rules, 42);
        assert_eq!(state.high_count, 10);
        assert_eq!(state.medium_count, 20);
        assert_eq!(state.low_count, 12);
        assert_eq!(state.config_location.to_str().unwrap(), "guardrails/cupcake.yaml");
    }
}