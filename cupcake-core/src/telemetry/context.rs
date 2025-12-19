//! TelemetryContext - orchestrates telemetry capture and output.
//!
//! Wraps CupcakeSpan and handles configuration, finalization, and writing.
//! Implements Drop to ensure telemetry is written even on panic/early return.

use anyhow::Result;
use serde_json::Value;
use std::path::PathBuf;
use tracing::{debug, warn};

use crate::engine::rulebook::TelemetryConfig;
use crate::harness::types::HarnessType;

use super::span::{CupcakeSpan, EnrichPhase, PolicyPhase, SignalsPhase};
use super::writer::TelemetryWriter;

/// Central telemetry context that flows through the entire evaluation pipeline.
///
/// Created in the CLI immediately after parsing stdin (before preprocessing),
/// ensuring we capture the raw event even if processing fails early.
///
/// ## Lifecycle
///
/// 1. Created with raw event in CLI (`TelemetryContext::new()`)
/// 2. Enrichment recorded after preprocessing (`record_enrichment()`)
/// 3. Policy phases started/finalized in Engine (`start_phase()`, `finalize_phase()`)
/// 4. Written via `finalize()` or `Drop` impl
///
/// ## Drop Guard
///
/// Implements `Drop` to ensure telemetry is written even if:
/// - The process panics
/// - An error causes early return via `?`
/// - Any unexpected exit path
#[derive(Debug)]
pub struct TelemetryContext {
    /// The root span containing all telemetry data
    pub span: CupcakeSpan,

    // --- Configuration (not in span) ---
    /// Whether debug file writing is enabled (--debug-files flag)
    debug_files_enabled: bool,

    /// Override directory for debug files
    debug_dir: Option<PathBuf>,

    /// Telemetry configuration from rulebook
    telemetry_config: Option<TelemetryConfig>,

    /// Whether finalize() has been called (prevents double-write in Drop)
    is_finalized: bool,
}

impl TelemetryContext {
    /// Create a new telemetry context capturing the raw event.
    ///
    /// Call IMMEDIATELY after parsing stdin JSON, BEFORE any preprocessing.
    pub fn new(raw_event: Value, harness: HarnessType, trace_id: String) -> Self {
        Self {
            span: CupcakeSpan::new(raw_event, harness, trace_id),
            debug_files_enabled: false,
            debug_dir: None,
            telemetry_config: None,
            is_finalized: false,
        }
    }

    /// Configure output destinations.
    pub fn configure(
        &mut self,
        debug_files_enabled: bool,
        debug_dir: Option<PathBuf>,
        telemetry_config: Option<TelemetryConfig>,
    ) {
        self.debug_files_enabled = debug_files_enabled;
        self.debug_dir = debug_dir;
        self.telemetry_config = telemetry_config;
    }

    /// Record the enrichment/preprocessing phase.
    pub fn record_enrichment(
        &mut self,
        enriched_event: Value,
        operations: Vec<String>,
        duration_us: u64,
    ) {
        let enrich = EnrichPhase::new(
            enriched_event,
            operations,
            duration_us,
            self.span.span_id().to_string(),
            self.span.start_time_unix_nano,
        );
        self.span.set_enrich(enrich);
    }

    /// Start a new policy evaluation phase.
    ///
    /// Returns a mutable reference to the phase for recording signals and evaluation.
    pub fn start_phase(&mut self, name: impl Into<String>) -> &mut PolicyPhase {
        let phase = PolicyPhase::new(name, self.span.span_id().to_string());
        self.span.add_phase(phase);
        self.span.current_phase_mut().unwrap()
    }

    /// Get mutable reference to the current (last) policy phase.
    pub fn current_phase_mut(&mut self) -> Option<&mut PolicyPhase> {
        self.span.current_phase_mut()
    }

    /// Start signals collection for the current phase.
    ///
    /// Returns None if no phase is active.
    pub fn start_signals(&mut self) -> Option<&mut SignalsPhase> {
        if let Some(phase) = self.span.current_phase_mut() {
            let signals =
                SignalsPhase::new(phase.span_id().to_string(), phase.start_time_unix_nano);
            phase.set_signals(signals);
            phase.signals.as_mut()
        } else {
            None
        }
    }

    /// Get mutable reference to current phase's signals.
    pub fn current_signals_mut(&mut self) -> Option<&mut SignalsPhase> {
        self.span.current_phase_mut()?.signals.as_mut()
    }

    /// Add an error to the telemetry record.
    pub fn add_error(&mut self, error: impl Into<String>) {
        self.span.add_error(error);
    }

    /// Get the trace ID.
    pub fn trace_id(&self) -> &str {
        &self.span.trace_id
    }

    /// Check if any output is configured.
    pub fn has_output_configured(&self) -> bool {
        self.debug_files_enabled
            || self
                .telemetry_config
                .as_ref()
                .map(|c| c.enabled)
                .unwrap_or(false)
    }

    /// Finalize and write all telemetry.
    ///
    /// Call at the end of evaluation. Drop impl also calls this if not finalized.
    pub fn finalize(&mut self, response: Option<Value>) -> Result<()> {
        if self.is_finalized {
            return Ok(());
        }

        // Finalize any open policy phases
        for phase in &mut self.span.phases {
            phase.finalize();
        }

        // Finalize the root span
        self.span.finalize(response);

        // Mark finalized BEFORE writing
        self.is_finalized = true;

        self.write_all()
    }

