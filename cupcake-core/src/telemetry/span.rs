//! Telemetry span types for hierarchical event capture.
//!
//! Three span types map to the event processing lifecycle:
//! - [`IngestSpan`] - Raw event capture (root span)
//! - [`EnrichSpan`] - Preprocessing results (child span)
//! - [`EvaluateSpan`] - Policy evaluation results (child span, may not exist)

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

use crate::engine::decision::{DecisionSet, FinalDecision};
use crate::harness::types::HarnessType;

/// Generate a unique 16-character hex span ID (8 bytes).
fn generate_span_id() -> String {
    // Use UUID v7 (time-ordered) - bytes 8-15 contain random bits for uniqueness
    let uuid = Uuid::now_v7();
    // Take bytes 8-15 (the random portion) for span ID to ensure uniqueness
    hex::encode(&uuid.as_bytes()[8..16])
}

/// Convert SystemTime to nanoseconds since Unix epoch.
fn system_time_to_nanos(time: &SystemTime) -> u64 {
    time.duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0)
}

/// Root span capturing the raw event before any processing.
///
/// This span is ALWAYS created when telemetry is enabled, ensuring
/// we capture every event even if it exits early.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestSpan {
    // --- OTLP-required fields ---
    /// Unique span identifier (8-byte hex, 16 chars)
    pub span_id: String,

    /// Parent span ID (empty for root span)
    pub parent_span_id: String,

    /// Start time in nanoseconds since Unix epoch
    pub start_time_unix_nano: u64,

    /// End time in nanoseconds since Unix epoch (set at finalize)
    pub end_time_unix_nano: u64,

    // --- Cupcake-specific fields ---
    /// The raw event exactly as received from stdin
    pub raw_event: Value,

    /// Unique trace identifier for this evaluation (UUID v7)
    pub trace_id: String,

    /// When the event was received (human-readable RFC3339)
    #[serde(serialize_with = "serialize_system_time")]
    pub timestamp: SystemTime,

    /// Which harness (agent) sent this event
    pub harness: HarnessType,
}

impl IngestSpan {
    /// Create a new ingest span capturing the raw event
    pub fn new(raw_event: Value, trace_id: String, harness: HarnessType) -> Self {
        let now = SystemTime::now();
        Self {
            span_id: generate_span_id(),
            parent_span_id: String::new(), // Root span has no parent
            start_time_unix_nano: system_time_to_nanos(&now),
            end_time_unix_nano: 0, // Set at finalize
            raw_event,
            trace_id,
            timestamp: now,
            harness,
        }
    }

    /// Set the end time for this span
    pub fn finalize(&mut self) {
        self.end_time_unix_nano = system_time_to_nanos(&SystemTime::now());
    }
}

/// Child span capturing preprocessing/enrichment results.
///
/// Records the event after preprocessing transformations like:
/// - Whitespace normalization (TOB-3 defense)
/// - Symlink resolution (TOB-4 defense)
/// - File path canonicalization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnrichSpan {
    // --- OTLP-required fields ---
    /// Unique span identifier (8-byte hex, 16 chars)
    pub span_id: String,

    /// Parent span ID (IngestSpan's span_id)
    pub parent_span_id: String,

    /// Start time in nanoseconds since Unix epoch
    pub start_time_unix_nano: u64,

    /// End time in nanoseconds since Unix epoch
    pub end_time_unix_nano: u64,

    // --- Cupcake-specific fields ---
    /// The event after preprocessing transformations
    pub enriched_event: Value,

    /// List of preprocessing operations applied
    /// e.g., ["whitespace_normalization", "symlink_resolution"]
    pub preprocessing_operations: Vec<String>,

    /// Time taken for preprocessing in microseconds
    pub duration_us: u64,
}

impl EnrichSpan {
    /// Create a new enrich span with the transformed event
    ///
    /// # Arguments
    /// * `enriched_event` - The event after preprocessing
    /// * `operations` - List of preprocessing operations applied
    /// * `duration_us` - Duration in microseconds
    /// * `parent_span_id` - The IngestSpan's span_id
    /// * `start_time_unix_nano` - When preprocessing started (from parent context)
    pub fn new(
        enriched_event: Value,
        operations: Vec<String>,
        duration_us: u64,
        parent_span_id: String,
        start_time_unix_nano: u64,
    ) -> Self {
        Self {
            span_id: generate_span_id(),
            parent_span_id,
            start_time_unix_nano,
            end_time_unix_nano: start_time_unix_nano + (duration_us * 1000), // us to ns
            enriched_event,
            preprocessing_operations: operations,
            duration_us,
        }
    }
}

