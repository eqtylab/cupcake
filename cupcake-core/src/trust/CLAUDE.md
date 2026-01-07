# Trust System Implementation

This module implements cryptographic trust verification for Cupcake scripts.

## Critical: Test Execution Requirements

**IMPORTANT**: Tests for this module MUST be run with the `--features deterministic-tests` flag. This is NOT optional - tests will fail intermittently without it.

### Why This Is Required

The trust system uses HMAC-SHA256 for integrity verification with keys derived from system entropy (machine ID, executable path, etc.). In production, this provides security through non-determinism. However, in tests, this causes failures because:

1. Integration tests in `tests/` compile the library in production mode (not test mode)
2. The `#[cfg(test)]` attribute is only active for unit tests, not integration tests
3. Without deterministic keys, HMAC signatures computed during save won't match during load
4. This manifests as "Trust manifest has been tampered with!" errors

### Solution

The `deterministic-tests` feature flag explicitly enables fixed key derivation for ALL test types:

```rust
#[cfg(feature = "deterministic-tests")]
{
    // Fixed keys for deterministic testing
    hasher.update(b"TEST_MODE_FIXED_PROJECT");
}

#[cfg(not(feature = "deterministic-tests"))]
{
    // Production: Use system-specific entropy
    hasher.update(machine_id);
    // ...
}
```

### Running Tests

Always use:
- `cargo test --features deterministic-tests`
- `cargo t` (configured alias in `.cargo/config.toml`)

## Architecture

### Key Components

- `hasher.rs` - HMAC key derivation and cryptographic operations
- `manifest.rs` - Trust manifest serialization and script management
- `verifier.rs` - Runtime verification of trusted scripts
- `error.rs` - Security-aware error handling

### Security Design

1. **Project-specific keys**: Each project derives unique HMAC keys
2. **System entropy**: Production keys include machine ID, user, exe path
3. **Tamper detection**: Any manifest modification triggers security alerts
4. **No external dependencies**: Self-contained trust verification

### Trust Workflow

1. `cupcake trust init` - Discovers and hashes all scripts
2. Manifest saved with HMAC signature
3. Runtime verification checks script hash before execution
4. Any modification triggers security alert

## Development Guidelines

- Never expose key material in logs or errors
- Maintain constant-time comparison for HMAC verification
- Test both positive and negative security scenarios
- Always run tests with `--features deterministic-tests`