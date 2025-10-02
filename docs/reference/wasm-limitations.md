# WASM Limitations in OPA Policies

## Overview

When OPA policies are compiled to WebAssembly (WASM), not all built-in functions are available. This document outlines the key limitations and recommended workarounds.

## sprintf Not Available in WASM

The `sprintf` function is marked as "SDK-dependent" and does not work in WASM-compiled policies.

### Workaround: Use concat

Instead of `sprintf`, use `concat` with `format_int` for building dynamic messages:

```rego
# ❌ Won't work in WASM
reason := sprintf("Coverage is %.1f%%", [coverage])

# ✅ Works in WASM
coverage_int := floor(coverage)
coverage_str := format_int(coverage_int, 10)
reason := concat("", ["Coverage is ", coverage_str, "%"])
```

### Example: Complex Messages

```rego
# ❌ Won't work in WASM
message := sprintf("User %s has %d items in %s", [username, count, location])

# ✅ Works in WASM  
count_str := format_int(count, 10)
message := concat(" ", [username, "has", count_str, "items in", location])
```

## Other SDK-Dependent Functions

The following commonly-used functions are also not available in WASM:
- `http.send` - No HTTP requests from WASM
- `opa.runtime()` - No runtime information
- `time.now_ns()` - No current time access
- `rand.intn()` - No random number generation
- `regex.split()` - Limited regex support (though `regex.match` works)

See `/Users/ramos/cupcake/cupcake-rego/opa-wasm-docs/compatibility.csv` for the full list.

## Available Alternatives

Most string and data manipulation functions work in WASM:
- ✅ `concat` - String concatenation
- ✅ `format_int` - Integer to string conversion
- ✅ `contains` - Substring checking
- ✅ `startswith`/`endswith` - String prefix/suffix
- ✅ `regex.match` - Basic regex matching
- ✅ `json.marshal`/`json.unmarshal` - JSON operations
- ✅ All comparison operators
- ✅ All arithmetic operators
- ✅ All array/object operations

## Impact on Cupcake Policies

For Cupcake policies, the main impact is slightly more verbose message formatting. This is a minor inconvenience and doesn't affect functionality.