//! Telemetry output writers.
//!
//! Handles writing TelemetryContext to various destinations:
//! - Debug files (.cupcake/debug/)
//! - Telemetry files (configurable destination)
//! - Future: OTLP export

use anyhow::Result;
use chrono::{DateTime, Local};
use std::fs;
use std::path::Path;

use crate::engine::rulebook::TelemetryFormat;

use super::context::TelemetryContext;

/// Telemetry output writer.
///
/// Currently supports file-based output. Future versions will add OTLP export.
pub struct TelemetryWriter;

impl TelemetryWriter {
    /// Write telemetry to debug file in .cupcake/debug/.
    ///
    /// Output format: Human-readable text for quick debugging.
    pub fn write_debug_file(ctx: &TelemetryContext, debug_dir: &Path) -> Result<()> {
        // Create directory if needed
        if !debug_dir.exists() {
            fs::create_dir_all(debug_dir)?;
        }

        // Generate filename with timestamp and trace_id
        let datetime: DateTime<Local> = ctx.ingest.timestamp.into();
        let filename = format!(
            "{}_{}.txt",
            datetime.format("%Y-%m-%d_%H-%M-%S"),
            ctx.ingest.trace_id
        );

        let file_path = debug_dir.join(filename);
        let content = Self::format_human_readable(ctx)?;

        fs::write(file_path, content)?;
        Ok(())
    }

    /// Write telemetry to configured destination.
    ///
    /// Output format depends on TelemetryConfig.format (json or text).
    pub fn write_telemetry(
        ctx: &TelemetryContext,
        format: &TelemetryFormat,
        destination: &Path,
    ) -> Result<()> {
        // Create directory if needed
        if !destination.exists() {
            fs::create_dir_all(destination)?;
        }

        // Generate filename
        let datetime: DateTime<Local> = ctx.ingest.timestamp.into();
        let extension = match format {
            TelemetryFormat::Json => "json",
            TelemetryFormat::Text => "txt",
        };
        let filename = format!(
            "{}_{}.{}",
            datetime.format("%Y-%m-%d_%H-%M-%S"),
            ctx.ingest.trace_id,
            extension
        );

        let file_path = destination.join(filename);

        // Format based on type (add newline for log parsers like Promtail)
        let content = match format {
            TelemetryFormat::Json => format!("{}\n", serde_json::to_string(ctx)?),
            TelemetryFormat::Text => Self::format_human_readable(ctx)?,
        };

        fs::write(file_path, content)?;
        Ok(())
    }

