//! Debug Logging System - Complete Event Lifecycle Capture
//!
//! Provides comprehensive debug logging to capture the complete lifecycle of every
//! Claude Code event through the Cupcake policy engine, regardless of whether
//! policies match or actions are taken.
//!
//! Only enabled via --debug-files CLI flag for zero production impact.

use anyhow::Result;
use chrono::{DateTime, Local};
use serde_json::Value;
use std::fs;
use std::path::Path;
use std::time::SystemTime;
use tracing::warn;

use crate::engine::decision::{DecisionSet, FinalDecision};

/// Central debug capture structure that accumulates state throughout evaluation
#[derive(Debug, Clone)]
pub struct DebugCapture {
    /// Whether debug file writing is enabled (from CLI flag)
    pub enabled: bool,

    /// Raw Claude Code event as received
    pub event_received: Value,

    /// Unique identifier for this evaluation
    pub trace_id: String,

    /// When the event was received
    pub timestamp: SystemTime,

    /// Debug output directory (defaults to .cupcake/debug if not specified)
    pub debug_dir: Option<std::path::PathBuf>,

    /// Did we find matching policies?
    pub routed: bool,

    /// Which policies matched during routing
    pub matched_policies: Vec<String>,

    /// What signals were needed for evaluation
    pub signals_configured: Vec<String>,

    /// Signal execution results
    pub signals_executed: Vec<SignalExecution>,

    /// Raw WASM output from policy evaluation
    pub wasm_decision_set: Option<DecisionSet>,

    /// Synthesized decision from the Intelligence Layer
    pub final_decision: Option<FinalDecision>,

    /// What we sent back to Claude Code
    pub response_to_claude: Option<Value>,

    /// What actions were configured to run
    pub actions_configured: Vec<String>,

    /// Action execution results
    pub actions_executed: Vec<ActionExecution>,

    /// Any errors encountered during evaluation
    pub errors: Vec<String>,
}

/// Signal execution details
#[derive(Debug, Clone)]
pub struct SignalExecution {
    /// Name of the signal
    pub name: String,

    /// Command that was executed
    pub command: String,

    /// Result of the signal execution
    pub result: Value,

    /// Duration in milliseconds (captured for display)
    pub duration_ms: Option<u128>,
}

/// Action execution details
#[derive(Debug, Clone)]
pub struct ActionExecution {
    /// Name of the action
    pub name: String,

    /// Command that was executed
    pub command: String,

    /// Duration and exit code (if captured)
    pub duration_ms: Option<u128>,
    pub exit_code: Option<i32>,
}

impl DebugCapture {
    /// Create a new debug capture for an event
    ///
    /// `enabled`: Whether debug file writing is enabled (from --debug-files CLI flag)
    /// `debug_dir`: Optional override for debug output directory (defaults to .cupcake/debug)
    pub fn new(
        event: Value,
        trace_id: String,
        enabled: bool,
        debug_dir: Option<std::path::PathBuf>,
    ) -> Self {
        Self {
            enabled,
            event_received: event,
            trace_id,
            timestamp: SystemTime::now(),
            debug_dir,
            routed: false,
            matched_policies: Vec::new(),
            signals_configured: Vec::new(),
            signals_executed: Vec::new(),
            wasm_decision_set: None,
            final_decision: None,
            response_to_claude: None,
            actions_configured: Vec::new(),
            actions_executed: Vec::new(),
            errors: Vec::new(),
        }
    }

    /// Add an error to the capture
    pub fn add_error(&mut self, error: String) {
        self.errors.push(error);
    }

