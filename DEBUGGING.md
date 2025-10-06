# Cupcake Debugging Guide

This guide covers debugging techniques for Cupcake, including policy evaluation tracing and cross-language debugging for bindings.

## Policy Evaluation Tracing

Cupcake provides comprehensive tracing to debug policy evaluation flow, timing, and decision-making.

### Enabling Tracing

Use the `CUPCAKE_TRACE` environment variable to enable structured JSON tracing output:

```bash
# Enable tracing for evaluation flow
CUPCAKE_TRACE=eval cupcake eval

# Enable tracing for specific modules
CUPCAKE_TRACE=wasm,synthesis cupcake eval

# Enable all tracing
CUPCAKE_TRACE=all cupcake eval

# Combine with RUST_LOG for additional logging
RUST_LOG=debug CUPCAKE_TRACE=eval cupcake eval
```

### Available Trace Modules

- `eval` - Main evaluation flow (routing, signals, WASM, synthesis)
- `signals` - Signal gathering and execution
- `wasm` - WASM runtime policy evaluation
- `synthesis` - Decision synthesis and prioritization  
- `routing` - Policy routing and matching
- `all` - Enable all trace modules

### Trace Output Format

When tracing is enabled, structured JSON is output to stderr with detailed spans:

```json
{
  "timestamp": "2024-09-03T12:00:00Z",
  "level": "INFO",
  "span": {
    "name": "evaluate",
    "trace_id": "018f4d2a-7c92-7f3e-b4c5-a3e9c7d8f5e1",
    "event_name": "PreToolUse",
    "tool_name": "Bash",
    "session_id": "test-123",
    "matched_policy_count": 1,
    "final_decision": "Allow",
    "duration_ms": 15
  }
}
```

### Understanding Trace Spans

#### 1. **evaluate** - Root Evaluation Span
- `trace_id`: Unique UUID v7 for this evaluation
- `event_name`: Claude Code hook event type
- `tool_name`: Tool being invoked (if applicable)
- `session_id`: Claude Code session identifier
- `matched_policy_count`: Number of policies that matched
- `final_decision`: The synthesized decision
- `duration_ms`: Total evaluation time

#### 2. **route_event** - Policy Routing
- `routing_key`: The computed routing key
- `matched_count`: Number of matching policies
- `policy_names`: List of matched policy packages

#### 3. **gather_signals** - Signal Collection
- `signal_count`: Number of signals executed
- `signals_executed`: Comma-separated list of signal names
- `duration_ms`: Time spent gathering signals

#### 4. **wasm_evaluate** - WASM Policy Evaluation
- `input_size_bytes`: Size of input JSON
- `output_size_bytes`: Size of result JSON
- `decision_count`: Total decisions from all policies
- `evaluation_time_ms`: WASM execution time

#### 5. **synthesize** - Decision Synthesis
- `total_decisions`: Sum of all decision types
- `halts`, `denials`, `blocks`, `asks`: Count by type
- `final_decision_type`: The prioritized decision
- `synthesis_time_us`: Synthesis duration in microseconds

### Debugging Common Scenarios

#### No Policies Matching

```bash
CUPCAKE_TRACE=routing cupcake eval < event.json
```

Look for `matched_count: 0` in the `route_event` span. Check:
- Policy metadata has correct `required_events` and `required_tools`
- Event JSON has proper `hook_event_name` and `tool_name` fields

#### Slow Evaluation

```bash
CUPCAKE_TRACE=eval cupcake eval < event.json
```

Check `duration_ms` fields to identify bottlenecks:
- Signal gathering often takes longest (external commands)
- WASM evaluation should be <5ms for most policies
- Synthesis should be <1ms

#### Unexpected Decisions

```bash
CUPCAKE_TRACE=synthesis cupcake eval < event.json
```

The `synthesize` span shows decision counts by type. Remember priority:
1. Halt (highest)
2. Deny/Block
3. Ask
4. AllowOverride
5. Allow (default)

### Performance Considerations

- **Zero Overhead**: Tracing has no performance impact when disabled
- **Production Safe**: Can be enabled in production for troubleshooting
- **JSON Format**: Output can be ingested by observability tools

### Integration with Observability Tools

The JSON output is compatible with tools like:
- Elasticsearch/Kibana for log aggregation
- Jaeger/Zipkin for distributed tracing (with adapter)
- CloudWatch/Datadog for metrics extraction

