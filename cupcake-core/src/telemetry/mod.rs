//! Telemetry capture for cupcake event lifecycle.
//!
//! ## Trace Hierarchy
//!
//! ```text
//! CupcakeSpan (root)
//!   ├── enrich: EnrichPhase (preprocessing)
//!   └── phases: Vec<PolicyPhase> (one per policy layer)
//!         ├── signals: SignalsPhase (external programs)
//!         └── evaluation: EvaluationResult (WASM → decision)
//! ```
//!
//! ## Key Features
//!
//! - **Drop guard**: Telemetry written even on panic/early return
//! - **OTLP-compatible**: Span fields follow OpenTelemetry spec
//! - **Signals as first-class**: External program execution tracked separately

pub mod context;
pub mod span;
pub mod writer;

pub use context::TelemetryContext;
pub use span::{
    CupcakeSpan, EnrichPhase, EvaluationResult, PolicyPhase, SignalExecution, SignalsPhase,
};
pub use writer::TelemetryWriter;