    /// Write the debug capture to a file if enabled
    ///
    /// Only writes if the `enabled` flag (from --debug-files CLI) is true.
    /// This ensures zero overhead when debug is disabled.
    pub fn write_if_enabled(&self) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }

        if let Err(e) = self.write_debug_file() {
            warn!("Failed to write debug file: {}", e);
        }
        Ok(())
    }

    /// Write the debug capture to a file
    fn write_debug_file(&self) -> Result<()> {
        // Use provided debug_dir or default to .cupcake/debug
        let debug_dir = self
            .debug_dir
            .as_deref()
            .unwrap_or_else(|| Path::new(".cupcake/debug"));

        // Create debug directory if it doesn't exist
        if !debug_dir.exists() {
            fs::create_dir_all(debug_dir)?;
        }

        // Generate filename with timestamp and trace_id
        let datetime: DateTime<Local> = self.timestamp.into();
        let filename = format!(
            "{}_{}.txt",
            datetime.format("%Y-%m-%d_%H-%M-%S"),
            self.trace_id
        );

        let file_path = debug_dir.join(filename);

        // Format the debug output
        let content = self.format_debug_output()?;

        // Write to file
        fs::write(file_path, content)?;

        Ok(())
    }

    /// Format the debug capture as human-readable text
    fn format_debug_output(&self) -> Result<String> {
        let datetime: DateTime<Local> = self.timestamp.into();
        let start_time = datetime.format("%Y-%m-%d %H:%M:%S").to_string();

        let mut output = String::new();

        // Header
        output.push_str(&format!(
            "===== Claude Code Event [{}] [{}] =====\n",
            start_time, self.trace_id
        ));

        // Extract event type and tool info from the event
        if let Some(event_name) = self
            .event_received
            .get("hook_event_name")
            .and_then(|v| v.as_str())
        {
            output.push_str(&format!("Event Type: {event_name}\n"));

            if let Some(tool_name) = self
                .event_received
                .get("tool_name")
                .and_then(|v| v.as_str())
            {
                output.push_str(&format!("Tool: {tool_name}\n"));
            }

            if let Some(session_id) = self
                .event_received
                .get("session_id")
                .and_then(|v| v.as_str())
            {
                output.push_str(&format!("Session ID: {session_id}\n"));
            }
        }

        output.push('\n');

        // Raw Event
        output.push_str("Raw Event:\n");
        output.push_str(&serde_json::to_string_pretty(&self.event_received)?);
        output.push_str("\n\n");

        // Routing
        output.push_str("----- Routing -----\n");
        if self.routed && !self.matched_policies.is_empty() {
            output.push_str(&format!(
                "Matched: Yes ({} policies)\n",
                self.matched_policies.len()
            ));
            for policy in &self.matched_policies {
                output.push_str(&format!("- {policy}\n"));
            }
        } else if self.routed {
            output.push_str("Matched: Yes (no specific policies captured)\n");
        } else {
            output.push_str("Matched: No\n");
        }
        output.push('\n');

        // Signals
        output.push_str("----- Signals -----\n");
        if !self.signals_configured.is_empty() {
            output.push_str(&format!(
                "Configured: {} signals\n",
                self.signals_configured.len()
            ));
            for signal in &self.signals_configured {
                output.push_str(&format!("- {signal}\n"));
            }
            output.push('\n');

            if !self.signals_executed.is_empty() {
                output.push_str("Executed:\n");
                for signal in &self.signals_executed {
                    output.push_str(&format!("[{}]\n", signal.name));
                    output.push_str(&format!("  Command: {}\n", signal.command));
                    if let Some(duration) = signal.duration_ms {
                        output.push_str(&format!("  Duration: {duration}ms\n"));
                    }
                    output.push_str(&format!(
                        "  Result: {}\n",
                        serde_json::to_string(&signal.result)?
                    ));
                }
            } else {
                output.push_str("Executed: None\n");
            }
        } else {
            output.push_str("Configured: None\n");
        }
        output.push('\n');

        // WASM Evaluation
        output.push_str("----- WASM Evaluation -----\n");
        if let Some(ref decision_set) = self.wasm_decision_set {
            output.push_str("Decision Set:\n");
            output.push_str(&format!("  Halts: {}\n", decision_set.halts.len()));
            for halt in &decision_set.halts {
                output.push_str(&format!(
                    "    - [{}] {} ({})\n",
                    halt.rule_id, halt.reason, halt.severity
                ));
            }

            output.push_str(&format!("  Denials: {}\n", decision_set.denials.len()));
            for denial in &decision_set.denials {
                output.push_str(&format!(
                    "    - [{}] {} ({})\n",
                    denial.rule_id, denial.reason, denial.severity
                ));
            }

            output.push_str(&format!("  Blocks: {}\n", decision_set.blocks.len()));
            for block in &decision_set.blocks {
                output.push_str(&format!(
                    "    - [{}] {} ({})\n",
                    block.rule_id, block.reason, block.severity
                ));
            }

            output.push_str(&format!("  Asks: {}\n", decision_set.asks.len()));
            for ask in &decision_set.asks {
                output.push_str(&format!(
                    "    - [{}] {} ({})\n",
                    ask.rule_id, ask.reason, ask.severity
                ));
            }

            output.push_str(&format!(
                "  Allow Overrides: {}\n",
                decision_set.allow_overrides.len()
            ));
            for allow in &decision_set.allow_overrides {
                output.push_str(&format!(
                    "    - [{}] {} ({})\n",
                    allow.rule_id, allow.reason, allow.severity
                ));
            }

            output.push_str(&format!("  Context: {}\n", decision_set.add_context.len()));
            for context in &decision_set.add_context {
                output.push_str(&format!("    - {context}\n"));
            }
        } else {
            output.push_str("No WASM evaluation performed\n");
        }
        output.push('\n');

        // Synthesis
        output.push_str("----- Synthesis -----\n");
        if let Some(ref final_decision) = self.final_decision {
            match final_decision {
                FinalDecision::Halt { reason, .. } => {
                    output.push_str(&format!("Final Decision: Halt\nReason: {reason}\n"));
                }
                FinalDecision::Deny { reason, .. } => {
                    output.push_str(&format!("Final Decision: Deny\nReason: {reason}\n"));
                }
                FinalDecision::Block { reason, .. } => {
                    output.push_str(&format!("Final Decision: Block\nReason: {reason}\n"));
                }
                FinalDecision::Ask { reason, .. } => {
                    output.push_str(&format!("Final Decision: Ask\nReason: {reason}\n"));
                }
                FinalDecision::AllowOverride { reason, .. } => {
                    output.push_str(&format!(
                        "Final Decision: AllowOverride\nReason: {reason}\n"
                    ));
                }
                FinalDecision::Allow { context } => {
                    output.push_str("Final Decision: Allow\n");
                    if !context.is_empty() {
                        output.push_str("Context:\n");
                        for ctx in context {
                            output.push_str(&format!("  - {ctx}\n"));
                        }
                    }
                }
            }
        } else {
            output.push_str("No synthesis performed\n");
        }
        output.push('\n');

        // Response to Claude
        output.push_str("----- Response to Claude -----\n");
        if let Some(ref response) = self.response_to_claude {
            output.push_str(&serde_json::to_string_pretty(response)?);
            output.push('\n');
        } else {
            output.push_str("No response generated\n");
        }
        output.push('\n');

        // Actions
        output.push_str("----- Actions -----\n");
        if !self.actions_configured.is_empty() {
            output.push_str(&format!(
                "Configured: {} action(s)\n",
                self.actions_configured.len()
            ));
            for action in &self.actions_configured {
                output.push_str(&format!("- {action}\n"));
            }

            if !self.actions_executed.is_empty() {
                output.push_str("Executed:\n");
                for action in &self.actions_executed {
                    output.push_str(&format!("[{}]\n", action.name));
                    output.push_str(&format!("  Command: {}\n", action.command));
                    if let Some(duration) = action.duration_ms {
                        output.push_str(&format!("  Duration: {duration}ms\n"));
                    }
                    if let Some(exit_code) = action.exit_code {
                        output.push_str(&format!("  Exit Code: {exit_code}\n"));
                    }
                }
            } else {
                output.push_str("Executed: None\n");
            }
        } else {
            output.push_str("Configured: None\n");
        }
        output.push('\n');

        // Errors
        if !self.errors.is_empty() {
            output.push_str("----- Errors -----\n");
            for (i, error) in self.errors.iter().enumerate() {
                output.push_str(&format!("{}. {}\n", i + 1, error));
            }
            output.push('\n');
        }

        // Footer with duration
        let end_time = datetime.format("%H:%M:%S%.3f").to_string();
        let elapsed = SystemTime::now()
            .duration_since(self.timestamp)
            .map(|d| format!(" Duration: {}ms", d.as_millis()))
            .unwrap_or_default();
        output.push_str(&format!("===== End Event [{end_time}]{elapsed} =====\n"));

        Ok(output)
    }
}

