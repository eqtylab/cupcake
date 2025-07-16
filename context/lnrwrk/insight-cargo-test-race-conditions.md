# Insight: Cargo Test Race Conditions

Learned: 2025-01-16T00:00:00Z
During: Debugging intermittent test failures

## Problem

Integration tests that spawn processes using `cargo run` suffer from race conditions when run in parallel. This manifests as:

1. **Test hangs** - Tests waiting indefinitely (e.g., `test_cli_init_command has been running for over 60 seconds`)
2. **Unexpected output** - Tests expecting specific stderr output but getting cargo lock messages instead
3. **Stdin deadlocks** - Process waiting for stdin close, test waiting for process exit

## Root Cause

When multiple tests run `cargo run` simultaneously, they fight for Cargo's internal locks:
- Package cache lock
- Build directory lock

This causes tests to either hang waiting for locks or receive unexpected "Blocking waiting for file lock" messages in stderr.

## Solution Pattern

Replace `cargo run` in tests with a pre-built binary:

```rust
use std::sync::Once;

static BUILD_ONCE: Once = Once::new();
static mut BINARY_PATH: Option<String> = None;

fn get_cupcake_binary() -> String {
    unsafe {
        BUILD_ONCE.call_once(|| {
            let output = Command::new("cargo")
                .args(&["build"])
                .output()
                .expect("Failed to build cupcake");
            
            if !output.status.success() {
                panic!("Failed to build cupcake binary: {}", 
                    String::from_utf8_lossy(&output.stderr));
            }
            
            let path = std::env::current_dir()
                .unwrap()
                .join("target")
                .join("debug")
                .join("cupcake");
            
            BINARY_PATH = Some(path.to_string_lossy().to_string());
        });
        
        BINARY_PATH.clone().unwrap()
    }
}
```

Then use in tests:
```rust
let cupcake_binary = get_cupcake_binary();
let mut child = Command::new(&cupcake_binary)
    .args(&["run", "--event", "PreToolUse"])
    // ... rest of setup
```

## Additional Tips

For stdin handling, ensure proper closure:
```rust
// Write to stdin and explicitly close it
{
    let stdin = child.stdin.as_mut().expect("Failed to open stdin");
    stdin.write_all(data.as_bytes()).expect("Failed to write");
    stdin.flush().expect("Failed to flush stdin");
}
// stdin is dropped here, closing the pipe
```

## Benefits

- Eliminates cargo lock contention
- Tests run faster (no repeated builds)
- More reliable and deterministic
- No more mysterious hangs or timing issues