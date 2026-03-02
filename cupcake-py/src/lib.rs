//! Python bindings for the Cupcake policy engine.
//!
//! This module provides PyO3 bindings that wrap the core BindingEngine,
//! exposing a Python-friendly API for policy evaluation.
//!
//! The design mirrors `cupcake-ts` (NAPI-RS bindings for Node.js):
//! - `PolicyEngine` class wraps `BindingEngine` from `cupcake-core`
//! - JSON in/out for maximum compatibility
//! - Both sync and async-compatible evaluation
//! - Thread-safe (`Send + Sync`) for use with Python's `ThreadPoolExecutor`

use cupcake_core::bindings::BindingEngine;
use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;

/// Native policy engine for evaluating Cupcake policies.
///
/// This class wraps the Rust `BindingEngine` and is intended to be used
/// through the pure-Python `cupcake.Cupcake` wrapper, though it can be
/// used directly for advanced use cases.
///
/// Thread Safety:
///   The engine is `Send + Sync` and can be shared across Python threads
///   (e.g. via `concurrent.futures.ThreadPoolExecutor`).
///
/// Example (direct usage)::
///
///     from cupcake._native import PolicyEngine
///     engine = PolicyEngine(".cupcake", "claude")
///     result_json = engine.evaluate('{"tool_name": "Bash", "command": "ls"}')
#[pyclass(name = "PolicyEngine")]
pub struct PyPolicyEngine {
    inner: BindingEngine,
}

#[pymethods]
impl PyPolicyEngine {
    /// Create a new PolicyEngine instance.
    ///
    /// Args:
    ///     path: Path to project directory or .cupcake folder.
    ///     harness: Harness type (``"claude"``, ``"cursor"``, ``"factory"``,
    ///              ``"opencode"``). Defaults to ``"claude"``.
    ///
    /// Raises:
    ///     RuntimeError: If the path doesn't exist, OPA binary is missing,
    ///                   or policy compilation fails.
    #[new]
    #[pyo3(signature = (path, harness=None))]
    fn new(path: String, harness: Option<String>) -> PyResult<Self> {
        let harness_str = harness.as_deref().unwrap_or("claude");
        let engine = BindingEngine::new(&path, harness_str)
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to initialize engine: {e}")))?;
        Ok(Self { inner: engine })
    }

    /// Evaluate a hook event against loaded policies.
    ///
    /// This method blocks the calling thread until evaluation completes.
    /// For async usage, run this in a thread pool via the Python wrapper's
    /// ``evaluate()`` async method.
    ///
    /// Args:
    ///     input_json: JSON string containing the hook event.
    ///
    /// Returns:
    ///     JSON string with the policy decision.
    ///
    /// Raises:
    ///     ValueError: If ``input_json`` is not valid JSON.
    ///     RuntimeError: If policy evaluation fails.
    fn evaluate(&self, input_json: &str) -> PyResult<String> {
        self.inner.evaluate_sync(input_json).map_err(|e| {
            if e.contains("Invalid input JSON") {
                PyValueError::new_err(e)
            } else {
                PyRuntimeError::new_err(e)
            }
        })
    }

    /// Return the Cupcake engine version string.
    fn version(&self) -> String {
        self.inner.version()
    }

    /// Return whether the engine is ready to evaluate policies.
    fn is_ready(&self) -> bool {
        self.inner.is_ready()
    }

    fn __repr__(&self) -> String {
        format!(
            "PolicyEngine(version={:?}, ready={})",
            self.inner.version(),
            self.inner.is_ready()
        )
    }
}

/// cupcake._native — low-level Rust bindings for the Cupcake policy engine.
///
/// This module is not intended for direct use. Import ``cupcake`` instead.
#[pymodule]
#[pyo3(name = "_native")]
fn cupcake_native(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyPolicyEngine>()?;
    Ok(())
}
