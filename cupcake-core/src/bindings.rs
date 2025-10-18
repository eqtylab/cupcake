//! FFI Binding Engine - Thread-safe abstraction for foreign language bindings
//!
//! This module provides a simplified, FFI-friendly interface to the Cupcake engine
//! designed for safe use from Python, Node.js, and other language bindings.
//!
//! Key Design Principles:
//! - Thread-safe by default (Arc<Engine>)
//! - String-based error handling (no complex Rust types across FFI)
//! - JSON in/out for maximum compatibility
//! - Both sync and async evaluation methods
//! - Single-threaded Tokio runtime for FFI compatibility

use crate::engine::Engine;
use anyhow::Result;
use serde_json::Value;
use std::sync::Arc;

/// FFI-friendly engine wrapper for foreign language bindings
///
/// This struct is designed to be:
/// - Cloneable (for multiple references)
/// - Thread-safe (Arc internally)
/// - FFI-compatible (simple methods, string errors)
#[derive(Clone)]
pub struct BindingEngine {
    /// The core engine wrapped in Arc for thread safety
    inner: Arc<Engine>,

    /// Dedicated runtime for this binding instance
    /// Uses current_thread for FFI compatibility (avoids thread-local issues)
    runtime: Arc<tokio::runtime::Runtime>,
}

impl BindingEngine {
    /// Create a new binding engine with the given project path and harness type
    ///
    /// # Arguments
    /// * `path` - Path to the project directory or .cupcake folder
    /// * `harness` - The AI coding agent harness type (e.g., "claude" or "cursor")
    ///
    /// # Returns
    /// * `Ok(BindingEngine)` - Successfully initialized engine
    /// * `Err(String)` - Error message suitable for FFI
    pub fn new(path: &str, harness: &str) -> Result<Self, String> {
        // Parse harness string
        let harness_type: crate::harness::types::HarnessType = harness
            .parse()
            .map_err(|e| format!("Invalid harness type '{harness}': {e}"))?;

        // Create a current_thread runtime for FFI compatibility
        // This avoids thread-local storage issues with multi-threaded runtime
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| format!("Failed to create Tokio runtime: {e}"))?;

        // Initialize the core engine using the runtime
        let engine = runtime
            .block_on(Engine::new(path, harness_type))
            .map_err(|e| format!("Failed to initialize core engine: {e}"))?;

        Ok(Self {
            inner: Arc::new(engine),
            runtime: Arc::new(runtime),
        })
    }

    /// Synchronous evaluation method for blocking language bindings
    ///
    /// This method is ideal for:
    /// - Python with GIL released (py.allow_threads)
    /// - Synchronous Node.js calls
    /// - Any FFI that expects blocking behavior
    ///
    /// # Arguments
    /// * `input_json` - JSON string containing the hook event
    ///
    /// # Returns
    /// * `Ok(String)` - JSON response with decision
    /// * `Err(String)` - Error message
    pub fn evaluate_sync(&self, input_json: &str) -> Result<String, String> {
        // Parse input JSON
        let input: Value =
            serde_json::from_str(input_json).map_err(|e| format!("Invalid input JSON: {e}"))?;

        // Evaluate using the runtime (blocks until complete)
        let decision = self
            .runtime
            .block_on(self.inner.evaluate(&input, None))
            .map_err(|e| format!("Core engine evaluation failed: {e}"))?;

        // Serialize the decision to JSON
        serde_json::to_string(&decision)
            .map_err(|e| format!("Failed to serialize final decision: {e}"))
    }

    /// Asynchronous evaluation method for async language bindings
    ///
    /// This method is ideal for:
    /// - Python with asyncio
    /// - Node.js with async/await
    /// - Any FFI that supports async operations
    ///
    /// # Arguments
    /// * `input_json` - JSON string containing the hook event
    ///
    /// # Returns
    /// * `Ok(String)` - JSON response with decision
    /// * `Err(String)` - Error message
    pub async fn evaluate_async(&self, input_json: &str) -> Result<String, String> {
        // Parse input JSON
        let input: Value =
            serde_json::from_str(input_json).map_err(|e| format!("Invalid input JSON: {e}"))?;

        // Evaluate asynchronously
        let decision = self
            .inner
            .evaluate(&input, None)
            .await
            .map_err(|e| format!("Core engine evaluation failed: {e}"))?;

        // Serialize the decision to JSON
        serde_json::to_string(&decision)
            .map_err(|e| format!("Failed to serialize final decision: {e}"))
    }

    /// Get engine version information
    ///
    /// Useful for debugging and compatibility checks
    pub fn version(&self) -> String {
        format!("cupcake-core {}", env!("CARGO_PKG_VERSION"))
    }

    /// Check if the engine is properly initialized
    ///
    /// Can be used for health checks from bindings
    pub fn is_ready(&self) -> bool {
        // In the future, we could add more sophisticated health checks
        true
    }
}

// Compile-time thread safety verification
//
// These assertions ensure BindingEngine can be safely shared between threads.
// If BindingEngine ever loses Send or Sync, compilation will fail immediately.
//
// This pattern works by type-checking the body of _assertions() at compile time.
// The function is never called (hence #[allow(dead_code)]), but the compiler must
// still verify that BindingEngine: Send + Sync when type-checking the function body.
// This gives us zero-cost compile-time trait verification.
const _: () = {
    #[allow(dead_code)] // Intentionally uncalled - exists only for type checking
    fn assert_send<T: Send>() {}
    #[allow(dead_code)] // Intentionally uncalled - exists only for type checking
    fn assert_sync<T: Sync>() {}

    #[allow(dead_code)] // Never executed - compiler type-checks this at compile time
    fn _assertions() {
        assert_send::<BindingEngine>();
        assert_sync::<BindingEngine>();
    }
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_binding_engine_creation() {
        // This will fail without proper test policies, but validates compilation
        let result = BindingEngine::new("test_path", "claude");
        assert!(result.is_err()); // Expected to fail without valid project
    }

    #[test]
    fn test_version() {
        // Version should always work even without initialized engine
        // But we need a valid engine first
        // This is more of a compilation test
    }
}
