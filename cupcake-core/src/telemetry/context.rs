//! TelemetryContext - the main telemetry capture structure.
//!
//! Flows through the entire evaluation pipeline, capturing data at each stage.
//! Implements Drop to ensure telemetry is written even on panic/early return.

use anyhow::Result;
use serde::Serialize;
use serde_json::Value;
use std::path::PathBuf;
use std::time::Instant;
use tracing::{debug, warn};

use crate::engine::rulebook::TelemetryConfig;
use crate::harness::types::HarnessType;

use super::span::{EnrichSpan, EvaluateSpan, IngestSpan};
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
/// 3. Evaluation phases recorded in Engine (`start_evaluation()`)
/// 4. Response recorded before output (`set_response()`)
/// 5. Written via `finalize()` or `Drop` impl
///
/// ## Drop Guard
///
/// Implements `Drop` to ensure telemetry is written even if:
/// - The process panics
/// - An error causes early return via `?`
/// - Any unexpected exit path
#[derive(Debug, Serialize)]
pub struct TelemetryContext {
    /// Root span capturing the raw event
    pub ingest: IngestSpan,

    /// Child span capturing preprocessing results
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enrich: Option<EnrichSpan>,

    /// Child spans for each evaluation phase (global, catalog, project)
    #[serde(default)]
    pub evaluations: Vec<EvaluateSpan>,

    /// Response sent back to the agent
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response_to_agent: Option<Value>,

    /// Errors encountered during processing
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<String>,

    /// Total duration from ingest to finalize in milliseconds
    #[serde(default)]
    pub total_duration_ms: u64,

    // --- Configuration (not serialized) ---
    /// Whether debug file writing is enabled (--debug-files flag)
    #[serde(skip)]
    debug_files_enabled: bool,

    /// Override directory for debug files
    #[serde(skip)]
    debug_dir: Option<PathBuf>,

    /// Telemetry configuration from rulebook
    #[serde(skip)]
    telemetry_config: Option<TelemetryConfig>,

    // --- Internal state (not serialized) ---
    /// Whether finalize() has been called (prevents double-write in Drop)
    #[serde(skip)]
    is_finalized: bool,

    /// Start time for total duration calculation
    #[serde(skip)]
    start_instant: Instant,
}

impl TelemetryContext {
    /// Create a new telemetry context capturing the raw event.
    ///
    /// This should be called IMMEDIATELY after parsing stdin JSON,
    /// BEFORE any preprocessing or mutations.
    ///
    /// # Arguments
    ///
    /// * `raw_event` - The event exactly as received from stdin
    /// * `harness` - Which agent harness sent this event
    /// * `trace_id` - Unique identifier for this evaluation
    pub fn new(raw_event: Value, harness: HarnessType, trace_id: String) -> Self {
        Self {
            ingest: IngestSpan::new(raw_event, trace_id, harness),
            enrich: None,
            evaluations: Vec::new(),
            response_to_agent: None,
            errors: Vec::new(),
            total_duration_ms: 0,
            debug_files_enabled: false,
            debug_dir: None,
            telemetry_config: None,
            is_finalized: false,
            start_instant: Instant::now(),
        }
    }

    /// Configure debug file output.
    ///
    /// Call this after engine initialization when telemetry config is available.
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

    /// Record the enrichment/preprocessing stage.
    ///
    /// Call this AFTER preprocessing but BEFORE engine evaluation.
    ///
    /// # Arguments
    ///
    /// * `enriched_event` - The event after preprocessing transformations
    /// * `operations` - List of preprocessing operations applied
    /// * `duration_us` - Time taken for preprocessing in microseconds
    pub fn record_enrichment(
        &mut self,
        enriched_event: Value,
        operations: Vec<String>,
        duration_us: u64,
    ) {
        // Enrich span starts right after ingest, use ingest's start time
        let start_time = self.ingest.start_time_unix_nano;
        self.enrich = Some(EnrichSpan::new(
            enriched_event,
            operations,
            duration_us,
            self.ingest.span_id.clone(),
            start_time,
        ));
    }

