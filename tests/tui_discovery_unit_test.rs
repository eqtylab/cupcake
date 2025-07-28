#[cfg(test)]
mod tests {
    use cupcake::cli::tui::init::state::{Agent, RuleFile};
    use cupcake::cli::tui::init::discovery::DiscoveryPattern;
    use cupcake::cli::tui::init::preview;
    use std::path::PathBuf;

    #[test]
    fn test_discovery_patterns() {
        let patterns = DiscoveryPattern::all();
        assert_eq!(patterns.len(), 7);
        
        // Verify each agent has patterns
        let agents: Vec<Agent> = patterns.iter().map(|p| p.agent).collect();
        assert!(agents.contains(&Agent::Claude));
        assert!(agents.contains(&Agent::Cursor));
        assert!(agents.contains(&Agent::Windsurf));
        assert!(agents.contains(&Agent::Kiro));
        assert!(agents.contains(&Agent::Copilot));
        assert!(agents.contains(&Agent::Aider));
        assert!(agents.contains(&Agent::Gemini));
    }

    #[test]
    fn test_rule_file_structure() {
        let rule_file = RuleFile {
            path: PathBuf::from("test.md"),
            agent: Agent::Claude,
            is_directory: false,
            children: vec![],
        };
        
        assert_eq!(rule_file.path, PathBuf::from("test.md"));
        assert_eq!(rule_file.agent.as_str(), "Claude");
        assert!(!rule_file.is_directory);
        assert!(rule_file.children.is_empty());
    }

    #[test]
    fn test_discovery_patterns_coverage() {
        let patterns = DiscoveryPattern::all();
        
        // Check Claude patterns
        let claude = patterns.iter().find(|p| p.agent == Agent::Claude).unwrap();
        assert!(claude.patterns.contains(&"CLAUDE.md"));
        assert!(!claude.is_directory);
        
        // Check Windsurf patterns
        let windsurf = patterns.iter().find(|p| p.agent == Agent::Windsurf).unwrap();
        assert!(windsurf.patterns.contains(&".windsurf/rules/"));
        assert!(windsurf.is_directory);
    }

    #[test]
    fn test_mock_preview() {
        let claude_path = PathBuf::from("CLAUDE.md");
        let preview = preview::mock_preview(&claude_path);
        assert!(preview.contains("Claude Development Rules"));
        assert!(preview.contains("Testing Standards"));
        
        let cursor_path = PathBuf::from(".cursor/rules");
        let preview = preview::mock_preview(&cursor_path);
        assert!(preview.contains("Cursor AI Rules"));
        
        let unknown_path = PathBuf::from("unknown.txt");
        let preview = preview::mock_preview(&unknown_path);
        assert!(preview.contains("Preview for: unknown.txt"));
    }

    #[test]
    fn test_agent_string_representation() {
        assert_eq!(Agent::Claude.as_str(), "Claude");
        assert_eq!(Agent::Cursor.as_str(), "Cursor");
        assert_eq!(Agent::Windsurf.as_str(), "Windsurf");
        assert_eq!(Agent::Kiro.as_str(), "Kiro");
        assert_eq!(Agent::Copilot.as_str(), "Copilot");
        assert_eq!(Agent::Aider.as_str(), "Aider");
    }
}