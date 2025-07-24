//! Concurrent rule extraction engine
//!
//! This module handles the parallel extraction of rules from multiple files.
//! Each file is processed in its own async task, with progress updates sent
//! back to the main UI thread.

use std::path::PathBuf;
use std::time::Duration;
use tokio::time::sleep;
use anyhow::Result;

use crate::cli::tui::init::events::AppEvent;
use crate::cli::tui::init::state::{ExtractedRule, Severity, PolicyDecision};

/// Extract rules from a single file
/// 
/// This spawns an async task that:
/// 1. Sends start event
/// 2. Simulates extraction with progress updates
/// 3. Returns stub rules
/// 4. Sends completion event
pub fn spawn_extraction_task(
    file_path: PathBuf,
    event_tx: tokio::sync::mpsc::UnboundedSender<AppEvent>,
) {
    tokio::spawn(async move {
        // Send start event
        let _ = event_tx.send(AppEvent::ExtractionStarted {
            file: file_path.clone(),
        });
        
        // Simulate extraction work
        match extract_rules_from_file(&file_path, &event_tx).await {
            Ok(rules) => {
                let _ = event_tx.send(AppEvent::ExtractionComplete {
                    file: file_path,
                    rules,
                });
            }
            Err(e) => {
                let _ = event_tx.send(AppEvent::ExtractionFailed {
                    file: file_path,
                    error: e.to_string(),
                });
            }
        }
    });
}

/// Simulate rule extraction from a file
async fn extract_rules_from_file(
    file_path: &PathBuf,
    event_tx: &tokio::sync::mpsc::UnboundedSender<AppEvent>,
) -> Result<Vec<ExtractedRule>> {
    // Determine extraction time based on file type
    let extraction_time = if file_path.to_string_lossy().contains("CLAUDE") {
        Duration::from_millis(1500) // Larger files take longer
    } else if file_path.is_dir() {
        Duration::from_millis(2000) // Directories take longest
    } else {
        Duration::from_millis(800) // Regular files
    };
    
    // Simple loading - just show we're working
    sleep(extraction_time).await;
    
    // Send a single progress update to show we're still alive
    let _ = event_tx.send(AppEvent::ExtractionProgress {
        file: file_path.clone(),
        progress: 1.0,
    });
    
    // Generate stub rules based on file
    let rules = generate_stub_rules(file_path);
    
    Ok(rules)
}

/// Create a policy decision based on severity and category
fn make_policy_decision(severity: Severity, category: &str) -> PolicyDecision {
    match (severity, category) {
        (Severity::High, _) => PolicyDecision {
            to_policy: true,
            rationale: "High severity rules should always be enforced".to_string(),
        },
        (Severity::Medium, "security" | "testing") => PolicyDecision {
            to_policy: true,
            rationale: "Security and testing rules are critical for code quality".to_string(),
        },
        (Severity::Medium, _) => PolicyDecision {
            to_policy: true,
            rationale: "Recommended for maintaining code standards".to_string(),
        },
        (Severity::Low, "formatting" | "code-style") => PolicyDecision {
            to_policy: false,
            rationale: "Style preferences can be project-specific".to_string(),
        },
        (Severity::Low, _) => PolicyDecision {
            to_policy: false,
            rationale: "Optional - enable based on team preferences".to_string(),
        },
    }
}

