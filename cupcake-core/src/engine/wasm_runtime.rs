//! WASM Runtime - Executes compiled Rego policies.
//!
//! Queries the `cupcake.system.evaluate` aggregation endpoint and returns a [`DecisionSet`].

use anyhow::{Context, Result};
use serde_json::Value;
use std::time::Instant;
use tracing::{debug, instrument, trace, warn};
use wasmtime::*;

use super::decision::DecisionSet;

// --- PART 1: Production-Grade Memory Configuration ---
// This logic is NOT deprecated. It is a required feature.

/// Converts a byte count to the number of wasmtime pages (64KB per page).
fn bytes_to_wasm_pages(bytes: u64) -> u32 {
    (bytes as f64 / 65536.0).ceil() as u32
}

/// Gets configured WASM memory limits with safe defaults and an absolute cap.
///
/// Accepts an optional max_memory_bytes parameter from CLI flag.
/// If None, uses the default of 10MB.
fn get_memory_config(max_memory_bytes_override: Option<usize>) -> (u32, Option<u32>) {
    const DEFAULT_INITIAL_PAGES: u32 = 5; // 320KB
    const DEFAULT_MAX_MEMORY_BYTES: u64 = 10 * 1024 * 1024; // 10MB
    const ABSOLUTE_MIN_MEMORY_BYTES: u64 = 1024 * 1024; // 1MB - defense in depth
    const ABSOLUTE_MAX_MEMORY_BYTES: u64 = 100 * 1024 * 1024; // 100MB

    // Use override from CLI or default
    let mut max_memory_bytes = max_memory_bytes_override
        .map(|b| b as u64)
        .unwrap_or(DEFAULT_MAX_MEMORY_BYTES);

    // Enforce minimum (defense in depth - already checked in CLI parsing)
    if max_memory_bytes < ABSOLUTE_MIN_MEMORY_BYTES {
        warn!(
            "Requested max memory ({} bytes) is below minimum ({}). Using minimum.",
            max_memory_bytes, ABSOLUTE_MIN_MEMORY_BYTES
        );
        max_memory_bytes = ABSOLUTE_MIN_MEMORY_BYTES;
    }

    // Enforce maximum cap
    if max_memory_bytes > ABSOLUTE_MAX_MEMORY_BYTES {
        warn!(
            "Requested max memory ({} bytes) exceeds absolute maximum ({}). Capping at maximum.",
            max_memory_bytes, ABSOLUTE_MAX_MEMORY_BYTES
        );
        max_memory_bytes = ABSOLUTE_MAX_MEMORY_BYTES;
    }

    let max_pages = bytes_to_wasm_pages(max_memory_bytes);
    (DEFAULT_INITIAL_PAGES, Some(max_pages))
}

/// WASM runtime for executing compiled Rego policies
pub struct WasmRuntime {
    engine: Engine,
    module: Module,
    /// The namespace for this runtime (e.g., "cupcake.system" or "cupcake.global.system")
    namespace: String,
    /// Optional max memory override from CLI (in bytes)
    max_memory_bytes: Option<usize>,
}

impl WasmRuntime {
    /// Create a new runtime from compiled WASM bytes with default namespace
    pub fn new(wasm_bytes: &[u8]) -> Result<Self> {
        Self::new_with_config(wasm_bytes, "cupcake.system", None)
    }

    /// Create a new runtime from compiled WASM bytes with specific namespace.
    /// Uses the default memory configuration (no override).
    pub fn new_with_namespace(wasm_bytes: &[u8], namespace: &str) -> Result<Self> {
        Self::new_with_config(wasm_bytes, namespace, None)
    }

