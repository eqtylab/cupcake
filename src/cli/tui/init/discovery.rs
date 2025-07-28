use std::collections::HashMap;
use std::path::{Path, PathBuf};
use glob::glob;
use walkdir::WalkDir;

use crate::Result;
use super::state::{Agent, RuleFile};

/// Pattern for discovering agent configuration files
pub struct DiscoveryPattern {
    /// Glob patterns to search for
    pub patterns: Vec<&'static str>,
    /// Agent type for discovered files
    pub agent: Agent,
    /// Whether to check if it's a directory
    pub is_directory: bool,
}

impl DiscoveryPattern {
    /// Get all discovery patterns
    pub fn all() -> Vec<Self> {
        vec![
            // Claude
            DiscoveryPattern {
                patterns: vec!["CLAUDE.md", "AGENT.md", "AGENTS.md"],
                agent: Agent::Claude,
                is_directory: false,
            },
            // Cursor
            DiscoveryPattern {
                patterns: vec![".cursor/rules", ".cursor/RULES", ".cursorrules"],
                agent: Agent::Cursor,
                is_directory: false,
            },
            // Windsurf
            DiscoveryPattern {
                patterns: vec![".windsurf/rules/", ".windsurf/rules"],
                agent: Agent::Windsurf,
                is_directory: true,
            },
            // Kiro
            DiscoveryPattern {
                patterns: vec![".kiro/steering/", ".kiro/steering"],
                agent: Agent::Kiro,
                is_directory: true,
            },
            // Copilot
            DiscoveryPattern {
                patterns: vec!["copilot-instructions", ".copilot-instructions", ".github/copilot-instructions.md"],
                agent: Agent::Copilot,
                is_directory: false,
            },
            // Aider
            DiscoveryPattern {
                patterns: vec![".aider.conf.yml", ".aider.conf.yaml", ".aider"],
                agent: Agent::Aider,
                is_directory: false,
            },
            // Gemini
            DiscoveryPattern {
                patterns: vec!["GEMINI.md"],
                agent: Agent::Gemini,
                is_directory: false,
            },
        ]
    }
}

/// Discover rule files in the given directory
pub async fn discover_files(root_dir: &Path) -> Result<Vec<RuleFile>> {
    let root_dir = root_dir.to_path_buf();
    
    // Run discovery in blocking task
    let files = tokio::task::spawn_blocking(move || {
        discover_files_sync(&root_dir)
    }).await.map_err(|e| anyhow::anyhow!("Task join error: {}", e))??;
    
    Ok(files)
}

/// Synchronous file discovery
fn discover_files_sync(root_dir: &Path) -> Result<Vec<RuleFile>> {
    let mut discovered = Vec::new();
    let mut seen = HashMap::new();
    
    // For case-insensitive deduplication on macOS
    let mut seen_canonical = HashMap::new();
    
    for pattern in DiscoveryPattern::all() {
        for glob_pattern in pattern.patterns {
            // Try both relative and absolute patterns
            let full_pattern = root_dir.join(glob_pattern);
            let pattern_str = full_pattern.to_string_lossy();
            
            // Use glob for pattern matching
            if let Ok(entries) = glob(&pattern_str) {
                for entry in entries.flatten() {
                    // Get canonical path for deduplication
                    let canonical = entry.canonicalize().unwrap_or(entry.clone());
                    
                    // Skip if we've already seen this file (case-insensitive check)
                    if seen_canonical.contains_key(&canonical) {
                        continue;
                    }
                    
                    // Check if it matches our expectations
                    let is_dir = entry.is_dir();
                    if pattern.is_directory != is_dir {
                        continue;
                    }
                    
                    // Create RuleFile
                    let mut rule_file = RuleFile {
                        path: entry.clone(),
                        agent: pattern.agent,
                        is_directory: is_dir,
                        children: vec![],
                    };
                    
                    // If it's a directory, discover children
                    if is_dir {
                        rule_file.children = discover_children(&entry)?;
                    }
                    
                    seen.insert(entry.clone(), discovered.len());
                    seen_canonical.insert(canonical, discovered.len());
                    discovered.push(rule_file);
                }
            }
        }
    }
    
    // Sort by agent type for consistent ordering
    discovered.sort_by_key(|f| f.agent as u8);
    
    Ok(discovered)
}

