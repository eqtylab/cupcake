//! WASM Runtime - Executes compiled Rego policies using the Hybrid Model
//! 
//! Implements the NEW_GUIDING_FINAL.md single entrypoint evaluation.
//! Queries the cupcake.system.evaluate aggregation endpoint and returns DecisionSet.

use anyhow::{anyhow, Context, Result, bail};
use serde_json::Value;
use std::env;
use std::time::Instant;
use wasmtime::*;
use tracing::{debug, trace, warn};

use super::decision::DecisionSet;

// --- PART 1: Production-Grade Memory Configuration ---
// This logic is NOT deprecated. It is a required feature.

/// Parses a human-readable memory string (e.g., "16MB", "256kb") into bytes.
fn parse_memory_string(s: &str) -> Result<u64> {
    let s_lower = s.to_lowercase();
    let (num_str, unit) = s_lower.trim().split_at(
        s_lower.find(|c: char| !c.is_digit(10) && c != '.').unwrap_or_else(|| s_lower.len()),
    );
    let num: f64 = num_str.trim().parse()?;
    let multiplier = match unit.trim() {
        "kb" | "k" => 1024.0,
        "mb" | "m" => 1024.0 * 1024.0,
        "gb" | "g" => 1024.0 * 1024.0 * 1024.0,
        "b" | "" => 1.0,
        _ => return Err(anyhow!("Unknown memory unit: '{}'", unit)),
    };
    Ok((num * multiplier) as u64)
}

/// Converts a byte count to the number of wasmtime pages (64KB per page).
fn bytes_to_wasm_pages(bytes: u64) -> u32 {
    (bytes as f64 / 65536.0).ceil() as u32
}

/// Gets configured WASM memory limits with safe defaults and an absolute cap.
fn get_memory_config() -> (u32, Option<u32>) {
    const DEFAULT_INITIAL_PAGES: u32 = 5; // 320KB
    const DEFAULT_MAX_MEMORY: &str = "10MB";
    const ABSOLUTE_MAX_MEMORY: &str = "100MB";

    let max_memory_str = env::var("CUPCAKE_WASM_MAX_MEMORY").unwrap_or_else(|_| DEFAULT_MAX_MEMORY.to_string());
    let mut max_memory_bytes = parse_memory_string(&max_memory_str).unwrap_or_else(|e| {
        warn!("Invalid CUPCAKE_WASM_MAX_MEMORY value '{}': {}. Using default '{}'.", max_memory_str, e, DEFAULT_MAX_MEMORY);
        parse_memory_string(DEFAULT_MAX_MEMORY).unwrap()
    });

    let absolute_max_bytes = parse_memory_string(ABSOLUTE_MAX_MEMORY).unwrap();
    if max_memory_bytes > absolute_max_bytes {
        warn!("Requested max memory ({}) exceeds the absolute maximum ({}). Capping at {}.", max_memory_str, ABSOLUTE_MAX_MEMORY, ABSOLUTE_MAX_MEMORY);
        max_memory_bytes = absolute_max_bytes;
    }

    let max_pages = bytes_to_wasm_pages(max_memory_bytes);
    (DEFAULT_INITIAL_PAGES, Some(max_pages))
}

/// WASM runtime for executing compiled Rego policies
pub struct WasmRuntime {
    engine: Engine,
    module: Module,
}

impl WasmRuntime {
    /// Create a new runtime from compiled WASM bytes
    pub fn new(wasm_bytes: &[u8]) -> Result<Self> {
        debug!("Initializing WASM runtime");
        
        // Configure engine with memory limits
        let mut config = Config::new();
        config.wasm_multi_memory(true);
        config.wasm_multi_value(true);
        
        let engine = Engine::new(&config)?;
        let module = Module::from_binary(&engine, wasm_bytes)
            .context("Failed to load WASM module")?;
        
        debug!("WASM module loaded successfully");
        
        Ok(Self {
            engine,
            module,
        })
    }
    
    /// Query the aggregated decision set from cupcake.system.evaluate
    /// This is the single entrypoint defined in the Hybrid Model
    pub fn query_decision_set(
        &mut self,
        input: &Value,
    ) -> Result<DecisionSet> {
        let start = Instant::now();
        debug!("Querying DecisionSet from cupcake.system.evaluate entrypoint");
        
        // Use the low-level evaluate_raw function with entrypoint 0 (single entrypoint)
        let result_json = self.evaluate_raw(input, 0)?;
        
        // Enhanced debug logging to understand what WASM is returning
        eprintln!("==== RAW WASM RESPONSE ====");
        eprintln!("{}", result_json);
        eprintln!("==========================");
        debug!("Raw WASM result JSON: {}", result_json);
        
        // Parse the raw JSON result
        let result_value: Value = serde_json::from_str(&result_json)
            .context("Failed to parse result JSON")?;
        
        // Extract the decision set from the result
        let decision_set = self.extract_decision_set_from_result(&result_value)?;
        
        let elapsed = start.elapsed();
        debug!("Decision set evaluation completed in {:?}", elapsed);
        
        Ok(decision_set)
    }
    