    /// Start a new evaluation phase span.
    ///
    /// Returns a mutable reference to the span for recording results.
    ///
    /// # Arguments
    ///
    /// * `phase` - Phase identifier: "global", "catalog:name", or "project"
    pub fn start_evaluation(&mut self, phase: impl Into<String>) -> &mut EvaluateSpan {
        let span = EvaluateSpan::new(phase, self.ingest.span_id.clone());
        self.evaluations.push(span);
        self.evaluations.last_mut().unwrap()
    }

    /// Get mutable reference to the current (last) evaluation span.
    ///
    /// Returns None if no evaluation has been started.
    pub fn current_evaluation_mut(&mut self) -> Option<&mut EvaluateSpan> {
        self.evaluations.last_mut()
    }

    /// Set the response that will be sent to the agent.
    pub fn set_response(&mut self, response: Value) {
        self.response_to_agent = Some(response);
    }

    /// Add an error to the telemetry record.
    pub fn add_error(&mut self, error: impl Into<String>) {
        self.errors.push(error.into());
    }

    /// Get the trace ID for this evaluation.
    pub fn trace_id(&self) -> &str {
        &self.ingest.trace_id
    }

    /// Get all evaluation spans.
    pub fn evaluations(&self) -> &[EvaluateSpan] {
        &self.evaluations
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
    /// This should be called at the end of evaluation. The Drop impl
    /// will also call this if not already finalized.
    ///
    /// # Arguments
    ///
    /// * `response` - Optional final response to record
    pub fn finalize(&mut self, response: Option<Value>) -> Result<()> {
        if self.is_finalized {
            return Ok(());
        }

        // Record response if provided
        if let Some(r) = response {
            self.response_to_agent = Some(r);
        }

        // Finalize any open evaluation spans
        for eval in &mut self.evaluations {
            eval.finalize();
        }

        // Finalize the ingest span (sets end_time_unix_nano)
        self.ingest.finalize();

        // Calculate total duration
        self.total_duration_ms = self.start_instant.elapsed().as_millis() as u64;

        // Mark as finalized BEFORE writing to prevent double-write
        self.is_finalized = true;

        // Write telemetry
        self.write_all()
    }

    /// Internal write method used by finalize() and Drop.
    fn write_all(&self) -> Result<()> {
        // Write debug files if enabled
        if self.debug_files_enabled {
            let debug_dir = self
                .debug_dir
                .as_deref()
                .unwrap_or_else(|| std::path::Path::new(".cupcake/debug"));

            if let Err(e) = TelemetryWriter::write_debug_file(self, debug_dir) {
                warn!("Failed to write debug file: {}", e);
            } else {
                debug!("Wrote telemetry debug file to {:?}", debug_dir);
            }
        }

        // Write telemetry if configured
        if let Some(ref config) = self.telemetry_config {
            if config.enabled {
                let destination = config
                    .destination
                    .as_deref()
                    .unwrap_or_else(|| std::path::Path::new(".cupcake/telemetry"));

                if let Err(e) = TelemetryWriter::write_telemetry(self, &config.format, destination)
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

/// Drop guard ensures telemetry is written even on panic/early return.
impl Drop for TelemetryContext {
    fn drop(&mut self) {
        if !self.is_finalized {
            // Check if we're panicking
            if std::thread::panicking() {
                self.errors.push("Process panicked unexpectedly".into());
            }

            // Calculate duration
            self.total_duration_ms = self.start_instant.elapsed().as_millis() as u64;

            // Finalize evaluation spans
            for eval in &mut self.evaluations {
                eval.finalize();
            }

            // Finalize ingest span (sets end_time_unix_nano)
            self.ingest.finalize();

            // Mark finalized to prevent issues if write_all somehow recurses
            self.is_finalized = true;

            // Best-effort write - don't panic in Drop
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
        assert_eq!(ctx.ingest.raw_event, raw);
        assert!(ctx.enrich.is_none());
        assert!(ctx.evaluations.is_empty());
        assert!(!ctx.is_finalized);
        // OTLP fields on ingest
        assert_eq!(ctx.ingest.span_id.len(), 16);
        assert!(ctx.ingest.parent_span_id.is_empty());
        assert!(ctx.ingest.start_time_unix_nano > 0);
    }

    #[test]
    fn test_enrichment_recording() {
        let raw = json!({"command": "rm -rf /"});
        let mut ctx = TelemetryContext::new(raw, HarnessType::ClaudeCode, "trace-123".into());

        let enriched = json!({"command": "rm -rf /", "resolved_file_path": "/canonical"});
        ctx.record_enrichment(enriched.clone(), vec!["symlink_resolution".into()], 100);

        assert!(ctx.enrich.is_some());
        let enrich = ctx.enrich.as_ref().unwrap();
        assert_eq!(enrich.enriched_event, enriched);
        assert_eq!(enrich.duration_us, 100);
        // OTLP parent-child relationship
        assert_eq!(enrich.parent_span_id, ctx.ingest.span_id);
        assert!(enrich.end_time_unix_nano > enrich.start_time_unix_nano);
    }

    #[test]
    fn test_evaluation_phases() {
        let raw = json!({});
        let mut ctx = TelemetryContext::new(raw, HarnessType::ClaudeCode, "trace-123".into());
        let ingest_span_id = ctx.ingest.span_id.clone();

        // Global phase
        {
            let global = ctx.start_evaluation("global");
            global.record_routing(true, &["global.policy".into()]);
        }

        // Project phase
        {
            let project = ctx.start_evaluation("project");
            project.record_routing(false, &[]);
            project.record_exit("No policies matched");
        }

        assert_eq!(ctx.evaluations.len(), 2);
        assert_eq!(ctx.evaluations[0].phase, "global");
        assert!(ctx.evaluations[0].routed);
        assert_eq!(ctx.evaluations[1].phase, "project");
        assert!(!ctx.evaluations[1].routed);
        // OTLP parent-child relationship
        assert_eq!(ctx.evaluations[0].parent_span_id, ingest_span_id);
        assert_eq!(ctx.evaluations[1].parent_span_id, ingest_span_id);
    }

    #[test]
    fn test_serialization() {
        let raw = json!({"test": true});
        let mut ctx = TelemetryContext::new(raw, HarnessType::ClaudeCode, "trace-123".into());
        ctx.record_enrichment(json!({"enriched": true}), vec!["op1".into()], 50);

        let json = serde_json::to_string_pretty(&ctx).expect("serialize");

        // Verify structure (trace_id is inside ingest span)
        assert!(json.contains("\"ingest\""));
        assert!(json.contains("\"enrich\""));
        assert!(json.contains("trace-123")); // trace_id is inside ingest
        assert!(json.contains("\"preprocessing_operations\""));
        // OTLP fields should be present
        assert!(json.contains("\"span_id\""));
        assert!(json.contains("\"parent_span_id\""));
        assert!(json.contains("\"start_time_unix_nano\""));
        assert!(json.contains("\"end_time_unix_nano\""));
    }

    #[test]
    fn test_finalize_prevents_double_write() {
        let raw = json!({});
        let mut ctx = TelemetryContext::new(raw, HarnessType::ClaudeCode, "trace-123".into());

        // First finalize
        ctx.finalize(None).unwrap();
        assert!(ctx.is_finalized);
        // Ingest span should have end_time set
        assert!(ctx.ingest.end_time_unix_nano > ctx.ingest.start_time_unix_nano);

        // Second finalize should be no-op
        ctx.finalize(None).unwrap();
    }

    #[test]
    fn test_otlp_parent_child_relationships() {
        let raw = json!({"hook_event_name": "PreToolUse"});
        let mut ctx = TelemetryContext::new(raw, HarnessType::ClaudeCode, "trace-456".into());
        let ingest_span_id = ctx.ingest.span_id.clone();

        // Record enrichment
        ctx.record_enrichment(json!({"enriched": true}), vec!["op1".into()], 100);

        // Start evaluation
        ctx.start_evaluation("project");

        // Verify parent-child relationships
        let enrich = ctx.enrich.as_ref().unwrap();
        assert_eq!(enrich.parent_span_id, ingest_span_id);

        let eval = &ctx.evaluations[0];
        assert_eq!(eval.parent_span_id, ingest_span_id);

        // All spans should have unique span_ids
        assert_ne!(ctx.ingest.span_id, enrich.span_id);
        assert_ne!(ctx.ingest.span_id, eval.span_id);
        assert_ne!(enrich.span_id, eval.span_id);
    }
}