/// Discover markdown and yaml files within a directory
fn discover_children(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut children = Vec::new();
    
    // Common directories to exclude from traversal
    let excluded_dirs = [
        "node_modules", ".git", "target", "build", "dist", ".next", "__pycache__",
        ".env", "vendor", "bin", "obj", ".terraform", ".cache", "tmp", "temp",
        ".nyc_output", "coverage", ".pytest_cache", ".vscode", ".idea", 
        ".DS_Store", "Thumbs.db", ".tmp", ".temp", "logs", "log", ".log",
        "out", "output", ".output", ".build", ".dist", ".target", ".bin",
        ".gradle", ".maven", "bazel-*", ".bazel", "cmake-build-*"
    ];
    
    // Check if a directory should be excluded
    let should_exclude_dir = |dir_name: &str| -> bool {
        excluded_dirs.iter().any(|&excluded| {
            dir_name == excluded || 
            dir_name.starts_with(excluded) ||
            (excluded.contains('*') && dir_name.contains(&excluded.replace('*', "")))
        })
    };
    
    for entry in WalkDir::new(dir)
        .min_depth(1)
        .max_depth(12) // Increased depth for deeper project structures
        .into_iter()
        .filter_entry(|e| {
            // Allow files, but filter out excluded directories
            if e.file_type().is_dir() {
                if let Some(dir_name) = e.file_name().to_str() {
                    !should_exclude_dir(dir_name)
                } else {
                    true // Allow if we can't get the name
                }
            } else {
                true // Always allow files
            }
        })
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        
        // Only include markdown and yaml files
        if path.is_file() {
            if let Some(ext) = path.extension() {
                let ext = ext.to_string_lossy().to_lowercase();
                if matches!(ext.as_str(), "md" | "markdown" | "yml" | "yaml") {
                    children.push(path.to_path_buf());
                }
            }
        }
    }
    
    Ok(children)
}

/// Mock discovery for testing
pub async fn mock_discover_files() -> Result<Vec<RuleFile>> {
    use std::time::Duration;
    use tokio::time::sleep;
    
    // Simulate some delay
    sleep(Duration::from_millis(100)).await;
    
    Ok(vec![
        RuleFile {
            path: PathBuf::from("CLAUDE.md"),
            agent: Agent::Claude,
            is_directory: false,
            children: vec![],
        },
        RuleFile {
            path: PathBuf::from(".cursor/rules"),
            agent: Agent::Cursor,
            is_directory: false,
            children: vec![],
        },
        RuleFile {
            path: PathBuf::from(".windsurf/rules/"),
            agent: Agent::Windsurf,
            is_directory: true,
            children: vec![
                PathBuf::from(".windsurf/rules/formatting.md"),
                PathBuf::from(".windsurf/rules/security.md"),
                PathBuf::from(".windsurf/rules/performance.md"),
            ],
        },
        RuleFile {
            path: PathBuf::from(".kiro/steering/"),
            agent: Agent::Kiro,
            is_directory: true,
            children: vec![
                PathBuf::from(".kiro/steering/agent-policy.yml"),
                PathBuf::from(".kiro/steering/constraints.yml"),
            ],
        },
        RuleFile {
            path: PathBuf::from("copilot-instructions"),
            agent: Agent::Copilot,
            is_directory: false,
            children: vec![],
        },
        RuleFile {
            path: PathBuf::from(".aider.conf.yml"),
            agent: Agent::Aider,
            is_directory: false,
            children: vec![],
        },
    ])
}