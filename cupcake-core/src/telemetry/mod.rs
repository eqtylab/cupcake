//! Hierarchical telemetry capture for complete event lifecycle tracking.
//!
//! This module provides structured telemetry that captures every agent event
//! at three stages:
//! 1. **Ingest** - Raw event before any preprocessing
//! 2. **Enrich** - Event after preprocessing (symlink resolution, whitespace normalization)
//! 3. **Evaluate** - Policy evaluation results (may not exist for early exits)
//!
//! ## Architecture
//!
//! ```text
//! cupcake.ingest (ROOT - ALWAYS created)
//!   ├── raw_event: Value
//!   ├── trace_id, timestamp, harness
//!   │
//!   ├── cupcake.enrich (CHILD)
//!   │     ├── enriched_event: Value
//!   │     └── preprocessing_operations: Vec<String>
//!   │
//!   └── cupcake.evaluate (CHILD - per phase, may not exist)
//!         ├── phase: "global" | "catalog:name" | "project"
//!         ├── routed, matched_policies
//!         ├── decision, exit_reason
//!         └── signals, wasm_decision_set
//! ```
//!
//! ## Key Features
//!
//! - **No early exit blindness**: Root span created in CLI before preprocessing
//! - **Drop guard**: Telemetry written even on panic/early return
//! - **OTLP-ready**: Span tree maps directly to OpenTelemetry format
//!
//! ## Implementation Log
//!
//! - 2025-12-XX: Initial implementation - span types, context, file writer

pub mod context;
pub mod span;
pub mod writer;

pub use context::TelemetryContext;
pub use span::{EnrichSpan, EvaluateSpan, IngestSpan};
pub use writer::TelemetryWriter;