Example: Filtering for slow evaluations
```bash
CUPCAKE_TRACE=eval cupcake eval 2>&1 | jq 'select(.span.duration_ms > 50)'
```

## Debug File Output

Cupcake can capture the complete lifecycle of every event evaluation to human-readable debug files.

### Enabling Debug Files

Set the `CUPCAKE_DEBUG_FILES` environment variable to enable comprehensive debug capture:

```bash
# Enable debug file output
CUPCAKE_DEBUG_FILES=1 cupcake eval < event.json

# Files are written to .cupcake/debug/
ls .cupcake/debug/
```

### Debug File Structure

**Location**: `.cupcake/debug/`
**Format**: `YYYY-MM-DD_HH-MM-SS_<trace_id>.txt`

Each file contains:
- Raw Claude Code event received
- Routing decisions (which policies matched)
- Signal execution results (commands run, outputs, timing)
- WASM evaluation output (decision set with all verbs)
- Final synthesized decision
- Response sent back to Claude Code
- Action execution results
- Any errors encountered

Example debug file:

```
===== Claude Code Event [2024-09-05 20:30:15] [abc123-def456] =====
Event Type: PreToolUse
Tool: Bash
Session ID: session-789

Raw Event:
{
  "hook_event_name": "PreToolUse",
  "tool_name": "Bash",
  "tool_input": { "command": "rm -rf /tmp/test" },
  ...
}

----- Routing -----
Matched: Yes (3 policies)
- cupcake.policies.security_policy
- cupcake.policies.builtins.rulebook_security_guardrails
- cupcake.global.policies.system_protection

----- Signals -----
Configured: 2 signals
- __builtin_rulebook_protected_paths
- __builtin_system_protection_paths

Executed:
[__builtin_rulebook_protected_paths]
  Command: echo '["/etc", "/System"]'
  Duration: 5ms
  Result: ["/etc", "/System"]

----- WASM Evaluation -----
Decision Set:
  Halts: 0
  Denials: 1
    - [SECURITY-001] Dangerous command blocked: rm -rf (HIGH)
  Blocks: 0
  Asks: 0
  Allow Overrides: 0
  Context: 0

----- Synthesis -----
Final Decision: Deny
Reason: Dangerous command blocked: rm -rf

----- Response to Claude -----
{
  "continue": false,
  "stopReason": "Dangerous command blocked: rm -rf"
}

----- Actions -----
Configured: 1 action (on_any_denial)
Executed:
[log_denial]
  Command: echo "Denial logged" >> /tmp/denials.log
  Duration: 10ms
  Exit Code: 0

===== End Event [20:30:15.234] Duration: 45ms =====
```

### When to Use Debug Files

- **Development**: Understanding policy evaluation flow
- **Troubleshooting**: See exactly why policies fired or didn't
- **Performance Analysis**: Timing data for each evaluation stage
- **Signal Debugging**: Verify signal outputs and commands
- **Action Verification**: Confirm actions executed correctly

### Performance Impact

- **Zero overhead when disabled** - Single environment variable check
- **Minimal impact when enabled** - File I/O happens once at end of evaluation
- **Production safe** - Can be enabled temporarily for troubleshooting

## Routing Debug System

Cupcake's routing system maps events to policies using metadata-driven routing keys. When policies don't fire as expected, you need visibility into the routing map.

### Enabling Routing Debug

Set `CUPCAKE_DEBUG_ROUTING=1` to dump the routing map to disk:

```bash
# Enable routing debug
CUPCAKE_DEBUG_ROUTING=1 cupcake eval < event.json

# Or with Claude Code CLI
CUPCAKE_DEBUG_ROUTING=1 claude -p "hello world"

# Output location
ls .cupcake/debug/routing/
```

### Output Formats

The routing debug system generates three formats:

#### 1. Text Format (Human-Readable)

Shows the routing map organized by routes with policies listed under each:

```
Route: PreToolUse:Bash [SPECIFIC]
  Policies (5):
    1. cupcake.policies.security
       File: ./.cupcake/policies/security_policy.rego
       Events: PreToolUse
       Tools: Bash, Edit
    2. cupcake.policies.builtins.git_pre_check
       File: (builtin)
       Events: PreToolUse
       Tools: Bash
```

#### 2. JSON Format (Programmatic Analysis)

Complete routing data for tooling:

