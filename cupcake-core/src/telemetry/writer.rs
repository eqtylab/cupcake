//! Telemetry output writers.
//!
//! Handles writing CupcakeSpan to various destinations:
//! - Debug files (.cupcake/debug/)
//! - Telemetry files (configurable destination)

use anyhow::Result;
use chrono::{DateTime, Local};
use std::fs;
use std::path::Path;

use crate::engine::rulebook::TelemetryFormat;

use super::span::CupcakeSpan;

/// Telemetry output writer.
pub struct TelemetryWriter;

impl TelemetryWriter {
    /// Write telemetry to debug file in .cupcake/debug/.
    pub fn write_debug_file(span: &CupcakeSpan, debug_dir: &Path) -> Result<()> {
        if !debug_dir.exists() {
            fs::create_dir_all(debug_dir)?;
        }

        let datetime: DateTime<Local> = span.timestamp.into();
        let filename = format!(
            "{}_{}.txt",
            datetime.format("%Y-%m-%d_%H-%M-%S"),
            span.trace_id
        );

        let file_path = debug_dir.join(filename);
        let content = Self::format_human_readable(span)?;

        fs::write(file_path, content)?;
        Ok(())
    }

    /// Write telemetry to configured destination.
    pub fn write_telemetry(
        span: &CupcakeSpan,
        format: &TelemetryFormat,
        destination: &Path,
    ) -> Result<()> {
        if !destination.exists() {
            fs::create_dir_all(destination)?;
        }

        let datetime: DateTime<Local> = span.timestamp.into();
        let extension = match format {
            TelemetryFormat::Json => "json",
            TelemetryFormat::Text => "txt",
        };
        let filename = format!(
            "{}_{}.{}",
            datetime.format("%Y-%m-%d_%H-%M-%S"),
            span.trace_id,
            extension
        );

        let file_path = destination.join(filename);

        let content = match format {
            TelemetryFormat::Json => format!("{}\n", serde_json::to_string(span)?),
            TelemetryFormat::Text => Self::format_human_readable(span)?,
        };

        fs::write(file_path, content)?;
        Ok(())
    }

