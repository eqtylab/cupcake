//! Telemetry span types for cupcake event lifecycle.
//!
//! The trace hierarchy follows the natural flow of policy evaluation:
//!
//! ```text
//! CupcakeSpan (root - entire evaluation lifecycle)
//!   ├── raw_event, harness, trace_id
//!   │
//!   ├── enrich: EnrichPhase (preprocessing)
//!   │     └── operations, enriched_event, duration
//!   │
//!   ├── phases: Vec<PolicyPhase> (one per policy layer)
//!   │     ├── name: "global" | "catalog:X" | "project"
//!   │     ├── signals: SignalsPhase (external program collection)
//!   │     │     └── signals: Vec<SignalExecution>
//!   │     └── evaluation: EvaluationResult (WASM → decision)
//!   │
//!   └── response, total_duration_ms
//! ```

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use uuid::Uuid;

use crate::engine::decision::{DecisionSet, FinalDecision};
use crate::harness::types::HarnessType;

// ============================================================================
// Helper Functions
// ============================================================================

/// Generate a unique 16-character hex span ID (8 bytes).
fn generate_span_id() -> String {
    let uuid = Uuid::now_v7();
    hex::encode(&uuid.as_bytes()[8..16])
}

/// Convert SystemTime to nanoseconds since Unix epoch.
fn system_time_to_nanos(time: &SystemTime) -> u64 {
    time.duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0)
}

/// Serialize SystemTime as RFC3339 string.
fn serialize_system_time<S>(time: &SystemTime, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    use chrono::{DateTime, Utc};
    let datetime: DateTime<Utc> = (*time).into();
    serializer.serialize_str(&datetime.to_rfc3339())
}

// ============================================================================
// Root Span
// ============================================================================

/// Root span capturing the entire cupcake evaluation lifecycle.
///
/// Created immediately when an agent event arrives, before any processing.
/// Contains all child phases and the final response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CupcakeSpan {
    // --- OTLP-compatible span identity ---
    /// Unique span identifier (16-char hex)
    pub span_id: String,

    /// Trace identifier for correlation (UUID v7, time-ordered)
    pub trace_id: String,

    /// Start time in nanoseconds since Unix epoch
    pub start_time_unix_nano: u64,

    /// End time in nanoseconds since Unix epoch (set at finalize)
    pub end_time_unix_nano: u64,

    // --- Event metadata ---
    /// The raw event exactly as received from the agent
    pub raw_event: Value,

    /// Which harness (agent) sent this event
    pub harness: HarnessType,

    /// When the event was received (human-readable)
    #[serde(serialize_with = "serialize_system_time")]
    pub timestamp: SystemTime,

    // --- Processing phases ---
    /// Preprocessing phase (symlink resolution, whitespace normalization)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enrich: Option<EnrichPhase>,

    /// Policy evaluation phases (global → catalog → project)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub phases: Vec<PolicyPhase>,

    // --- Final output ---
    /// Response sent back to the agent
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response: Option<Value>,

    /// Errors encountered during processing
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<String>,

    /// Total evaluation duration in milliseconds
    pub total_duration_ms: u64,

    // --- Internal timing (not serialized) ---
    #[serde(skip)]
    start_instant: Option<Instant>,
}

impl CupcakeSpan {
    /// Create a new root span capturing the raw agent event.
    pub fn new(raw_event: Value, harness: HarnessType, trace_id: String) -> Self {
        let now = SystemTime::now();
        Self {
            span_id: generate_span_id(),
            trace_id,
            start_time_unix_nano: system_time_to_nanos(&now),
            end_time_unix_nano: 0,
            raw_event,
            harness,
            timestamp: now,
            enrich: None,
            phases: Vec::new(),
            response: None,
            errors: Vec::new(),
            total_duration_ms: 0,
            start_instant: Some(Instant::now()),
        }
    }

    /// Get the span_id for child spans to reference as parent.
    pub fn span_id(&self) -> &str {
        &self.span_id
    }

    /// Record the enrichment phase.
    pub fn set_enrich(&mut self, enrich: EnrichPhase) {
        self.enrich = Some(enrich);
    }

