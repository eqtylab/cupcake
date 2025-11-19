# Test Fixtures

This directory contains minimal test policies used by the TypeScript binding tests.

## Structure

```
test-fixtures/
└── .cupcake/
    └── policies/
        ├── system/
        │   └── evaluate.rego    # System aggregation entrypoint
        └── test_basic.rego       # Basic test policy
```

## Usage

The tests in `__test__/` use these fixtures to verify the TypeScript bindings work correctly without requiring a full Cupcake project setup.

These policies are compiled to WASM during test initialization and provide basic policy evaluation functionality for testing the binding layer.
