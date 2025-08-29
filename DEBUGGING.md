# Cross-Language Debugging Guide for Cupcake

This guide helps debug issues across the Rust/Python/Node.js boundaries in Cupcake bindings.

## Common Issues and Solutions

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