    /// Add a policy evaluation phase.
    pub fn add_phase(&mut self, phase: PolicyPhase) {
        self.phases.push(phase);
    }

    /// Get the current (last) policy phase mutably.
    pub fn current_phase_mut(&mut self) -> Option<&mut PolicyPhase> {
        self.phases.last_mut()
    }

    /// Record an error.
    pub fn add_error(&mut self, error: impl Into<String>) {
        self.errors.push(error.into());
    }

    /// Finalize the span with the response.
    pub fn finalize(&mut self, response: Option<Value>) {
        self.response = response;
        self.end_time_unix_nano = system_time_to_nanos(&SystemTime::now());
        if let Some(start) = self.start_instant.take() {
            self.total_duration_ms = start.elapsed().as_millis() as u64;
        }
    }
}

// ============================================================================
// Enrich Phase
// ============================================================================

/// Preprocessing phase - transforms raw event before policy evaluation.
///
/// Operations include:
/// - `whitespace_normalization` - Collapse multiple spaces (TOB-3 defense)
/// - `symlink_resolution` - Resolve symlinks to canonical paths (TOB-4 defense)
/// - `content_unification` - Unify Write/Edit content fields
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnrichPhase {
    /// Unique span identifier
    pub span_id: String,

    /// Parent span ID (CupcakeSpan.span_id)
    pub parent_span_id: String,

    /// Start time in nanoseconds since Unix epoch
    pub start_time_unix_nano: u64,

    /// End time in nanoseconds since Unix epoch
    pub end_time_unix_nano: u64,

    /// The event after preprocessing transformations
    pub enriched_event: Value,

    /// List of preprocessing operations applied
    pub operations: Vec<String>,

    /// Duration in microseconds
    pub duration_us: u64,
}

impl EnrichPhase {
    /// Create a new enrich phase.
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
            end_time_unix_nano: start_time_unix_nano + (duration_us * 1000),
            enriched_event,
            operations,
            duration_us,
        }
    }
}

// ============================================================================
// Policy Phase
// ============================================================================

/// A single policy evaluation phase (global, catalog overlay, or project).
///
/// Each phase follows the flow: route → collect signals → evaluate WASM → synthesize
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyPhase {
    /// Unique span identifier
    pub span_id: String,

    /// Parent span ID (CupcakeSpan.span_id)
    pub parent_span_id: String,

    /// Start time in nanoseconds since Unix epoch
    pub start_time_unix_nano: u64,

    /// End time in nanoseconds since Unix epoch
    pub end_time_unix_nano: u64,

    /// Phase name: "global", "catalog:overlay_name", or "project"
    pub name: String,

    /// Signal collection sub-phase
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signals: Option<SignalsPhase>,

    /// Policy evaluation result
    pub evaluation: EvaluationResult,

    /// Total phase duration in milliseconds
    pub duration_ms: u64,

    // --- Internal timing (not serialized) ---
    #[serde(skip)]
    start_instant: Option<Instant>,
}

impl PolicyPhase {
    /// Create a new policy phase.
    pub fn new(name: impl Into<String>, parent_span_id: String) -> Self {
        Self {
            span_id: generate_span_id(),
            parent_span_id,
            start_time_unix_nano: system_time_to_nanos(&SystemTime::now()),
            end_time_unix_nano: 0,
            name: name.into(),
            signals: None,
            evaluation: EvaluationResult::default(),
            duration_ms: 0,
            start_instant: Some(Instant::now()),
        }
    }

    /// Get the span_id for child spans.
    pub fn span_id(&self) -> &str {
        &self.span_id
    }

    /// Set the signals phase.
    pub fn set_signals(&mut self, signals: SignalsPhase) {
        self.signals = Some(signals);
    }

    /// Get mutable reference to evaluation result.
    pub fn evaluation_mut(&mut self) -> &mut EvaluationResult {
        &mut self.evaluation
    }

