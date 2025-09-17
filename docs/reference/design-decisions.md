Here is the further condensed technical specification.

### **1. Modern Rego Syntax (v1.0+)**

Rego v1.0 enforces explicit syntax to disambiguate single-value rules from multi-value (set) rules.

- The `if` keyword is **mandatory** for rules with bodies.
- The `contains` keyword is **required** to define multi-value rules (partial sets).

| Old Syntax (Invalid in v1.0) | **Modern v1.0 Equivalent**   | **Description**              |
| :--------------------------- | :--------------------------- | :--------------------------- |
| `p { true }`                 | `p if { true }`              | Single-value rule.           |
| `p.a { true }`               | `p contains "a" if { true }` | Multi-value rule (set).      |
| `p.a`                        | `p contains "a"`             | Constant multi-value set.    |
| `p.a.b { true }`             | `p.a.b if { true }`          | Nested object, single-value. |

### **2. Policy Metadata**

Metadata is machine-readable YAML inside `# METADATA` comment blocks that are programmatically accessible.

**Scopes:**

- `rule`: Applies to the next rule in the file.
- `document`: Applies to all rules with the same name within a package.
- `package`: Applies to all rules in the package, across files.
- `subpackages`: Applies to the package and all its descendants.

**Capabilities:**

- **API Definition**: `entrypoint: true` marks a rule as a primary, queryable decision for build tooling.
- **Type Safety**: The `schemas` field links inputs and data to schemas for static type-checking.
- **Dynamic Output**: Access metadata at runtime via `rego.metadata.rule()` to generate rich results (e.g., severity, error messages).
- **Auditing**: Use `authors`, `organizations`, and `custom` fields for ownership and compliance tracking.

### **3. WebAssembly (WASM) Compilation**

Policies can be compiled into portable, secure, and performant WASM modules.

**Security Model: Deny-by-Default Sandbox**

- **Isolation**: WASM has its own memory space, preventing access to host application memory.
- **Permissions**: WASM has no I/O (filesystem, network) capabilities by default. The host must explicitly provide them.
- **Safety**: This model allows for the safe execution of untrusted or dynamically loaded policies, unlike native libraries which share the host's process space and permissions.

**Compilation & Execution**

- **Build Command**: `opa build -t wasm -e <entrypoint> <policy_files>`
- **Execution**: Use language-specific SDKs (e.g., JavaScript) or a WASM runtime with the OPA WASM ABI.
- **Core ABI Exports**: `opa_eval_ctx_new`, `opa_eval_ctx_set_input`, `opa_eval_ctx_set_data`, `eval`, `opa_eval_ctx_get_result`, `opa_json_parse`, `opa_malloc`, `opa_free`.

### **4. High-Performance WASM via Host-Side Indexing**

The Go runtime uses rule indexing for O(1) lookups. Raw WASM evaluation is an O(n) linear scan. Replicate native performance in a WASM host with this pattern:

1.  **Index Creation**: At startup, the host application pre-processes policies (or their metadata) to build a routing map. This map links input attributes (e.g., `input.method`, `input.path`) to the relevant WASM policy entrypoints.
2.  **Runtime Routing**: On a request, the host uses the map to identify the small subset of policies that can possibly match the input.
3.  **Scoped Evaluation**: The host invokes the WASM module, evaluating only the pre-selected, relevant rules.
