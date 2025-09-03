# Using Tracing to Debug Failing Tests

## Quick Start

When a test fails, enable tracing to see what's happening internally:

```bash
# Run specific test with full tracing
CUPCAKE_TRACE=all cargo test test_name --features deterministic-tests -- --nocapture

# Filter to specific components
CUPCAKE_TRACE=routing,wasm cargo test test_name --features deterministic-tests -- --nocapture
```

## Common Test Failure Patterns and How Tracing Helps

### 1. Policy Not Matching When Expected

**Symptom**: Test expects a policy to fire but gets `Allow` instead

**Debug with tracing**:
```bash
CUPCAKE_TRACE=routing cargo test -- --nocapture
```

**Look for**:
```json
{
  "span": {
    "name": "route_event",
    "routing_key": "PreToolUse:Bash",
    "matched_count": 0,  // <-- Problem: No matches
    "policy_names": []
  }
}
```

**Common causes**:
- Wrong metadata in test policy (check `required_events`, `required_tools`)
- Policy not in expected directory
- Package name mismatch

### 2. Signal Not Being Gathered

**Symptom**: Policy expects signal data but gets undefined

**Debug with tracing**:
```bash
CUPCAKE_TRACE=signals cargo test -- --nocapture
```

**Look for**:
```json
{
  "span": {
    "name": "gather_signals",
    "signal_count": 0,  // <-- Problem: No signals gathered
    "signals_executed": "",
    "duration_ms": 0
  }
}
```

**Common causes**:
- Signal not declared in policy's `required_signals`
- Signal name mismatch
- Guidebook not configured in test

### 3. Wrong Decision Priority

**Symptom**: Test expects `Deny` but gets `Ask`

**Debug with tracing**:
```bash
CUPCAKE_TRACE=synthesis cargo test -- --nocapture
```

**Look for**:
```json
{
  "span": {
    "name": "synthesize",
    "total_decisions": 2,
    "halts": 0,
    "denials": 1,
    "asks": 1,  // <-- Both fired, but wrong priority
    "final_decision_type": "Deny"
  }
}
```

**Common causes**:
- Multiple policies firing with different verbs
- Check synthesis priority: Halt > Deny/Block > Ask > Allow

### 4. WASM Evaluation Failures

**Symptom**: Test fails with "Failed to parse DecisionSet"

**Debug with tracing**:
```bash
CUPCAKE_TRACE=wasm cargo test -- --nocapture
```

**Look for**:
```json
{
  "span": {
    "name": "wasm_evaluate",
    "input_size_bytes": 1024,
    "output_size_bytes": 0,  // <-- Problem: No output
    "evaluation_time_ms": 1
  }
}
```

**Common causes**:
- Syntax error in Rego policy
- Missing `import rego.v1`
- Incorrect decision object structure

### 5. Timing/Performance Issues

**Symptom**: Test timeout or slow performance

**Debug with tracing**:
```bash
CUPCAKE_TRACE=eval cargo test -- --nocapture
```

**Look for**:
```json
{
  "span": {
    "name": "evaluate",
    "duration_ms": 5000  // <-- Problem: Very slow
  }
}
{
  "span": {
    "name": "gather_signals",
    "duration_ms": 4950  // <-- Culprit: Signals taking too long
  }
}
```

**Common causes**:
- Signal commands hanging or timing out
- Large policy compilation
- Infinite loops in Rego

## Advanced Debugging Techniques

### 1. Trace Filtering with jq

```bash
# Show only slow operations (>100ms)
CUPCAKE_TRACE=all cargo test 2>&1 | jq 'select(.span.duration_ms > 100)'

# Show only routing decisions
CUPCAKE_TRACE=all cargo test 2>&1 | jq 'select(.span.name == "route_event")'

# Show failed signal executions
CUPCAKE_TRACE=signals cargo test 2>&1 | jq 'select(.span.exit_code != 0)'
```

### 2. Comparing Test vs Production

```bash
# Capture test trace
CUPCAKE_TRACE=all cargo test test_name 2> test_trace.json

# Capture production trace
CUPCAKE_TRACE=all cupcake eval < event.json 2> prod_trace.json

# Compare routing decisions
diff <(jq '.span | select(.name == "route_event")' test_trace.json) \
     <(jq '.span | select(.name == "route_event")' prod_trace.json)
```

### 3. Test-Specific Tracing Helper

Add to your test file:
```rust
fn debug_with_tracing<F>(test_fn: F) 
where F: FnOnce() 
{
    // Enable tracing for this test only
    std::env::set_var("CUPCAKE_TRACE", "all");
    
    // Initialize tracing subscriber
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(tracing_subscriber::EnvFilter::new("trace"))
        .init();
    
    test_fn();
}

#[test]
fn test_complex_scenario() {
    debug_with_tracing(|| {
        // Your test code here
        // Will automatically have tracing enabled
    });
}
```

## Test Writing Best Practices

1. **Add trace assertions**: Check specific trace output in tests
2. **Use descriptive signal names**: Makes traces easier to understand
3. **Log test setup**: Add debug!() calls in test setup code
4. **Capture traces**: Save traces from passing tests as baselines

## Environment Variables for Test Debugging

- `CUPCAKE_TRACE=all` - Enable all tracing
- `RUST_LOG=debug` - General debug logging
- `RUST_BACKTRACE=1` - Stack traces on panic
- `--nocapture` - Show all output during tests

## Example: Complete Test Debug Session

```bash
# 1. Run failing test with full tracing
CUPCAKE_TRACE=all RUST_LOG=debug cargo test test_builtin_policy_evaluation --features deterministic-tests -- --nocapture 2> trace.json

# 2. Find where it went wrong
cat trace.json | jq '.span | select(.name == "route_event")'
# Output shows no policies matched

# 3. Check why routing failed
cat trace.json | jq '.fields.message | select(. != null)' | grep -i "policy"
# Shows "Policy missing routing directive"

# 4. Fix the test policy metadata and re-run
# Test now passes!
```

## Summary

Tracing transforms test debugging from guesswork to data-driven investigation. Enable it whenever a test fails unexpectedly to immediately see:
- Which policies matched (or didn't)
- What signals were gathered
- How decisions were synthesized  
- Where time was spent
- What data flowed through each stage

This makes fixing test failures much faster and more systematic.