/// Generate stub rules for a file
/// 
/// In the real implementation, this would call an LLM to extract rules.
/// For now, we generate plausible stub rules based on the file type.
fn generate_stub_rules(file_path: &PathBuf) -> Vec<ExtractedRule> {
    let file_name = file_path.file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    
    // IDs will be reassigned in compile_rules() anyway
    let base_id = 0;
    
    // Generate different rules based on file type
    if file_name.contains("CLAUDE") || file_name.contains("AGENT") {
        vec![
            ExtractedRule {
                id: base_id,
                source_file: file_path.clone(),
                description: "Always run tests before committing code".to_string(),
                severity: Severity::High,
                category: "testing".to_string(),
                when: "pre-commit".to_string(),
                block_on_violation: true,
                policy_decision: make_policy_decision(Severity::High, "testing"),
            },
            ExtractedRule {
                id: base_id + 1,
                source_file: file_path.clone(),
                description: "Use TypeScript strict mode in all files".to_string(),
                severity: Severity::Medium,
                category: "code-style".to_string(),
                when: "file-change".to_string(),
                block_on_violation: false,
                policy_decision: make_policy_decision(Severity::Medium, "code-style"),
            },
            ExtractedRule {
                id: base_id + 2,
                source_file: file_path.clone(),
                description: "Document all public API functions".to_string(),
                severity: Severity::Low,
                category: "documentation".to_string(),
                when: "file-change".to_string(),
                block_on_violation: false,
                policy_decision: make_policy_decision(Severity::Low, "documentation"),
            },
        ]
    } else if file_name.contains("cursor") || file_name.contains("rules") {
        vec![
            ExtractedRule {
                id: base_id,
                source_file: file_path.clone(),
                description: "No console.log statements in production code".to_string(),
                severity: Severity::High,
                category: "code-quality".to_string(),
                when: "file-change".to_string(),
                block_on_violation: true,
                policy_decision: make_policy_decision(Severity::High, "code-quality"),
            },
            ExtractedRule {
                id: base_id + 1,
                source_file: file_path.clone(),
                description: "Prefer async/await over raw promises".to_string(),
                severity: Severity::Low,
                category: "code-style".to_string(),
                when: "tool-call".to_string(),
                block_on_violation: false,
                policy_decision: make_policy_decision(Severity::Low, "code-style"),
            },
        ]
    } else if file_name.contains("kiro") || file_name.contains("steering") {
        vec![
            ExtractedRule {
                id: base_id,
                source_file: file_path.clone(),
                description: "Require pull request approval before merging".to_string(),
                severity: Severity::High,
                category: "workflow".to_string(),
                when: "pre-commit".to_string(),
                block_on_violation: true,
                policy_decision: make_policy_decision(Severity::High, "workflow"),
            },
            ExtractedRule {
                id: base_id + 1,
                source_file: file_path.clone(),
                description: "All CI tests must pass before merge".to_string(),
                severity: Severity::High,
                category: "testing".to_string(),
                when: "pre-commit".to_string(),
                block_on_violation: true,
                policy_decision: make_policy_decision(Severity::High, "testing"),
            },
            ExtractedRule {
                id: base_id + 2,
                source_file: file_path.clone(),
                description: "Format code with prettier on save".to_string(),
                severity: Severity::Low,
                category: "formatting".to_string(),
                when: "file-change".to_string(),
                block_on_violation: false,
                policy_decision: make_policy_decision(Severity::Low, "formatting"),
            },
        ]
    } else if file_name.contains("copilot") {
        vec![
            ExtractedRule {
                id: base_id,
                source_file: file_path.clone(),
                description: "Follow security best practices for authentication".to_string(),
                severity: Severity::High,
                category: "security".to_string(),
                when: "tool-call".to_string(),
                block_on_violation: false,
                policy_decision: make_policy_decision(Severity::High, "security"),
            },
            ExtractedRule {
                id: base_id + 1,
                source_file: file_path.clone(),
                description: "Use environment variables for sensitive config".to_string(),
                severity: Severity::Medium,
                category: "security".to_string(),
                when: "file-change".to_string(),
                block_on_violation: false,
                policy_decision: make_policy_decision(Severity::Medium, "security"),
            },
        ]
    } else if file_name.contains("GEMINI") {
        vec![
            ExtractedRule {
                id: base_id,
                source_file: file_path.clone(),
                description: "Optimize for mobile-first responsive design".to_string(),
                severity: Severity::Medium,
                category: "ui-ux".to_string(),
                when: "file-change".to_string(),
                block_on_violation: false,
                policy_decision: make_policy_decision(Severity::Medium, "ui-ux"),
            },
            ExtractedRule {
                id: base_id + 1,
                source_file: file_path.clone(),
                description: "Ensure accessibility compliance (WCAG 2.1)".to_string(),
                severity: Severity::High,
                category: "accessibility".to_string(),
                when: "file-change".to_string(),
                block_on_violation: false,
                policy_decision: make_policy_decision(Severity::High, "accessibility"),
            },
        ]
    } else {
        // Default rules for unknown file types
        vec![
            ExtractedRule {
                id: base_id,
                source_file: file_path.clone(),
                description: "Follow project coding standards".to_string(),
                severity: Severity::Low,
                category: "general".to_string(),
                when: "file-change".to_string(),
                block_on_violation: false,
                policy_decision: make_policy_decision(Severity::Low, "general"),
            },
        ]
    }
}

/// Compile and deduplicate rules from all extractions
/// 
/// This takes all extracted rules and:
/// 1. Removes exact duplicates
/// 2. Prioritizes by severity
/// 3. Groups by category
pub fn compile_rules(all_rules: Vec<ExtractedRule>) -> Vec<ExtractedRule> {
    use std::collections::HashSet;
    
    let mut seen_descriptions = HashSet::new();
    let mut unique_rules = Vec::new();
    
    // First pass: collect unique rules
    for rule in all_rules {
        let key = (rule.description.clone(), rule.severity as u8);
        if seen_descriptions.insert(key) {
            unique_rules.push(rule);
        }
    }
    
    // Sort by policy recommendation first, then severity (High first), then by category
    unique_rules.sort_by(|a, b| {
        match b.policy_decision.to_policy.cmp(&a.policy_decision.to_policy) {
            std::cmp::Ordering::Equal => {
                // High=0, Medium=1, Low=2 for proper ordering
                let a_priority = match a.severity {
                    Severity::High => 0,
                    Severity::Medium => 1,
                    Severity::Low => 2,
                };
                let b_priority = match b.severity {
                    Severity::High => 0,
                    Severity::Medium => 1,
                    Severity::Low => 2,
                };
                match a_priority.cmp(&b_priority) {
                    std::cmp::Ordering::Equal => a.category.cmp(&b.category),
                    other => other,
                }
            }
            other => other,
        }
    });
    
    // Re-assign IDs sequentially
    for (idx, rule) in unique_rules.iter_mut().enumerate() {
        rule.id = idx;
    }
    
    unique_rules
}