    /// Record a signal execution.
    /// Creates the SignalsPhase if it doesn't exist.
    pub fn record_signal(&mut self, signal: SignalExecution) {
        if self.signals.is_none() {
            self.signals = Some(SignalsPhase::new(
                self.span_id.clone(),
                self.start_time_unix_nano,
            ));
        }
        if let Some(ref mut signals) = self.signals {
            signals.add_signal(signal);
        }
    }

    /// Finalize the phase, recording duration.
    pub fn finalize(&mut self) {
        self.end_time_unix_nano = system_time_to_nanos(&SystemTime::now());
        if let Some(start) = self.start_instant.take() {
            self.duration_ms = start.elapsed().as_millis() as u64;
        }
    }
}

// ============================================================================
// Signals Phase
// ============================================================================

/// Signal collection phase - executes external programs to gather context.
///
/// Signals are shell commands defined in the rulebook that provide additional
/// context for policy evaluation (e.g., `git status`, file checks).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalsPhase {
    /// Unique span identifier
    pub span_id: String,

    /// Parent span ID (PolicyPhase.span_id)
    pub parent_span_id: String,

    /// Start time in nanoseconds since Unix epoch
    pub start_time_unix_nano: u64,

    /// End time in nanoseconds since Unix epoch
    pub end_time_unix_nano: u64,

    /// Individual signal executions
    pub signals: Vec<SignalExecution>,

    /// Total signal collection duration in milliseconds
    pub duration_ms: u64,
}

impl SignalsPhase {
    /// Create a new signals phase.
    pub fn new(parent_span_id: String, start_time_unix_nano: u64) -> Self {
        Self {
            span_id: generate_span_id(),
            parent_span_id,
            start_time_unix_nano,
            end_time_unix_nano: 0,
            signals: Vec::new(),
            duration_ms: 0,
        }
    }

    /// Add a signal execution.
    pub fn add_signal(&mut self, signal: SignalExecution) {
        self.signals.push(signal);
    }

    /// Finalize with end time and duration.
    pub fn finalize(&mut self, duration_ms: u64) {
        self.end_time_unix_nano = system_time_to_nanos(&SystemTime::now());
        self.duration_ms = duration_ms;
    }
}

/// Record of a single signal execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalExecution {
    /// Signal name from rulebook
    pub name: String,

    /// Shell command that was executed
    pub command: String,

    /// Result (parsed JSON or raw output)
    pub result: Value,

    /// Execution duration in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,

    /// Exit code if available
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
}

// ============================================================================
// Evaluation Result
// ============================================================================

/// Result of policy evaluation (WASM execution + synthesis).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EvaluationResult {
    /// Whether routing found matching policies
    pub routed: bool,

    /// Names of policies that matched during routing
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub matched_policies: Vec<String>,

    /// Raw decision set from WASM evaluation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wasm_decision_set: Option<DecisionSet>,

    /// Final synthesized decision
    #[serde(skip_serializing_if = "Option::is_none")]
    pub final_decision: Option<FinalDecision>,

    /// Reason for early exit (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_reason: Option<String>,
}

impl EvaluationResult {
    /// Record routing results.
    pub fn record_routing(&mut self, matched: bool, policies: &[String]) {
        self.routed = matched;
        self.matched_policies = policies.to_vec();
    }

    /// Record WASM evaluation results.
    pub fn record_wasm_result(&mut self, decision_set: &DecisionSet) {
        self.wasm_decision_set = Some(decision_set.clone());
    }

    /// Record the final synthesized decision.
    pub fn record_final_decision(&mut self, decision: &FinalDecision) {
        self.final_decision = Some(decision.clone());
    }