/// Child span capturing policy evaluation results.
///
/// One EvaluateSpan is created per evaluation phase:
/// - "global" - Global policies (highest priority)
/// - "catalog:name" - Catalog overlay policies
/// - "project" - Project-level policies
///
/// This span may NOT exist if the event exits before evaluation
/// (e.g., routing finds no matching policies).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluateSpan {
    // --- OTLP-required fields ---
    /// Unique span identifier (8-byte hex, 16 chars)
    pub span_id: String,

    /// Parent span ID (IngestSpan's span_id)
    pub parent_span_id: String,

    /// Start time in nanoseconds since Unix epoch
    pub start_time_unix_nano: u64,

    /// End time in nanoseconds since Unix epoch (set at finalize)
    pub end_time_unix_nano: u64,

    // --- Cupcake-specific fields ---
    /// Evaluation phase: "global", "catalog:overlay_name", or "project"
    pub phase: String,

    /// Whether routing found matching policies
    pub routed: bool,

    /// Names of policies that matched during routing
    #[serde(default)]
    pub matched_policies: Vec<String>,

    /// Raw decision set from WASM evaluation
    /// Contains all individual policy decisions before synthesis
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wasm_decision_set: Option<DecisionSet>,

    /// Final synthesized decision after priority resolution
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub final_decision: Option<FinalDecision>,

    /// Reason for early exit, if any
    /// e.g., "No policies matched - implicit allow"
    /// e.g., "Global halt: Protected path violation"
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_reason: Option<String>,

    /// Signals that were executed for this phase
    #[serde(default)]
    pub signals_executed: Vec<SignalExecution>,

    /// Time taken for this evaluation phase in milliseconds
    pub duration_ms: u64,

    /// When this evaluation phase started (for duration calculation)
    #[serde(skip)]
    start_time: Option<std::time::Instant>,
}

impl EvaluateSpan {
    /// Create a new evaluate span for a phase
    ///
    /// # Arguments
    /// * `phase` - Evaluation phase identifier
    /// * `parent_span_id` - The IngestSpan's span_id
    pub fn new(phase: impl Into<String>, parent_span_id: String) -> Self {
        Self {
            span_id: generate_span_id(),
            parent_span_id,
            start_time_unix_nano: system_time_to_nanos(&SystemTime::now()),
            end_time_unix_nano: 0, // Set at finalize
            phase: phase.into(),
            routed: false,
            matched_policies: Vec::new(),
            wasm_decision_set: None,
            final_decision: None,
            exit_reason: None,
            signals_executed: Vec::new(),
            duration_ms: 0,
            start_time: Some(std::time::Instant::now()),
        }
    }

    /// Record routing results
    pub fn record_routing(&mut self, matched: bool, policies: &[String]) {
        self.routed = matched;
        self.matched_policies = policies.to_vec();
    }

    /// Record WASM evaluation results
    pub fn record_wasm_result(&mut self, decision_set: &DecisionSet) {
        self.wasm_decision_set = Some(decision_set.clone());
    }

    /// Record the final synthesized decision
    pub fn record_final_decision(&mut self, decision: &FinalDecision) {
        self.final_decision = Some(decision.clone());
    }

    /// Record an early exit reason
    pub fn record_exit(&mut self, reason: impl Into<String>) {
        self.exit_reason = Some(reason.into());
    }

    /// Record a signal execution
    pub fn record_signal(&mut self, execution: SignalExecution) {
        self.signals_executed.push(execution);
    }

    /// Finalize the span, recording duration and end time
    pub fn finalize(&mut self) {
        if let Some(start) = self.start_time.take() {
            self.duration_ms = start.elapsed().as_millis() as u64;
            // Calculate end time from start + duration
            self.end_time_unix_nano =
                self.start_time_unix_nano + (self.duration_ms * 1_000_000); // ms to ns
        }
    }
}

impl Default for EvaluateSpan {
    fn default() -> Self {
        Self::new("unknown", String::new())
    }
}

/// Record of a signal execution for telemetry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalExecution {
    /// Name of the signal
    pub name: String,

    /// Command that was executed
    pub command: String,

    /// Result of the signal execution (parsed JSON or raw output)
    pub result: Value,

    /// Duration in milliseconds
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,

    /// Exit code if available
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
}