    /// Create a new runtime from compiled WASM bytes with namespace and memory config
    pub fn new_with_config(
        wasm_bytes: &[u8],
        namespace: &str,
        max_memory_bytes: Option<usize>,
    ) -> Result<Self> {
        debug!("Initializing WASM runtime");

        // Configure engine with memory limits
        let mut config = Config::new();
        config.wasm_multi_memory(true);
        config.wasm_multi_value(true);

        let engine = Engine::new(&config)?;
        let module =
            Module::from_binary(&engine, wasm_bytes).context("Failed to load WASM module")?;

        debug!("WASM module loaded successfully");

        Ok(Self {
            engine,
            module,
            namespace: namespace.to_string(),
            max_memory_bytes,
        })
    }

    /// Query the aggregated decision set from cupcake.system.evaluate
    /// This is the single entrypoint defined in the Hybrid Model
    /// Thread-safe: creates fresh Store per evaluation
    #[instrument(
        name = "wasm_evaluate",
        skip(self, input),
        fields(
            input_size_bytes = input.to_string().len(),
            output_size_bytes = tracing::field::Empty,
            decision_count = tracing::field::Empty,
            evaluation_time_ms = tracing::field::Empty
        )
    )]
    pub fn query_decision_set(&self, input: &Value) -> Result<DecisionSet> {
        let start = Instant::now();
        debug!(
            "Querying DecisionSet from {}.evaluate entrypoint",
            self.namespace
        );

        // Use the low-level evaluate_raw function with entrypoint 0 (single entrypoint)
        let result_json = self.evaluate_raw(input, 0)?;

        debug!("Raw WASM result JSON: {}", result_json);

        // Parse the raw JSON result
        let result_value: Value =
            serde_json::from_str(&result_json).context("Failed to parse result JSON")?;

        // Extract the decision set from the result
        let decision_set = self.extract_decision_set_from_result(&result_value)?;

        let elapsed = start.elapsed();

        // Record span fields
        let current_span = tracing::Span::current();
        current_span.record("output_size_bytes", result_json.len());
        current_span.record("decision_count", decision_set.decision_count());
        current_span.record("evaluation_time_ms", elapsed.as_millis());

        debug!("Decision set evaluation completed in {:?}", elapsed);
        trace!(
            decisions = decision_set.decision_count(),
            duration_ms = elapsed.as_millis(),
            "WASM evaluation complete"
        );

        Ok(decision_set)
    }

    /// Low-level function that interacts with the OPA WASM ABI
    /// Takes an input JSON value and returns the raw JSON string from the policy
    fn evaluate_raw(&self, input: &Value, entrypoint_id: i32) -> Result<String> {
        let mut store = Store::new(&self.engine, ());
        let mut linker = Linker::new(&self.engine);

        // Use the robust, configurable memory logic with CLI override
        let (initial_pages, max_pages) = get_memory_config(self.max_memory_bytes);
        let memory_ty = MemoryType::new(initial_pages, max_pages);
        let memory = Memory::new(&mut store, memory_ty)?;
        linker.define(&mut store, "env", "memory", memory)?;

        // Provide the required OPA host functions
        linker.func_wrap("env", "opa_abort", |_: Caller<'_, ()>, addr: i32| {
            tracing::error!(addr, "OPA policy aborted execution.");
        })?;
        linker.func_wrap("env", "opa_println", |_: Caller<'_, ()>, _: i32| {})?;
        linker.func_wrap(
            "env",
            "opa_builtin0",
            |_: Caller<'_, ()>, _: i32, _: i32| -> i32 { 0 },
        )?;
        linker.func_wrap(
            "env",
            "opa_builtin1",
            |_: Caller<'_, ()>, _: i32, _: i32, _: i32| -> i32 { 0 },
        )?;
        linker.func_wrap(
            "env",
            "opa_builtin2",
            |_: Caller<'_, ()>, _: i32, _: i32, _: i32, _: i32| -> i32 { 0 },
        )?;
        linker.func_wrap(
            "env",
            "opa_builtin3",
            |_: Caller<'_, ()>, _: i32, _: i32, _: i32, _: i32, _: i32| -> i32 { 0 },
        )?;
        linker.func_wrap(
            "env",
            "opa_builtin4",
            |_: Caller<'_, ()>, _: i32, _: i32, _: i32, _: i32, _: i32, _: i32| -> i32 { 0 },
        )?;

        let instance = linker.instantiate(&mut store, &self.module)?;

        // Get the tools exported by the WASM module
        let memory = instance
            .get_memory(&mut store, "memory")
            .context("`memory` export not found")?;
        let opa_malloc = instance.get_typed_func::<i32, i32>(&mut store, "opa_malloc")?;
        let opa_heap_ptr_get =
            instance.get_typed_func::<(), i32>(&mut store, "opa_heap_ptr_get")?;
        let opa_eval = instance
            .get_typed_func::<(i32, i32, i32, i32, i32, i32, i32), i32>(&mut store, "opa_eval")?;

        let input_json = serde_json::to_string(input)?;
        debug!("WASM input JSON: {}", input_json);
        let input_bytes = input_json.as_bytes();

        let input_ptr = opa_malloc.call(&mut store, input_bytes.len() as i32)?;
        memory.write(&mut store, input_ptr as usize, input_bytes)?;

        let heap_ptr_before = opa_heap_ptr_get.call(&mut store, ())?;

        let result_ptr = opa_eval.call(
            &mut store,
            (
                0,
                entrypoint_id,
                0,
                input_ptr,
                input_bytes.len() as i32,
                heap_ptr_before,
                0,
            ),
        )?;

        read_string_from_memory(&memory, &mut store, result_ptr)
    }

    /// Extract the decision set from the WASM result
    fn extract_decision_set_from_result(&self, result: &Value) -> Result<DecisionSet> {
        debug!("Extracting DecisionSet from cupcake.system.evaluate result");
        debug!("Raw result structure: {:?}", result);

        // The OPA eval result format can be either:
        // 1. An array with a single object: [{"result": <decision_set>}]
        // 2. Direct decision set object: {"denials": [...], "halts": [...]}

        let decision_value = if let Some(result_array) = result.as_array() {
            if result_array.is_empty() {
                // No result means undefined - return empty decision set
                debug!("Empty result array, returning default DecisionSet");
                return Ok(DecisionSet::default());
            }

            // Check if it's wrapped in {"result": <decision_set>}
            let first_element = &result_array[0];
            if let Some(wrapper) = first_element.as_object() {
                if let Some(result_field) = wrapper.get("result") {
                    debug!("Found wrapped result format");
                    result_field
                } else {
                    debug!("Array element is not wrapped, using directly");
                    first_element
                }
            } else {
                debug!("Array element is not an object");
                first_element
            }
        } else {
            // Direct decision set object
            debug!("Found direct decision set format");
            result
        };

        // Parse the result as a DecisionSet
        debug!(
            "Attempting to parse decision_value: {}",
            serde_json::to_string_pretty(decision_value).unwrap_or_default()
        );

        let decision_set: DecisionSet = serde_json::from_value(decision_value.clone())
            .with_context(|| {
                format!(
                    "Failed to parse DecisionSet. Raw value: {}",
                    serde_json::to_string_pretty(decision_value).unwrap_or_default()
                )
            })?;

        debug!(
            "Successfully extracted DecisionSet with {} total decisions",
            decision_set.decision_count()
        );
        debug!(
            "DecisionSet details - denials: {}, halts: {}, blocks: {}",
            decision_set.denials.len(),
            decision_set.halts.len(),
            decision_set.blocks.len()
        );
        Ok(decision_set)
    }
}

/// Helper function to read a null-terminated string from WASM memory
fn read_string_from_memory(memory: &Memory, store: &mut Store<()>, ptr: i32) -> Result<String> {
    let mut buffer = Vec::new();
    let mut offset = ptr as usize;
    loop {
        let mut byte = [0u8];
        memory.read(&mut *store, offset, &mut byte)?;
        if byte[0] == 0 {
            break;
        }
        buffer.push(byte[0]);
        offset += 1;
    }
    Ok(String::from_utf8(buffer)?)
}

