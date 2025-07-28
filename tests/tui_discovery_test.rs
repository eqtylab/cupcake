use cupcake::cli::tui::init::discovery::{discover_files, mock_discover_files};
use cupcake::cli::tui::init::state::Agent;

#[tokio::test]
async fn test_mock_discovery() {
    let files = mock_discover_files().await.unwrap();
    
    // Should discover 6 files
    assert_eq!(files.len(), 6);
    
    // Check each agent is represented
    let agents: Vec<Agent> = files.iter().map(|f| f.agent).collect();
    assert!(agents.contains(&Agent::Claude));
    assert!(agents.contains(&Agent::Cursor));
    assert!(agents.contains(&Agent::Windsurf));
    assert!(agents.contains(&Agent::Kiro));
    assert!(agents.contains(&Agent::Copilot));
    assert!(agents.contains(&Agent::Aider));
    
    // Check directories have children
    let windsurf = files.iter().find(|f| f.agent == Agent::Windsurf).unwrap();
    assert!(windsurf.is_directory);
    assert_eq!(windsurf.children.len(), 3);
    
    let kiro = files.iter().find(|f| f.agent == Agent::Kiro).unwrap();
    assert!(kiro.is_directory);
    assert_eq!(kiro.children.len(), 2);
}

#[tokio::test]
async fn test_real_discovery_in_temp_dir() {
    use std::fs;
    use tempfile::TempDir;
    
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();
    
    // Create some test files
    fs::write(root.join("CLAUDE.md"), "# Claude rules").unwrap();
    fs::create_dir_all(root.join(".cursor")).unwrap();
    fs::write(root.join(".cursor/rules"), "cursor rules").unwrap();
    fs::create_dir_all(root.join(".windsurf/rules")).unwrap();
    fs::write(root.join(".windsurf/rules/test.md"), "windsurf rule").unwrap();
    
    let files = discover_files(root).await.unwrap();
    
    // Should find at least Claude and Cursor
    assert!(files.len() >= 2);
    
    let claude = files.iter().find(|f| f.agent == Agent::Claude);
    assert!(claude.is_some());
    assert!(!claude.unwrap().is_directory);
    
    let cursor = files.iter().find(|f| f.agent == Agent::Cursor);
    assert!(cursor.is_some());
    
    let windsurf = files.iter().find(|f| f.agent == Agent::Windsurf);
    assert!(windsurf.is_some());
    assert!(windsurf.unwrap().is_directory);
    assert_eq!(windsurf.unwrap().children.len(), 1);
}

#[test]
fn test_discovery_pattern_coverage() {
    use cupcake::cli::tui::init::discovery::DiscoveryPattern;
    
    let patterns = DiscoveryPattern::all();
    
    // Should have patterns for all 7 agents (including Gemini)
    assert_eq!(patterns.len(), 7);
    
    // Each pattern should have at least one glob
    for pattern in patterns {
        assert!(!pattern.patterns.is_empty());
    }
}