// Helper for serializing SystemTime
fn serialize_system_time<S>(time: &SystemTime, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    use chrono::{DateTime, Utc};
    let datetime: DateTime<Utc> = (*time).into();
    serializer.serialize_str(&datetime.to_rfc3339())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_ingest_span_creation() {
        let raw = json!({"hook_event_name": "PreToolUse", "tool_name": "Bash"});
        let span = IngestSpan::new(raw.clone(), "trace-123".into(), HarnessType::ClaudeCode);

        assert_eq!(span.trace_id, "trace-123");
        assert_eq!(span.raw_event, raw);
        assert_eq!(span.harness, HarnessType::ClaudeCode);
        // OTLP fields
        assert_eq!(span.span_id.len(), 16); // 8 bytes = 16 hex chars
        assert!(span.parent_span_id.is_empty()); // Root span has no parent
        assert!(span.start_time_unix_nano > 0);
        assert_eq!(span.end_time_unix_nano, 0); // Not finalized yet
    }

    #[test]
    fn test_ingest_span_finalize() {
        let raw = json!({"test": true});
        let mut span = IngestSpan::new(raw, "trace-456".into(), HarnessType::ClaudeCode);

        assert_eq!(span.end_time_unix_nano, 0);
        span.finalize();
        assert!(span.end_time_unix_nano > span.start_time_unix_nano);
    }

    #[test]
    fn test_enrich_span_creation() {
        let enriched = json!({"resolved_file_path": "/canonical/path"});
        let parent_span_id = "abc123def4567890".to_string();
        let start_time: u64 = 1_000_000_000_000;
        let duration_us: u64 = 150;

        let span = EnrichSpan::new(
            enriched.clone(),
            vec!["symlink_resolution".into()],
            duration_us,
            parent_span_id.clone(),
            start_time,
        );

        assert_eq!(span.enriched_event, enriched);
        assert_eq!(span.preprocessing_operations, vec!["symlink_resolution"]);
        assert_eq!(span.duration_us, duration_us);
        // OTLP fields
        assert_eq!(span.span_id.len(), 16);
        assert_eq!(span.parent_span_id, parent_span_id);
        assert_eq!(span.start_time_unix_nano, start_time);
        assert_eq!(
            span.end_time_unix_nano,
            start_time + (duration_us * 1000)
        );
    }

    #[test]
    fn test_evaluate_span_creation() {
        let parent_span_id = "parent12345678ab".to_string();
        let span = EvaluateSpan::new("project", parent_span_id.clone());

        assert_eq!(span.phase, "project");
        assert_eq!(span.span_id.len(), 16);
        assert_eq!(span.parent_span_id, parent_span_id);
        assert!(span.start_time_unix_nano > 0);
        assert_eq!(span.end_time_unix_nano, 0); // Not finalized yet
    }

    #[test]
    fn test_evaluate_span_routing() {
        let mut span = EvaluateSpan::new("project", "parent123".into());
        span.record_routing(true, &["policy.a".into(), "policy.b".into()]);

        assert!(span.routed);
        assert_eq!(span.matched_policies.len(), 2);
    }

    #[test]
    fn test_evaluate_span_early_exit() {
        let mut span = EvaluateSpan::new("project", "parent123".into());
        span.record_routing(false, &[]);
        span.record_exit("No policies matched - implicit allow");

        assert!(!span.routed);
        assert_eq!(
            span.exit_reason,
            Some("No policies matched - implicit allow".into())
        );
    }

    #[test]
    fn test_evaluate_span_finalize() {
        let mut span = EvaluateSpan::new("global", "parent456".into());
        std::thread::sleep(std::time::Duration::from_millis(1));
        span.finalize();

        assert!(span.duration_ms >= 1);
        assert!(span.end_time_unix_nano > span.start_time_unix_nano);
    }

    #[test]
    fn test_span_serialization() {
        let span = EvaluateSpan::new("global", "parent789".into());
        let json = serde_json::to_string(&span).expect("serialize");
        assert!(json.contains("\"phase\":\"global\""));
        assert!(json.contains("\"routed\":false"));
        // OTLP fields should be present
        assert!(json.contains("\"span_id\""));
        assert!(json.contains("\"parent_span_id\""));
        assert!(json.contains("\"start_time_unix_nano\""));
        assert!(json.contains("\"end_time_unix_nano\""));
    }
}