// Include test modules
#[cfg(test)]
mod tests;

#[cfg(test)]
mod unit_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_debug_capture_creation() {
        let event = json!({
            "hook_event_name": "PreToolUse",
            "tool_name": "Bash",
            "session_id": "test-session"
        });

        let capture = DebugCapture::new(event.clone(), "test-trace-123".to_string(), true, None);

        assert!(capture.enabled);
        assert_eq!(capture.trace_id, "test-trace-123");
        assert_eq!(capture.event_received, event);
        assert!(!capture.routed);
        assert!(capture.matched_policies.is_empty());
        assert!(capture.errors.is_empty());
    }

    #[test]
    fn test_add_error() {
        let event = json!({});
        let mut capture = DebugCapture::new(event, "trace-id".to_string(), false, None);

        capture.add_error("Test error".to_string());
        assert_eq!(capture.errors.len(), 1);
        assert_eq!(capture.errors[0], "Test error");
    }

    #[test]
    fn test_format_debug_output() {
        let event = json!({
            "hook_event_name": "PreToolUse",
            "tool_name": "Bash",
            "session_id": "test-session"
        });

        let mut capture = DebugCapture::new(event, "trace-123".to_string(), true, None);
        capture.routed = true;
        capture.matched_policies.push("test.policy".to_string());
        capture.add_error("Test error".to_string());

        let output = capture.format_debug_output().unwrap();

        assert!(output.contains("===== Claude Code Event"));
        assert!(output.contains("[trace-123]"));
        assert!(output.contains("Event Type: PreToolUse"));
        assert!(output.contains("Tool: Bash"));
        assert!(output.contains("Matched: Yes (1 policies)"));
        assert!(output.contains("- test.policy"));
        assert!(output.contains("----- Errors -----"));
        assert!(output.contains("1. Test error"));
        assert!(output.contains("===== End Event"));
    }

    #[test]
    fn test_write_if_enabled_writes_file() {
        // Test that write_if_enabled actually writes the file when enabled=true

        let event = json!({
            "hook_event_name": "PreToolUse",
            "tool_name": "Bash"
        });
        let capture = DebugCapture::new(event, "test_trace".to_string(), true, None);

        // Should write and not error
        let result = capture.write_if_enabled();
        assert!(result.is_ok());

        // Verify file was created
        let debug_dir = Path::new(".cupcake/debug");
        if debug_dir.exists() {
            // File should exist with trace_id in name
            let files: Vec<_> = std::fs::read_dir(debug_dir)
                .unwrap()
                .filter_map(|e| e.ok())
                .filter(|e| {
                    e.file_name()
                        .to_str()
                        .map(|s| s.contains("test_trace"))
                        .unwrap_or(false)
                })
                .collect();
            assert!(!files.is_empty(), "Debug file should have been created");

            // Clean up
            for file in files {
                let _ = std::fs::remove_file(file.path());
            }
        }
    }
}