```json
{
  "timestamp": "2025-09-18_13-54-10",
  "project": {
    "routing_entries": {
      "PreToolUse:Bash": [
        {
          "package": "cupcake.policies.security",
          "file": "./.cupcake/policies/security_policy.rego",
          "events": ["PreToolUse"],
          "tools": ["Bash", "Edit"],
          "signals": ["git_branch"]
        }
      ]
    }
  },
  "statistics": {
    "total_routes": 7,
    "wildcard_routes": 4
  }
}
```

#### 3. DOT Format (Visual Graphs)

Graphviz format for generating routing diagrams:

```bash
# Generate PNG from DOT file
dot -Tpng .cupcake/debug/routing/routing_map_*.dot -o routing.png
```

The graph shows three layers:
1. **Events** (yellow ovals) - Hook event types
2. **Tools** (green diamonds) - Tool names
3. **Policies** (blue boxes) - Policy packages

Edges show routing relationships from events through tools to policies.

### Routing Key Concepts

**Routing Keys:**
- `PreToolUse:Bash` - Routes PreToolUse events with Bash tool specifically
- `PreToolUse` - Routes all PreToolUse events (wildcard)
- `PreToolUse:mcp__postgres__execute_sql` - MCP tools use full names

**Wildcard Policies:**
Policies with events but no tools match ALL tools for that event. They appear in both the wildcard route and all specific tool routes.

**Global vs Project:**
Global policies (from user config directory) and project policies (from `.cupcake/`) are tracked separately with different namespaces.

### Debugging Routing Issues

**No Policies Matching:**
```bash
CUPCAKE_DEBUG_ROUTING=1 cupcake eval < event.json
```
Check the generated routing map:
- Verify policy metadata has correct `required_events` and `required_tools`
- Confirm event JSON has proper `hook_event_name` and `tool_name` fields
- Look for the specific routing key you expect (e.g., `PreToolUse:Bash`)

**Too Many/Few Policies:**
- Check wildcard policies (they match all tools)
- Verify global policies aren't conflicting with project policies
- Use the DOT graph to visualize complex routing relationships

**Performance Issues:**
- Look for routes with many policies (might slow evaluation)
- Consider if wildcard policies should be more specific
- Check signal count (signals are executed for all matched policies)

### Performance

- **Zero impact when disabled** - Single environment variable check returns early
- **One-time cost at startup** - Debug writes happen during engine initialization
- **Not evaluated per-event** - Routing map is built once and reused

## Cross-Language Debugging

### Python/Node.js Bindings

### 1. Python: Segmentation Fault / Access Violation

**Symptom**: Python crashes with `Segmentation fault` or `Windows Access Violation`

**Diagnosis**:
```bash
# Enable Rust backtraces
export RUST_BACKTRACE=1
export RUST_BACKTRACE=full  # For more detail

# Run Python with fault handler
python -X faulthandler your_script.py

# Use GDB on Linux/macOS
gdb python
(gdb) run your_script.py
(gdb) bt  # Get backtrace after crash
```

**Common Causes**:
- Missing `py.allow_threads()` causing deadlock
- Incorrect PyO3 version mismatch
- Building with wrong Python version

**Solution**:
```bash
# Rebuild with correct Python
maturin build --interpreter python3.9

# Verify ABI compatibility
python -c "import sys; print(sys.version)"
```

### 2. GIL Not Released (Python Freezes)

**Symptom**: Multi-threaded Python app freezes during Cupcake evaluation

**Diagnosis**:
```python
import threading
import cupcake

def test_gil():
    def worker():
        print(f"Thread {threading.current_thread().name} starting")
        result = cupcake.eval({"hookEventName": "test"})
        print(f"Thread {threading.current_thread().name} done")
    
    threads = [threading.Thread(target=worker) for _ in range(3)]
    for t in threads:
        t.start()
    for t in threads:
        t.join(timeout=5)
        if t.is_alive():
            print(f"Thread {t.name} is blocked!")
```

**Solution**: Ensure `py.allow_threads()` wraps all evaluation calls in Rust:
```rust
fn evaluate(&self, input: String, py: Python) -> PyResult<String> {
    py.allow_threads(|| {  // CRITICAL: Release GIL
        self.inner.evaluate_sync(&input)
    })
}
```

### 3. Memory Leaks

**Diagnosis with Valgrind** (Linux/macOS):
```bash
valgrind --leak-check=full --show-leak-kinds=all \
    python -c "import cupcake; cupcake.init(); [cupcake.eval({'hookEventName': 'test'}) for _ in range(1000)]"
```

