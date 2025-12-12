//! Signal telemetry collection for engine evaluation.
//!
//! This module provides a minimal container for collecting signal execution
//! data during policy evaluation. The collected data is passed to the
//! telemetry system for output.

use crate::telemetry::span::SignalExecution;

/// Collects signal execution data during policy evaluation for telemetry.
///
/// This is a simple container used internally by the engine to accumulate
/// signal execution results, which are then recorded in telemetry spans.
#[derive(Debug, Clone, Default)]
pub struct SignalTelemetry {
    /// Signal execution results
    pub signals: Vec<SignalExecution>,
}

impl SignalTelemetry {
    /// Create a new empty signal telemetry container
    pub fn new() -> Self {
        Self::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signal_telemetry_creation() {
        let telemetry = SignalTelemetry::new();
        assert!(telemetry.signals.is_empty());
    }
}
