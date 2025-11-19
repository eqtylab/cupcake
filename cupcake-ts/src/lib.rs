//! TypeScript/Node.js bindings for Cupcake policy engine
//!
//! This module provides NAPI-RS bindings that wrap the core BindingEngine,
//! exposing a JavaScript-friendly API for policy evaluation in Node.js applications.

use cupcake_core::bindings::BindingEngine;
use napi::bindgen_prelude::*;
use napi_derive::napi;

/// PolicyEngine class for evaluating policies in Node.js
///
/// # Thread Safety
/// This class is thread-safe and can be used concurrently from multiple
/// JavaScript promises/async functions.
///
/// # Performance
/// - `evaluate_sync`: Blocks the event loop. Use only in CLI scripts or startup code.
/// - `evaluate_async`: Non-blocking, runs on libuv threadpool. Recommended for servers.
#[napi]
pub struct PolicyEngine {
    inner: BindingEngine,
}

#[napi]
impl PolicyEngine {
    /// Create a new PolicyEngine instance
    ///
    /// # Arguments
    /// * `path` - Path to project directory or .cupcake folder
    /// * `harness` - Optional harness type ('claude' or 'cursor'). Defaults to 'claude'.
    ///
    /// # Errors
    /// Returns error if:
    /// - Path doesn't exist or isn't a valid Cupcake project
    /// - OPA binary not found (install with `npx @eqtylab/cupcake install-opa`)
    /// - Policy compilation fails
    ///
    /// # Example
    /// ```javascript
    /// const engine = new PolicyEngine('.cupcake', 'claude');
    /// ```
    #[napi(constructor)]
    pub fn new(path: String, harness: Option<String>) -> Result<Self> {
        let harness_str = harness.as_deref().unwrap_or("claude");
        let engine = BindingEngine::new(&path, harness_str)
            .map_err(|e| Error::new(Status::GenericFailure, format!("Failed to initialize engine: {}", e)))?;

        Ok(Self { inner: engine })
    }

    /// Synchronously evaluate a hook event (BLOCKS event loop)
    ///
    /// ⚠️  WARNING: This method blocks the Node.js event loop until evaluation completes.
    /// Only use this in:
    /// - CLI scripts where blocking is acceptable
    /// - Startup/initialization code
    /// - Simple one-off evaluations
    ///
    /// For web servers or long-running processes, use `evaluateAsync` instead.
    ///
    /// # Arguments
    /// * `input` - JSON string representing the hook event
    ///
    /// # Returns
    /// JSON string with the policy decision
    ///
    /// # Example
    /// ```javascript
    /// const decision = engine.evaluateSync(JSON.stringify({
    ///   kind: 'shell',
    ///   command: 'git push'
    /// }));
    /// ```
    #[napi(js_name = "evaluateSync")]
    pub fn evaluate_sync(&self, input: String) -> Result<String> {
        self.inner
            .evaluate_sync(&input)
            .map_err(|e| {
                if e.contains("Invalid input JSON") {
                    Error::new(Status::InvalidArg, e)
                } else {
                    Error::new(Status::GenericFailure, e)
                }
            })
    }

    /// Asynchronously evaluate a hook event (RECOMMENDED, non-blocking)
    ///
    /// This method runs the evaluation on libuv's worker thread pool,
    /// allowing the event loop to handle other requests while evaluation runs.
    ///
    /// # Arguments
    /// * `input` - JSON string representing the hook event
    ///
    /// # Returns
    /// Promise<String> - Resolves to JSON string with the policy decision
    ///
    /// # Example
    /// ```javascript
    /// const decision = await engine.evaluateAsync(JSON.stringify({
    ///   kind: 'shell',
    ///   command: 'git push'
    /// }));
    /// ```
    #[napi(js_name = "evaluateAsync")]
    pub fn evaluate_async(&self, input: String) -> AsyncTask<EvaluateTask> {
        AsyncTask::new(EvaluateTask {
            engine: self.inner.clone(),
            input,
        })
    }

    /// Get the Cupcake version string
    #[napi]
    pub fn version(&self) -> String {
        self.inner.version()
    }

    /// Check if the engine is ready to evaluate policies
    #[napi(js_name = "isReady")]
    pub fn is_ready(&self) -> bool {
        self.inner.is_ready()
    }
}

/// Background task for async evaluation
///
/// This task runs on libuv's worker thread pool, keeping the event loop free.
pub struct EvaluateTask {
    engine: BindingEngine,
    input: String,
}

#[napi]
impl Task for EvaluateTask {
    type Output = String;
    type JsValue = String;

    /// Compute runs on a background thread
    fn compute(&mut self) -> Result<Self::Output> {
        self.engine.evaluate_sync(&self.input).map_err(|e| {
            if e.contains("Invalid input JSON") {
                Error::new(Status::InvalidArg, e)
            } else {
                Error::new(Status::GenericFailure, e)
            }
        })
    }

    /// Resolve runs on the main thread to convert to JS value
    fn resolve(&mut self, _env: Env, output: Self::Output) -> Result<Self::JsValue> {
        Ok(output)
    }
}

/// Thread safety verification
/// This const block ensures the BindingEngine is Send + Sync at compile time
const _: () = {
    fn assert_send<T: Send>() {}
    fn assert_sync<T: Sync>() {}

    fn _assertions() {
        assert_send::<PolicyEngine>();
        assert_sync::<PolicyEngine>();
    }
};
