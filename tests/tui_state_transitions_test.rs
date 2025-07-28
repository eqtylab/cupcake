use cupcake::cli::tui::init::state::{WizardState, StateTransition, DiscoveryState, Severity};

#[test]
fn test_wizard_state_names() {
    let discovery = WizardState::Discovery(DiscoveryState::default());
    assert_eq!(discovery.name(), "Discovery");
    
    let extraction = WizardState::Extraction(Default::default());
    assert_eq!(extraction.name(), "Extraction");
    
    let review = WizardState::Review(Default::default());
    assert_eq!(review.name(), "Review");
    
    let compilation = WizardState::Compilation(Default::default());
    assert_eq!(compilation.name(), "Compilation");
    
    let success = WizardState::Success(cupcake::cli::tui::init::state::SuccessState {
        total_rules: 10,
        high_count: 3,
        medium_count: 5,
        low_count: 2,
        config_location: "test".into(),
    });
    assert_eq!(success.name(), "Success");
}

#[test]
fn test_can_go_back() {
    let discovery = WizardState::Discovery(Default::default());
    assert!(discovery.can_go_back());
    
    let success = WizardState::Success(cupcake::cli::tui::init::state::SuccessState {
        total_rules: 10,
        high_count: 3,
        medium_count: 5,
        low_count: 2,
        config_location: "test".into(),
    });
    assert!(!success.can_go_back());
}

#[test]
fn test_can_quit() {
    let discovery = WizardState::Discovery(Default::default());
    assert!(discovery.can_quit());
    
    let compilation = WizardState::Compilation(Default::default());
    assert!(!compilation.can_quit());
}

#[test]
fn test_state_transition_enum() {
    // Just ensure the enum values exist and can be compared
    assert_eq!(StateTransition::Continue, StateTransition::Continue);
    assert_ne!(StateTransition::Continue, StateTransition::Back);
    assert_ne!(StateTransition::Skip, StateTransition::Quit);
}

#[test]
fn test_severity_default() {
    let severity = Severity::default();
    assert_eq!(severity, Severity::Medium);
}

#[test]
fn test_agent_names() {
    use cupcake::cli::tui::init::state::Agent;
    
    assert_eq!(Agent::Claude.as_str(), "Claude");
    assert_eq!(Agent::Cursor.as_str(), "Cursor");
    assert_eq!(Agent::Windsurf.as_str(), "Windsurf");
    assert_eq!(Agent::Kiro.as_str(), "Kiro");
    assert_eq!(Agent::Copilot.as_str(), "Copilot");
    assert_eq!(Agent::Aider.as_str(), "Aider");
}