    /// Format telemetry as human-readable text.
    fn format_human_readable(ctx: &TelemetryContext) -> Result<String> {
        let datetime: DateTime<Local> = ctx.ingest.timestamp.into();
        let start_time = datetime.format("%Y-%m-%d %H:%M:%S").to_string();

        let mut output = String::new();

        // Header
        output.push_str(&format!(
            "===== Cupcake Telemetry [{}] [{}] =====\n",
            start_time, ctx.ingest.trace_id
        ));
        output.push_str(&format!("Harness: {:?}\n", ctx.ingest.harness));
        output.push_str(&format!("Total Duration: {}ms\n", ctx.total_duration_ms));
        output.push('\n');

        // Ingest Stage
        output.push_str("----- STAGE: Ingest (Raw Event) -----\n");
        output.push_str(&serde_json::to_string_pretty(&ctx.ingest.raw_event)?);
        output.push_str("\n\n");

        // Enrich Stage
        output.push_str("----- STAGE: Enrich (Preprocessed) -----\n");
        if let Some(ref enrich) = ctx.enrich {
            output.push_str(&format!(
                "Operations: {}\n",
                enrich.preprocessing_operations.join(", ")
            ));
            output.push_str(&format!("Duration: {}μs\n", enrich.duration_us));
            output.push_str("Enriched Event:\n");
            output.push_str(&serde_json::to_string_pretty(&enrich.enriched_event)?);
            output.push_str("\n\n");
        } else {
            output.push_str("(No enrichment recorded)\n\n");
        }

        // Evaluate Stages
        output.push_str("----- STAGE: Evaluate (Policy Evaluation) -----\n");
        if ctx.evaluations.is_empty() {
            output.push_str("(No evaluation performed - early exit before engine)\n\n");
        } else {
            for (i, eval) in ctx.evaluations.iter().enumerate() {
                output.push_str(&format!("\n[Phase {}: {}]\n", i + 1, eval.phase));
                output.push_str(&format!("  Routed: {}\n", eval.routed));

                if !eval.matched_policies.is_empty() {
                    output.push_str(&format!(
                        "  Matched Policies: {}\n",
                        eval.matched_policies.join(", ")
                    ));
                }

                if let Some(ref exit_reason) = eval.exit_reason {
                    output.push_str(&format!("  Exit Reason: {exit_reason}\n"));
                }

                if let Some(ref decision_set) = eval.wasm_decision_set {
                    output.push_str("  WASM Decision Set:\n");
                    output.push_str(&format!("    Halts: {}\n", decision_set.halts.len()));
                    output.push_str(&format!("    Denials: {}\n", decision_set.denials.len()));
                    output.push_str(&format!("    Blocks: {}\n", decision_set.blocks.len()));
                    output.push_str(&format!("    Asks: {}\n", decision_set.asks.len()));
                    output.push_str(&format!(
                        "    Modifications: {}\n",
                        decision_set.modifications.len()
                    ));
                    output.push_str(&format!(
                        "    Context: {}\n",
                        decision_set.add_context.len()
                    ));

                    // Show individual decisions if any
                    for halt in &decision_set.halts {
                        output.push_str(&format!(
                            "      - [HALT] {}: {} ({})\n",
                            halt.rule_id, halt.reason, halt.severity
                        ));
                    }
                    for denial in &decision_set.denials {
                        output.push_str(&format!(
                            "      - [DENY] {}: {} ({})\n",
                            denial.rule_id, denial.reason, denial.severity
                        ));
                    }
                    for block in &decision_set.blocks {
                        output.push_str(&format!(
                            "      - [BLOCK] {}: {} ({})\n",
                            block.rule_id, block.reason, block.severity
                        ));
                    }
                    for ask in &decision_set.asks {
                        output.push_str(&format!(
                            "      - [ASK] {}: {} ({})\n",
                            ask.rule_id, ask.reason, ask.severity
                        ));
                    }
                }

                if let Some(ref final_decision) = eval.final_decision {
                    output.push_str(&format!("  Final Decision: {final_decision:?}\n"));
                }

                if !eval.signals_executed.is_empty() {
                    output.push_str(&format!(
                        "  Signals Executed: {}\n",
                        eval.signals_executed.len()
                    ));
                    for signal in &eval.signals_executed {
                        output.push_str(&format!("    - {}: {}\n", signal.name, signal.command));
                    }
                }

                output.push_str(&format!("  Duration: {}ms\n", eval.duration_ms));
            }
            output.push('\n');
        }

        // Response
        output.push_str("----- Response to Agent -----\n");
        if let Some(ref response) = ctx.response_to_agent {
            output.push_str(&serde_json::to_string_pretty(response)?);
            output.push('\n');
        } else {
            output.push_str("(No response recorded)\n");
        }
        output.push('\n');

        // Errors
        if !ctx.errors.is_empty() {
            output.push_str("----- Errors -----\n");
            for (i, error) in ctx.errors.iter().enumerate() {
                output.push_str(&format!("{}. {}\n", i + 1, error));
            }
            output.push('\n');
        }

        // Footer
        output.push_str(&format!(
            "===== End Telemetry [{}ms] =====\n",
            ctx.total_duration_ms
        ));

        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::harness::types::HarnessType;
    use serde_json::json;
    use tempfile::tempdir;

    #[test]
    fn test_write_debug_file() {
        let raw = json!({"hook_event_name": "PreToolUse", "tool_name": "Bash"});
        let mut ctx = TelemetryContext::new(raw, HarnessType::ClaudeCode, "test-trace".into());
        ctx.record_enrichment(
            json!({"enriched": true}),
            vec!["whitespace_normalization".into()],
            100,
        );

        let dir = tempdir().unwrap();
        TelemetryWriter::write_debug_file(&ctx, dir.path()).unwrap();

        // Verify file was created
        let files: Vec<_> = fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .collect();
        assert_eq!(files.len(), 1);

        // Verify content
        let content = fs::read_to_string(files[0].path()).unwrap();
        assert!(content.contains("Cupcake Telemetry"));
        assert!(content.contains("test-trace"));
        assert!(content.contains("PreToolUse"));
    }

    #[test]
    fn test_write_telemetry_json() {
        let raw = json!({"test": true});
        let ctx = TelemetryContext::new(raw, HarnessType::ClaudeCode, "json-trace".into());

        let dir = tempdir().unwrap();
        TelemetryWriter::write_telemetry(&ctx, &TelemetryFormat::Json, dir.path()).unwrap();

        let files: Vec<_> = fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map(|s| s == "json").unwrap_or(false))
            .collect();
        assert_eq!(files.len(), 1);

        // Verify it's valid JSON
        let content = fs::read_to_string(files[0].path()).unwrap();
        let _: serde_json::Value = serde_json::from_str(&content).unwrap();
    }

    #[test]
    fn test_human_readable_format() {
        let raw = json!({"hook_event_name": "PreToolUse"});
        let mut ctx = TelemetryContext::new(raw, HarnessType::ClaudeCode, "readable-trace".into());

        // Add enrichment
        ctx.record_enrichment(
            json!({"resolved": true}),
            vec!["symlink_resolution".into()],
            50,
        );

        // Add evaluation
        let eval = ctx.start_evaluation("project");
        eval.record_routing(false, &[]);
        eval.record_exit("No policies matched - implicit allow");

        ctx.add_error("Test error for formatting");

        let output = TelemetryWriter::format_human_readable(&ctx).unwrap();

        // Verify sections present
        assert!(output.contains("STAGE: Ingest"));
        assert!(output.contains("STAGE: Enrich"));
        assert!(output.contains("STAGE: Evaluate"));
        assert!(output.contains("Response to Agent"));
        assert!(output.contains("Errors"));
        assert!(output.contains("No policies matched"));
    }
}