    /// Low-level function that interacts with the OPA WASM ABI
    /// Takes an input JSON value and returns the raw JSON string from the policy
    fn evaluate_raw(&self, input: &Value, entrypoint_id: i32) -> Result<String> {
        let mut store = Store::new(&self.engine, ());
        let mut linker = Linker::new(&self.engine);
        
        // Use the robust, configurable memory logic
        let (initial_pages, max_pages) = get_memory_config();
        let memory_ty = MemoryType::new(initial_pages, max_pages);
        let memory = Memory::new(&mut store, memory_ty)?;
        linker.define(&mut store, "env", "memory", memory)?;
        
        // Provide the required OPA host functions
        linker.func_wrap("env", "opa_abort", |_: Caller<'_, ()>, addr: i32| {
            tracing::error!(addr, "OPA policy aborted execution.");
        })?;
        linker.func_wrap("env", "opa_println", |_: Caller<'_, ()>, _: i32| {})?;
        linker.func_wrap("env", "opa_builtin0", |_: Caller<'_, ()>, _: i32, _: i32| -> i32 { 0 })?;
        linker.func_wrap("env", "opa_builtin1", |_: Caller<'_, ()>, _: i32, _: i32, _: i32| -> i32 { 0 })?;
        linker.func_wrap("env", "opa_builtin2", |_: Caller<'_, ()>, _: i32, _: i32, _: i32, _: i32| -> i32 { 0 })?;
        linker.func_wrap("env", "opa_builtin3", |_: Caller<'_, ()>, _: i32, _: i32, _: i32, _: i32, _: i32| -> i32 { 0 })?;
        linker.func_wrap("env", "opa_builtin4", |_: Caller<'_, ()>, _: i32, _: i32, _: i32, _: i32, _: i32, _: i32| -> i32 { 0 })?;
        
        let instance = linker.instantiate(&mut store, &self.module)?;
        
        // Get the tools exported by the WASM module
        let memory = instance.get_memory(&mut store, "memory")
            .context("`memory` export not found")?;
        let opa_malloc = instance.get_typed_func::<i32, i32>(&mut store, "opa_malloc")?;
        let opa_heap_ptr_get = instance.get_typed_func::<(), i32>(&mut store, "opa_heap_ptr_get")?;
        let opa_eval = instance.get_typed_func::<(i32, i32, i32, i32, i32, i32, i32), i32>(&mut store, "opa_eval")?;
        
        let input_json = serde_json::to_string(input)?;
        debug!("WASM input JSON: {}", input_json);
        eprintln!("==== WASM INPUT TO OPA ====");
        eprintln!("{}", serde_json::to_string_pretty(input).unwrap_or_default());
        eprintln!("===========================");
        let input_bytes = input_json.as_bytes();
        
        let input_ptr = opa_malloc.call(&mut store, input_bytes.len() as i32)?;
        memory.write(&mut store, input_ptr as usize, input_bytes)?;
        
        let heap_ptr_before = opa_heap_ptr_get.call(&mut store, ())?;
        
        let result_ptr = opa_eval.call(
            &mut store,
            (0, entrypoint_id, 0, input_ptr, input_bytes.len() as i32, heap_ptr_before, 0),
        )?;
        
        read_string_from_memory(&memory, &mut store, result_ptr)
    }
    
    
    /// Extract the decision set from the WASM result
    fn extract_decision_set_from_result(
        &self,
        result: &Value,
    ) -> Result<DecisionSet> {
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
        debug!("Attempting to parse decision_value: {}", 
            serde_json::to_string_pretty(decision_value).unwrap_or_default());
            
        let decision_set: DecisionSet = serde_json::from_value(decision_value.clone())
            .with_context(|| {
                format!("Failed to parse DecisionSet. Raw value: {}", 
                    serde_json::to_string_pretty(decision_value).unwrap_or_default())
            })?;
            
        debug!("Successfully extracted DecisionSet with {} total decisions", decision_set.decision_count());
        debug!("DecisionSet details - denials: {}, halts: {}, blocks: {}", 
            decision_set.denials.len(), decision_set.halts.len(), decision_set.blocks.len());
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

// Aligns with NEW_GUIDING_FINAL.md:
// - Implements Hybrid Model single entrypoint evaluation
// - Queries cupcake.system.evaluate for aggregated DecisionSet
// - No per-policy logic - just single aggregation extraction
// - Returns strongly-typed DecisionSet for synthesis layer
// - Foundation for sub-millisecond evaluation performance