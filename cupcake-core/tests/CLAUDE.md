# Test Policy Requirements

When writing integration tests that create Rego policies, follow these requirements:

## Critical: Test Execution Requirements

**IMPORTANT**: All tests MUST be run with the `--features deterministic-tests` flag. This is NOT optional - the trust system tests will fail intermittently without it due to non-deterministic HMAC key derivation in production mode. 

The feature flag ensures deterministic key generation for reliable test execution in both unit tests (src/) and integration tests (tests/). Without this flag, tests will experience race conditions and HMAC verification failures.

Run tests using:
- `cargo test --features deterministic-tests`
- `cargo t` (configured alias)

## System Evaluate Policy

**MUST** use the authoritative `system/evaluate.rego` from `fixtures/system_evaluate.rego`:

```rego
package cupcake.system
import rego.v1

# METADATA with entrypoint: true

evaluate := decision_set if {
    decision_set := {
        "halts": collect_verbs("halt"),
        "denials": collect_verbs("deny"),
        "blocks": collect_verbs("block"),
        "asks": collect_verbs("ask"),
        "modifications": collect_verbs("modify"),
        "add_context": collect_verbs("add_context")
    }
}

collect_verbs(verb_name) := result if {
    # Uses walk() to traverse data.cupcake.policies
    # ... full implementation
}
```

**DO NOT** use simplified static versions - they cause OPA compilation failures.

## Test Policies

- Use `import rego.v1` for modern Rego syntax
- Add `# METADATA` blocks with routing requirements
- Use decision verb sets: `deny contains decision if { ... }`
- Include structured decision objects with `reason`, `severity`, `rule_id`

## Directory Structure

Match the authoritative structure:
```
.cupcake/
├── policies/
│   ├── system/
│   │   └── evaluate.rego  # Authoritative version only
│   └── *.rego             # Test policies
├── signals/
└── actions/
```