    /// Record an early exit reason.
    pub fn record_exit(&mut self, reason: impl Into<String>) {
        self.exit_reason = Some(reason.into());
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_cupcake_span_creation() {
        let raw = json!({"hook_event_name": "PreToolUse", "tool_name": "Bash"});
        let span = CupcakeSpan::new(raw.clone(), HarnessType::ClaudeCode, "trace-123".into());

        assert_eq!(span.trace_id, "trace-123");
        assert_eq!(span.raw_event, raw);
        assert_eq!(span.harness, HarnessType::ClaudeCode);
        assert_eq!(span.span_id.len(), 16);
        assert!(span.start_time_unix_nano > 0);
        assert_eq!(span.end_time_unix_nano, 0); // Not finalized yet
    }

    #[test]
    fn test_cupcake_span_finalize() {
        let raw = json!({"test": true});
        let mut span = CupcakeSpan::new(raw, HarnessType::ClaudeCode, "trace-456".into());

        assert_eq!(span.end_time_unix_nano, 0);
        span.finalize(Some(json!({"decision": "allow"})));

        assert!(span.end_time_unix_nano > span.start_time_unix_nano);
        assert!(span.response.is_some());
    }

    #[test]
    fn test_enrich_phase() {
        let enriched = json!({"resolved_file_path": "/canonical/path"});
        let phase = EnrichPhase::new(
            enriched.clone(),
            vec!["symlink_resolution".into()],
            150,
            "parent123".into(),
            1_000_000_000_000,
        );

        assert_eq!(phase.enriched_event, enriched);
        assert_eq!(phase.operations, vec!["symlink_resolution"]);
        assert_eq!(phase.duration_us, 150);
        assert_eq!(phase.span_id.len(), 16);
    }

    #[test]
    fn test_policy_phase() {
        let mut phase = PolicyPhase::new("project", "parent456".into());

        assert_eq!(phase.name, "project");
        assert_eq!(phase.span_id.len(), 16);

        phase
            .evaluation_mut()
            .record_routing(true, &["policy.a".into()]);
        assert!(phase.evaluation.routed);

        std::thread::sleep(std::time::Duration::from_millis(1));
        phase.finalize();
        assert!(phase.duration_ms >= 1);
    }

    #[test]
    fn test_signals_phase() {
        let mut signals = SignalsPhase::new("parent789".into(), 1_000_000_000_000);

        signals.add_signal(SignalExecution {
            name: "git_status".into(),
            command: "git status --porcelain".into(),
            result: json!(["M src/main.rs"]),
            duration_ms: Some(45),
            exit_code: Some(0),
        });

        assert_eq!(signals.signals.len(), 1);
        assert_eq!(signals.signals[0].name, "git_status");
    }

    #[test]
    fn test_full_trace_structure() {
        // Build a complete trace
        let raw = json!({"hook_event_name": "PreToolUse", "tool_name": "Bash"});
        let mut span = CupcakeSpan::new(raw, HarnessType::ClaudeCode, "trace-full".into());

        // Add enrich phase
        span.set_enrich(EnrichPhase::new(
            json!({"enriched": true}),
            vec!["whitespace_normalization".into()],
            100,
            span.span_id().to_string(),
            span.start_time_unix_nano,
        ));

        // Add policy phase
        let mut phase = PolicyPhase::new("project", span.span_id().to_string());

        // Add signals to phase
        let mut signals =
            SignalsPhase::new(phase.span_id().to_string(), phase.start_time_unix_nano);
        signals.add_signal(SignalExecution {
            name: "git_status".into(),
            command: "git status".into(),
            result: json!([]),
            duration_ms: Some(10),
            exit_code: Some(0),
        });
        signals.finalize(10);
        phase.set_signals(signals);

        // Add evaluation result
        phase
            .evaluation_mut()
            .record_routing(true, &["policy.test".into()]);
        phase
            .evaluation_mut()
            .record_exit("No policies matched - implicit allow");
        phase.finalize();

        span.add_phase(phase);
        span.finalize(Some(json!({"decision": "allow"})));

        // Verify structure
        assert!(span.enrich.is_some());
        assert_eq!(span.phases.len(), 1);
        assert!(span.phases[0].signals.is_some());
        assert_eq!(span.phases[0].signals.as_ref().unwrap().signals.len(), 1);
        assert!(span.response.is_some());

        // Verify serialization works
        let json = serde_json::to_string_pretty(&span).expect("serialize");
        assert!(json.contains("\"trace_id\": \"trace-full\""));
        assert!(json.contains("\"name\": \"project\""));
        assert!(json.contains("\"git_status\""));
    }
}