    /// Internal write method.
    fn write_all(&self) -> Result<()> {
        if self.debug_files_enabled {
            let debug_dir = self
                .debug_dir
                .as_deref()
                .unwrap_or_else(|| std::path::Path::new(".cupcake/debug"));

            if let Err(e) = TelemetryWriter::write_debug_file(&self.span, debug_dir) {
                warn!("Failed to write debug file: {}", e);
            } else {
                debug!("Wrote telemetry debug file to {:?}", debug_dir);
            }
        }

        if let Some(ref config) = self.telemetry_config {
            if config.enabled {
                let destination = config
                    .destination
                    .as_deref()
                    .unwrap_or_else(|| std::path::Path::new(".cupcake/telemetry"));

                if let Err(e) =
                    TelemetryWriter::write_telemetry(&self.span, &config.format, destination)
                {
                    warn!("Failed to write telemetry: {}", e);
                } else {
                    debug!("Wrote telemetry to {:?}", destination);
                }
            }
        }

        Ok(())
    }
}

impl Drop for TelemetryContext {
    fn drop(&mut self) {
        if !self.is_finalized {
            if std::thread::panicking() {
                self.span.add_error("Process panicked unexpectedly");
            }

            // Finalize phases
            for phase in &mut self.span.phases {
                phase.finalize();
            }

            // Finalize root span
            self.span.finalize(None);

            self.is_finalized = true;

            if self.has_output_configured() {
                let _ = self.write_all();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_context_creation() {
        let raw = json!({"hook_event_name": "PreToolUse"});
        let ctx = TelemetryContext::new(raw.clone(), HarnessType::ClaudeCode, "trace-123".into());

        assert_eq!(ctx.trace_id(), "trace-123");
        assert_eq!(ctx.span.raw_event, raw);
        assert!(ctx.span.enrich.is_none());
        assert!(ctx.span.phases.is_empty());
        assert!(!ctx.is_finalized);
    }

    #[test]
    fn test_enrichment_recording() {
        let raw = json!({"command": "rm -rf /"});
        let mut ctx = TelemetryContext::new(raw, HarnessType::ClaudeCode, "trace-123".into());

        let enriched = json!({"command": "rm -rf /", "resolved_file_path": "/canonical"});
        ctx.record_enrichment(enriched.clone(), vec!["symlink_resolution".into()], 100);

        assert!(ctx.span.enrich.is_some());
        let enrich = ctx.span.enrich.as_ref().unwrap();
        assert_eq!(enrich.enriched_event, enriched);
        assert_eq!(enrich.duration_us, 100);
        assert_eq!(enrich.parent_span_id, ctx.span.span_id);
    }

    #[test]
    fn test_policy_phases() {
        let raw = json!({});
        let mut ctx = TelemetryContext::new(raw, HarnessType::ClaudeCode, "trace-123".into());

        // Global phase
        {
            let global = ctx.start_phase("global");
            global
                .evaluation_mut()
                .record_routing(true, &["global.policy".into()]);
        }

        // Project phase
        {
            let project = ctx.start_phase("project");
            project.evaluation_mut().record_routing(false, &[]);
            project.evaluation_mut().record_exit("No policies matched");
        }

        assert_eq!(ctx.span.phases.len(), 2);
        assert_eq!(ctx.span.phases[0].name, "global");
        assert!(ctx.span.phases[0].evaluation.routed);
        assert_eq!(ctx.span.phases[1].name, "project");
        assert!(!ctx.span.phases[1].evaluation.routed);
    }

    #[test]
    fn test_signals_recording() {
        use super::super::span::SignalExecution;

        let raw = json!({});
        let mut ctx = TelemetryContext::new(raw, HarnessType::ClaudeCode, "trace-123".into());

        ctx.start_phase("project");

        if let Some(signals) = ctx.start_signals() {
            signals.add_signal(SignalExecution {
                name: "git_status".into(),
                command: "git status --porcelain".into(),
                result: json!(["M src/main.rs"]),
                duration_ms: Some(45),
                exit_code: Some(0),
            });
            signals.finalize(45);
        }

        let phase = &ctx.span.phases[0];
        assert!(phase.signals.is_some());
        assert_eq!(phase.signals.as_ref().unwrap().signals.len(), 1);
    }

    #[test]
    fn test_finalize_prevents_double_write() {
        let raw = json!({});
        let mut ctx = TelemetryContext::new(raw, HarnessType::ClaudeCode, "trace-123".into());

        ctx.finalize(None).unwrap();
        assert!(ctx.is_finalized);
        assert!(ctx.span.end_time_unix_nano > ctx.span.start_time_unix_nano);

        // Second finalize is no-op
        ctx.finalize(None).unwrap();
    }

    #[test]
    fn test_serialization() {
        let raw = json!({"test": true});
        let mut ctx = TelemetryContext::new(raw, HarnessType::ClaudeCode, "trace-123".into());
        ctx.record_enrichment(json!({"enriched": true}), vec!["op1".into()], 50);
        ctx.start_phase("project");

        let json = serde_json::to_string_pretty(&ctx.span).expect("serialize");

        assert!(json.contains("\"trace_id\": \"trace-123\""));
        assert!(json.contains("\"enrich\""));
        assert!(json.contains("\"phases\""));
        assert!(json.contains("\"operations\""));
    }
}