**Diagnosis with Python tracemalloc**:
```python
import tracemalloc
import cupcake

tracemalloc.start()
cupcake.init(".cupcake")

# Take snapshot 1
snapshot1 = tracemalloc.take_snapshot()

# Run many evaluations
for _ in range(1000):
    cupcake.eval({"hookEventName": "test"})

# Take snapshot 2
snapshot2 = tracemalloc.take_snapshot()

# Compare
top_stats = snapshot2.compare_to(snapshot1, 'lineno')
for stat in top_stats[:10]:
    print(stat)
```

### 4. Thread Safety Issues

**Testing Concurrent Access**:
```python
import concurrent.futures
import cupcake

cupcake.init(".cupcake")

def stress_test(n):
    with concurrent.futures.ThreadPoolExecutor(max_workers=10) as executor:
        futures = [
            executor.submit(cupcake.eval, {"hookEventName": "test", "id": i})
            for i in range(n)
        ]
        results = [f.result() for f in futures]
    return results

# Should complete without deadlocks or crashes
results = stress_test(100)
```

### 5. Performance Profiling

**Rust Side** (using `cargo flamegraph`):
```bash
cargo install flamegraph
cargo flamegraph --dev --bin cupcake -- eval
```

**Python Side** (using `cProfile`):
```python
import cProfile
import pstats
import cupcake

cupcake.init(".cupcake")

profiler = cProfile.Profile()
profiler.enable()

for _ in range(100):
    cupcake.eval({"hookEventName": "test"})

profiler.disable()
stats = pstats.Stats(profiler)
stats.sort_stats('cumulative')
stats.print_stats(20)
```

### 6. FFI Boundary Errors

**Debug Logging**:
```rust
// Add debug prints at FFI boundary
#[pyfunction]
fn evaluate(input: String) -> PyResult<String> {
    eprintln!("FFI: Received input: {}", input);
    let result = internal_evaluate(input)?;
    eprintln!("FFI: Returning result: {}", result);
    Ok(result)
}
```

**Environment Variables**:
```bash
# Enable all Rust logging
export RUST_LOG=debug

# Enable Tokio console (for async debugging)
export TOKIO_CONSOLE=1

# Enable Python fault handler
export PYTHONFAULTHANDLER=1
```

### 7. Build Issues

**Clean Rebuild**:
```bash
# Clean everything
cargo clean
rm -rf target/
rm -rf cupcake-py/target/
rm -rf cupcake-py/build/
rm -rf ~/.cache/cupcake/

# Rebuild with verbose output
VERBOSE=1 maturin build

# Check symbol exports
nm -D target/release/libcupcake_py.so | grep PyInit
```

**Verify Python Extension**:
```python
import importlib.util
import sys

# Find the module
spec = importlib.util.find_spec("cupcake.cupcake_native")
print(f"Module location: {spec.origin}")

# Check symbols
import ctypes
lib = ctypes.CDLL(spec.origin)
print(f"Has PyInit: {'PyInit_cupcake_native' in dir(lib)}")
```

## Platform-Specific Issues

### macOS
- **Issue**: `Library not loaded` errors
- **Solution**: Check `otool -L` output, use `install_name_tool` if needed

### Linux
- **Issue**: GLIBC version mismatch
- **Solution**: Build on oldest supported distro or use manylinux Docker

### Windows
- **Issue**: Missing MSVC runtime
- **Solution**: Install Visual C++ Redistributables

## Getting Help

When reporting issues, include:

1. **System Info**:
```bash
python -c "import sys, platform; print(f'Python: {sys.version}\\nPlatform: {platform.platform()}')"
cargo version
```

2. **Rust Backtrace**:
```bash
RUST_BACKTRACE=full python your_script.py 2> error.log
```

3. **Minimal Reproduction**:
```python
# Smallest code that triggers the issue
import cupcake
cupcake.init(".cupcake")
result = cupcake.eval({"hookEventName": "problem"})
```

## Testing Checklist

Before deploying:

- [ ] Run Python tests: `pytest cupcake-py/tests/`
- [ ] Run Rust tests: `cargo test --workspace --features cupcake-core/deterministic-tests`
- [ ] Test thread safety with 100+ concurrent evaluations
- [ ] Verify GIL release with threading test
- [ ] Check memory usage remains stable over 1000+ evaluations
- [ ] Test on all target platforms (macOS, Linux, Windows)
- [ ] Verify OPA binary downloads and checksum verification