    /// Format telemetry as human-readable text.
    fn format_human_readable(span: &CupcakeSpan) -> Result<String> {
        let datetime: DateTime<Local> = span.timestamp.into();
        let start_time = datetime.format("%Y-%m-%d %H:%M:%S").to_string();

        let mut out = String::new();

        // Header
        out.push_str(&format!(
            "===== Cupcake Trace [{}] [{}] =====\n",
            start_time, span.trace_id
        ));
        out.push_str(&format!("Harness: {:?}\n", span.harness));
        out.push_str(&format!("Duration: {}ms\n\n", span.total_duration_ms));

        // Raw Event
        out.push_str("----- RAW EVENT -----\n");
        out.push_str(&serde_json::to_string_pretty(&span.raw_event)?);
        out.push_str("\n\n");

        // Enrich Phase
        out.push_str("----- ENRICH -----\n");
        if let Some(ref enrich) = span.enrich {
            out.push_str(&format!("Operations: {}\n", enrich.operations.join(", ")));
            out.push_str(&format!("Duration: {}μs\n", enrich.duration_us));
            out.push_str("Enriched:\n");
            out.push_str(&serde_json::to_string_pretty(&enrich.enriched_event)?);
            out.push_str("\n\n");
        } else {
            out.push_str("(not recorded)\n\n");
        }

        // Policy Phases
        if span.phases.is_empty() {
            out.push_str("----- POLICY PHASES -----\n");
            out.push_str("(no phases - early exit before evaluation)\n\n");
        } else {
            for (i, phase) in span.phases.iter().enumerate() {
                out.push_str(&format!(
                    "----- PHASE {}: {} ({}) -----\n",
                    i + 1,
                    phase.name.to_uppercase(),
                    phase.span_id
                ));

                // Signals
                if let Some(ref signals) = phase.signals {
                    out.push_str(&format!(
                        "Signals: {} collected in {}ms\n",
                        signals.signals.len(),
                        signals.duration_ms
                    ));
                    for sig in &signals.signals {
                        out.push_str(&format!("  - {}: {}", sig.name, sig.command));
                        if let Some(ms) = sig.duration_ms {
                            out.push_str(&format!(" ({ms}ms)"));
                        }
                        if let Some(code) = sig.exit_code {
                            out.push_str(&format!(" [exit {code}]"));
                        }
                        out.push('\n');
                        // Show result if small
                        let result_str = serde_json::to_string(&sig.result)?;
                        if result_str.len() < 200 {
                            out.push_str(&format!("    → {result_str}\n"));
                        }
                    }
                } else {
                    out.push_str("Signals: none\n");
                }

                // Evaluation
                let eval = &phase.evaluation;
                out.push_str(&format!("Routed: {}\n", eval.routed));

                if !eval.matched_policies.is_empty() {
                    out.push_str(&format!("Matched: {}\n", eval.matched_policies.join(", ")));
                }

                if let Some(ref exit) = eval.exit_reason {
                    out.push_str(&format!("Exit: {exit}\n"));
                }

                if let Some(ref ds) = eval.wasm_decision_set {
                    out.push_str("WASM Results:\n");
                    if !ds.halts.is_empty() {
                        out.push_str(&format!("  Halts: {}\n", ds.halts.len()));
                        for h in &ds.halts {
                            out.push_str(&format!(
                                "    [{}] {} ({})\n",
                                h.rule_id, h.reason, h.severity
                            ));
                        }
                    }
                    if !ds.denials.is_empty() {
                        out.push_str(&format!("  Denials: {}\n", ds.denials.len()));
                        for d in &ds.denials {
                            out.push_str(&format!(
                                "    [{}] {} ({})\n",
                                d.rule_id, d.reason, d.severity
                            ));
                        }
                    }
                    if !ds.blocks.is_empty() {
                        out.push_str(&format!("  Blocks: {}\n", ds.blocks.len()));
                        for b in &ds.blocks {
                            out.push_str(&format!(
                                "    [{}] {} ({})\n",
                                b.rule_id, b.reason, b.severity
                            ));
                        }
                    }
                    if !ds.asks.is_empty() {
                        out.push_str(&format!("  Asks: {}\n", ds.asks.len()));
                        for a in &ds.asks {
                            out.push_str(&format!(
                                "    [{}] {} ({})\n",
                                a.rule_id, a.reason, a.severity
                            ));
                        }
                    }
                    if !ds.modifications.is_empty() {
                        out.push_str(&format!("  Modifications: {}\n", ds.modifications.len()));
                    }
                    if !ds.add_context.is_empty() {
                        out.push_str(&format!("  Context: {}\n", ds.add_context.len()));
                    }
                }

                if let Some(ref decision) = eval.final_decision {
                    out.push_str(&format!("Decision: {decision:?}\n"));
                }

                out.push_str(&format!("Phase Duration: {}ms\n\n", phase.duration_ms));
            }
        }

        // Response
        out.push_str("----- RESPONSE -----\n");
        if let Some(ref response) = span.response {
            out.push_str(&serde_json::to_string_pretty(response)?);
            out.push('\n');
        } else {
            out.push_str("(none)\n");
        }
        out.push('\n');

        // Errors
        if !span.errors.is_empty() {
            out.push_str("----- ERRORS -----\n");
            for (i, err) in span.errors.iter().enumerate() {
                out.push_str(&format!("{}. {}\n", i + 1, err));
            }
            out.push('\n');
        }

        out.push_str(&format!("===== End [{}ms] =====\n", span.total_duration_ms));

        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::harness::types::HarnessType;
    use crate::telemetry::{EnrichPhase, PolicyPhase, SignalExecution, SignalsPhase};
    use serde_json::json;
    use tempfile::tempdir;

    fn make_test_span() -> CupcakeSpan {
        let mut span = CupcakeSpan::new(
            json!({"hook_event_name": "PreToolUse", "tool_name": "Bash"}),
            HarnessType::ClaudeCode,
            "test-trace".into(),
        );

        span.set_enrich(EnrichPhase::new(
            json!({"enriched": true}),
            vec!["whitespace_normalization".into()],
            100,
            span.span_id().to_string(),
            span.start_time_unix_nano,
        ));

        let mut phase = PolicyPhase::new("project", span.span_id().to_string());

        let mut signals =
            SignalsPhase::new(phase.span_id().to_string(), phase.start_time_unix_nano);
        signals.add_signal(SignalExecution {
            name: "git_status".into(),
            command: "git status --porcelain".into(),
            result: json!(["M src/main.rs"]),
            duration_ms: Some(25),
            exit_code: Some(0),
        });
        signals.finalize(25);
        phase.set_signals(signals);

        phase.evaluation_mut().record_routing(false, &[]);
        phase.evaluation_mut().record_exit("No policies matched");
        phase.finalize();

        span.add_phase(phase);
        span.finalize(Some(json!({"decision": "allow"})));

        span
    }

    #[test]
    fn test_write_debug_file() {
        let span = make_test_span();
        let dir = tempdir().unwrap();

        TelemetryWriter::write_debug_file(&span, dir.path()).unwrap();

        let files: Vec<_> = fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .collect();
        assert_eq!(files.len(), 1);

        let content = fs::read_to_string(files[0].path()).unwrap();
        assert!(content.contains("Cupcake Trace"));
        assert!(content.contains("test-trace"));
        assert!(content.contains("git_status"));
    }

    #[test]
    fn test_write_telemetry_json() {
        let span = make_test_span();
        let dir = tempdir().unwrap();

        TelemetryWriter::write_telemetry(&span, &TelemetryFormat::Json, dir.path()).unwrap();

        let files: Vec<_> = fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map(|s| s == "json").unwrap_or(false))
            .collect();
        assert_eq!(files.len(), 1);

        let content = fs::read_to_string(files[0].path()).unwrap();
        let _: serde_json::Value = serde_json::from_str(&content).unwrap();
    }

    #[test]
    fn test_human_readable_format() {
        let span = make_test_span();
        let output = TelemetryWriter::format_human_readable(&span).unwrap();

        assert!(output.contains("RAW EVENT"));
        assert!(output.contains("ENRICH"));
        assert!(output.contains("PHASE 1: PROJECT"));
        assert!(output.contains("Signals: 1 collected"));
        assert!(output.contains("git_status"));
        assert!(output.contains("RESPONSE"));
    }
}
