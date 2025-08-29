//! Python bindings for Cupcake engine
//! 
//! This module provides PyO3-based Python bindings for the Cupcake policy engine.
//! 
//! Critical Design Elements:
//! - MUST use py.allow_threads() to release GIL during evaluation
//! - Thread-safe via BindingEngine's Arc<Engine>
//! - Compatible with Python 3.9+ via abi3
//! - Supports both sync and async Python usage

use cupcake_core::bindings::BindingEngine;
use pyo3::prelude::*;
use pyo3::exceptions::{PyRuntimeError, PyValueError};

/// Python-accessible policy engine
/// 
/// This class wraps the Rust BindingEngine for use from Python.
/// It's designed to be used either directly or through the Python wrapper.
/// 
/// Example:
/// ```python
/// from cupcake.cupcake_native import PolicyEngine
/// 
/// engine = PolicyEngine(".cupcake")
/// result = engine.evaluate('{"hookEventName": "PreToolUse", "tool_name": "Bash"}')
/// ```
#[pyclass(name = "PolicyEngine")]
struct PyPolicyEngine {
    inner: BindingEngine,
}

#[pymethods]
impl PyPolicyEngine {
    /// Create a new PolicyEngine instance
    /// 
    /// Args:
    ///     path (str): Path to the project directory or .cupcake folder
    /// 
    /// Raises:
    ///     RuntimeError: If engine initialization fails
    #[new]
    fn new(path: String) -> PyResult<Self> {
        let engine = BindingEngine::new(&path)
            .map_err(|e| PyRuntimeError::new_err(e))?;
        Ok(Self { inner: engine })
    }
    
    /// Evaluate a hook event and return the decision
    /// 
    /// CRITICAL: This method releases the Python GIL using py.allow_threads()
    /// This allows other Python threads to run while Rust evaluates the policies,
    /// preventing the entire Python application from freezing.
    /// 
    /// Args:
    ///     input (str): JSON string containing the hook event
    /// 
    /// Returns:
    ///     str: JSON string containing the decision
    /// 
    /// Raises:
    ///     RuntimeError: If evaluation fails
    ///     ValueError: If input is not valid JSON
    fn evaluate(&self, input: String, py: Python) -> PyResult<String> {
        // CRITICAL: Release the GIL while evaluating
        // Without this, multi-threaded Python apps (like web servers) will freeze
        py.allow_threads(|| {
            self.inner.evaluate_sync(&input)
                .map_err(|e| {
                    if e.contains("Invalid input JSON") {
                        PyValueError::new_err(e)
                    } else {
                        PyRuntimeError::new_err(e)
                    }
                })
        })
    }
    
    /// Get the engine version
    /// 
    /// Returns:
    ///     str: Version string like "cupcake-core 0.1.0"
    fn version(&self) -> String {
        self.inner.version()
    }
    
    /// Check if the engine is ready
    /// 
    /// Returns:
    ///     bool: True if the engine is initialized and ready
    fn is_ready(&self) -> bool {
        self.inner.is_ready()
    }
    
    /// String representation for debugging
    fn __repr__(&self) -> String {
        format!("PolicyEngine(version='{}')", self.inner.version())
    }
}

/// Python module initialization
/// 
/// This function is called by Python when importing the module.
/// It registers the PolicyEngine class and any module-level functions.
#[pymodule]
#[pyo3(name = "cupcake_native")]
fn cupcake_py(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Add the PolicyEngine class
    m.add_class::<PyPolicyEngine>()?;
    
    // Add module-level version function
    m.add_function(wrap_pyfunction!(get_version, m)?)?;
    
    // Add module docstring
    m.add("__doc__", "Native Rust bindings for the Cupcake policy engine")?;
    
    Ok(())
}

/// Module-level version function
/// 
/// Returns the version of the native bindings
#[pyfunction]
fn get_version() -> String {
    format!("cupcake-py {}", env!("CARGO_PKG_VERSION"))
}

// Note on GIL and performance:
// 
// The py.allow_threads() call in evaluate() is CRITICAL for performance.
// Without it, the Python GIL remains held during the entire policy evaluation,
// which can take several milliseconds. This blocks ALL other Python threads.
// 
// In a web server with 100 concurrent requests, this would serialize all
// evaluations, turning a 5ms operation into a 500ms bottleneck.
// 
// With py.allow_threads(), multiple Python threads can call evaluate()
// concurrently, and the thread-safe Rust engine handles them in parallel.