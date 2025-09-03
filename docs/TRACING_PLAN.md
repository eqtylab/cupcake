# Policy Evaluation Tracing Implementation Plan

## Overview

Implement elegant, performant tracing for Cupcake policy evaluation that provides deep visibility into the evaluation flow while maintaining production performance and following Rust observability best practices.

## Current State Analysis

- **Existing Infrastructure**: Cupcake already uses `tracing` and `tracing-subscriber` with `EnvFilter`
- **Current Logging**: Basic info/debug logs exist but lack structured spans and detailed evaluation flow
- **Performance Tracking**: Only WASM runtime has elapsed time measurement
- **No trace-level logging or structured spans currently implemented**

## Proposed Solution: Structured Span-Based Tracing

### 1. Environment Variable Control

- Use existing `RUST_LOG` environment variable (already supported)
- Add new `CUPCAKE_TRACE` environment variable for evaluation-specific tracing
- Example: `CUPCAKE_TRACE=eval` or `CUPCAKE_TRACE=eval,signals,wasm`

### 2. Structured Span Implementation

Add tracing spans using the `#[instrument]` attribute and manual spans:

```rust
// In engine/mod.rs
#[instrument(
    skip(self, input),
    fields(
        event_name = %event_name,
        tool_name = ?tool_name,
        trace_id = %generate_trace_id()
    )
)]
pub async fn evaluate(&self, input: &Value) -> Result<FinalDecision>
```

Key spans to add:
- **Evaluation Root Span**: Complete evaluation lifecycle
- **Routing Span**: Policy matching and routing decisions
- **Signal Gathering Span**: Signal execution with individual signal spans
- **WASM Evaluation Span**: Policy evaluation in WASM runtime
- **Synthesis Span**: Decision synthesis and prioritization
- **Action Execution Span**: Post-decision action execution

### 3. Structured Fields for Each Span

- **Evaluation**: `trace_id`, `event_name`, `tool_name`, `session_id`
- **Routing**: `matched_policy_count`, `routing_key`, `policies`
- **Signals**: `signal_name`, `exit_code`, `execution_time_ms`
- **WASM**: `input_size_bytes`, `output_size_bytes`, `evaluation_time_ms`
- **Synthesis**: `decision_type`, `decision_count`, `priority`
- **Actions**: `rule_id`, `action_type`, `execution_time_ms`

### 4. Trace Output Format

When `CUPCAKE_TRACE=eval` is set, output structured JSON lines to stderr:

```json
{"timestamp":"2024-09-03T12:00:00Z","level":"TRACE","span":"evaluate","trace_id":"abc123","event_name":"PreToolUse","tool_name":"Bash","matched_policies":1,"duration_ms":15}
{"timestamp":"2024-09-03T12:00:00.005Z","level":"TRACE","span":"signal","trace_id":"abc123","signal_name":"git_status","exit_code":0,"duration_ms":3}
{"timestamp":"2024-09-03T12:00:00.010Z","level":"TRACE","span":"wasm_eval","trace_id":"abc123","decision_count":2,"duration_ms":5}
{"timestamp":"2024-09-03T12:00:00.015Z","level":"TRACE","span":"synthesis","trace_id":"abc123","final_decision":"Allow","context_count":1}
```

### 5. Implementation Details

#### Phase 1: Core Tracing Infrastructure
1. Add `trace_id` generation using UUIDs
2. Update `tracing_subscriber` initialization to support JSON output when tracing enabled
3. Add custom `EvaluationTrace` layer for structured output

#### Phase 2: Span Instrumentation
1. Add `#[instrument]` to key async functions
2. Create manual spans for synchronous code blocks
3. Add structured fields to all spans

#### Phase 3: Performance Metrics
1. Add `Instant::now()` measurements to all major operations
2. Include timing data in span fields
3. Add memory usage tracking for WASM operations

#### Phase 4: Optional Enhancements
1. Add OpenTelemetry export support (optional feature flag)
2. Add trace sampling configuration
3. Add trace correlation with Claude Code session IDs

## Benefits

1. **Zero Performance Impact**: Tracing only active when explicitly enabled
2. **Industry Standard**: Uses established `tracing` ecosystem
3. **Non-Invasive**: Uses attributes and spans, minimal code changes
4. **Debugging Power**: Complete visibility into evaluation flow
5. **Production Ready**: Can be safely enabled in production for troubleshooting
6. **Integration Friendly**: JSON output can be consumed by observability tools

## Files to Modify

1. `cupcake-cli/src/main.rs` - Enhanced tracing subscriber setup
2. `cupcake-core/src/engine/mod.rs` - Main evaluation instrumentation
3. `cupcake-core/src/engine/wasm_runtime.rs` - WASM evaluation tracing
4. `cupcake-core/src/engine/synthesis.rs` - Decision synthesis tracing
5. `cupcake-core/src/engine/guidebook.rs` - Signal execution tracing
6. `Cargo.toml` - Add `tracing` features: `["json", "env-filter"]`

## Testing Strategy

1. Add integration test that enables tracing and verifies output structure
2. Benchmark to ensure no performance regression when tracing disabled
3. Test with various `CUPCAKE_TRACE` configurations

This approach provides powerful debugging capabilities while maintaining elegance, performance, and compatibility with industry-standard observability tools.