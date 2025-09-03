## **Python & Node.js Bindings for Cupcake**

This document specifies the technical approach for creating, building, and distributing Python and Node.js bindings for the Cupcake engine.

While we detail Node.js in a fair manner compared to Python, we will prioritize Python. We're an end-to-end working MVP.

### **Architectural Foundation: A Cargo Workspace**

A Cargo workspace is the definitive choice for this project. It provides a clean, scalable, and industry-standard structure that directly maps to the end-goal of distributing separate packages to PyPI and npm.

1.  **Build & Distribution Simplicity:** Tools like `maturin` (for Python) and `@napi-rs/cli` (for Node.js) are designed to work on a per-crate basis. A workspace provides a dedicated crate for each binding, making build configurations clean and independent.
2.  **True Separation of Concerns:** Each binding has unique FFI dependencies (`pyo3`, `napi-rs`). A workspace isolates these, preventing dependency bloat in the core engine or conflicts between bindings.
3.  **Clarity and Maintainability:** The dependency graph becomes explicit: `cupcake-py` depends on `cupcake-core`. This is far clearer than a web of `#[cfg(feature = "...")]` attributes scattered throughout the code.
4.  **Future-Proofing:** If a Go or Ruby binding is ever needed, adding a new crate to the workspace is a trivial, isolated change.

---

### **Phase 1: Foundational Refactoring**

This phase establishes a correct, thread-safe core engine within a new workspace structure, incorporating critical changes to the WebAssembly runtime handling for safe concurrency.

**1.1. Project Migration to Cargo Workspace**

The existing single crate will be migrated into `cupcake-core` and `cupcake-cli` library and binary crates within a new workspace.

- **Root `Cargo.toml`:**

  ```toml
  [workspace]
  members = ["cupcake-core", "cupcake-cli", "cupcake-py", "cupcake-js"]
  resolver = "2"

  [workspace.dependencies]
  tokio = { version = "1.46.1", features = ["full"] }
  serde_json = "1.0"
  anyhow = "1.0"
  # ... other shared dependencies
  ```

- **Directory Structure:**
  ```
  cupcake-rewrite/
  ├── Cargo.toml          # Workspace root
  ├── cupcake-core/       # All engine/harness/trust library code
  │   ├── src/
  │   └── Cargo.toml
  ├── cupcake-cli/        # The `main.rs` binary code
  │   ├── src/
  │   └── Cargo.toml
  ├── cupcake-py/         # Python binding crate
  └── cupcake-js/         # Node.js binding crate
  ```

**1.2. `WasmRuntime` and Engine Refactor for Concurrency**

Due to `wasmtime` constraints, the `Store` object, which holds state, cannot be shared across threads. It must be created fresh for each evaluation. This makes the `WasmRuntime` itself stateless during evaluation and inherently thread-safe, simplifying the core `Engine`'s design.

- **`cupcake-core/src/engine/wasm_runtime.rs`:**

  ```rust
  pub struct WasmRuntime {
      engine: Engine,
      module: Module,
      linker: Linker<()>,
  }

  impl WasmRuntime {
      // ... new() will prepare the long-lived engine, module, and linker ...

      pub fn query_decision_set(&self, input: &Value) -> Result<DecisionSet> {
          // Create a fresh, short-lived Store for each evaluation for thread safety.
          let mut store = Store::new(&self.engine, ());
          let instance = self.linker.instantiate(&mut store, &self.module)?;

          // Perform evaluation using this ephemeral store and instance.
          let result_json = self.evaluate_raw(&mut store, &instance, input)?;

          // ... parse result_json into DecisionSet ...
          Ok(decision_set)
      } // Store and Instance are dropped here, ensuring memory safety.
  }
  ```

- **`cupcake-core/src/engine/mod.rs`:**

  ```rust
  use anyhow::Result;
  use serde_json::Value;

  pub struct Engine {
      // ... other fields ...
      // No Mutex needed; WasmRuntime is now thread-safe by design.
      wasm_runtime: Option<wasm_runtime::WasmRuntime>,
  }

  impl Engine {
      pub async fn new(project_path: impl AsRef<Path>) -> Result<Self> {
          // ... initialization ...
      }

      pub async fn evaluate(&self, input: &Value) -> Result<decision::FinalDecision> {
          // ... routing and signal logic ...
          let runtime = self.wasm_runtime.as_ref().context("WASM runtime not initialized")?;
          let decision_set = runtime.query_decision_set(input)?;
          // ... synthesis and actions ...
          Ok(final_decision)
      }
  }
  ```

### **Phase 2: FFI Abstraction Layer**

A dedicated `BindingEngine` will serve as the public-facing Rust API for all foreign language bindings, using a `current_thread` Tokio runtime for optimal FFI performance and safety.

- **Location:** `cupcake-core/src/bindings.rs`
- **Implementation:**

  ```rust
  use crate::engine::Engine;
  use serde_json::Value;
  use std::sync::Arc;

  #[derive(Clone)]
  pub struct BindingEngine {
      inner: Arc<Engine>,
      runtime: Arc<tokio::runtime::Runtime>,
  }

  impl BindingEngine {
      pub fn new(path: &str) -> Result<Self, String> {
          let runtime = tokio::runtime::Builder::new_current_thread()
              .enable_all()
              .build()
              .map_err(|e| format!("Failed to create Tokio runtime: {}", e))?;

          let engine = runtime
              .block_on(Engine::new(path))
              .map_err(|e| format!("Failed to initialize core engine: {}", e))?;

          Ok(Self {
              inner: Arc::new(engine),
              runtime: Arc::new(runtime),
          })
      }

      pub fn evaluate_sync(&self, input_json: &str) -> Result<String, String> {
          let input: Value = serde_json::from_str(input_json)
              .map_err(|e| format!("Invalid input JSON: {}", e))?;

          let decision = self
              .runtime
              .block_on(self.inner.evaluate(&input))
              .map_err(|e| format!("Core engine evaluation failed: {}", e))?;

          serde_json::to_string(&decision)
              .map_err(|e| format!("Failed to serialize final decision: {}", e))
      }

      pub async fn evaluate_async(&self, input_json: &str) -> Result<String, String> {
          let input: Value = serde_json::from_str(input_json)
              .map_err(|e| format!("Invalid input JSON: {}", e))?;

          let decision = self
              .inner
              .evaluate(&input)
              .await
              .map_err(|e| format!("Core engine evaluation failed: {}", e))?;

          serde_json::to_string(&decision)
              .map_err(|e| format!("Failed to serialize final decision: {}", e))
      }
  }
  ```

### **Phase 3: Python Binding MVP**

- **`cupcake-py/src/lib.rs` (Rust Binding):**

  ```rust
  use cupcake_core::bindings::BindingEngine;
  use pyo3::prelude::*;

  #[pyclass(name = "PolicyEngine")]
  struct PyPolicyEngine {
      inner: BindingEngine,
  }

  #[pymethods]
  impl PyPolicyEngine {
      #[new]
      fn new(path: String) -> PyResult<Self> {
          let engine = BindingEngine::new(&path)
              .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e))?;
          Ok(Self { inner: engine })
      }

      fn evaluate(&self, input: String, py: Python) -> PyResult<String> {
          py.allow_threads(|| {
              self.inner.evaluate_sync(&input)
                  .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e))
          })
      }
  }
  ```

- **`cupcake-py/cupcake/__init__.py` (Python API):**

  ```python
  import asyncio
  import json
  from typing import Dict, Any, Optional
  from .cupcake_native import PolicyEngine
  from .installer import ensure_opa_installed

  class Cupcake:
      def __init__(self):
          self._engine: Optional[PolicyEngine] = None

      def init(self, path: str = ".cupcake") -> None:
          ensure_opa_installed()
          self._engine = PolicyEngine(path)

      async def init_async(self, path: str = ".cupcake") -> None:
          await asyncio.to_thread(ensure_opa_installed)
          self._engine = await asyncio.to_thread(PolicyEngine, path)

      def eval(self, event: Dict[str, Any]) -> Dict[str, Any]:
          if not self._engine:
              raise RuntimeError("Cupcake engine not initialized. Call init() or init_async() first.")
          result_json = self._engine.evaluate(json.dumps(event))
          return json.loads(result_json)

      async def eval_async(self, event: Dict[str, Any]) -> Dict[str, Any]:
          if not self._engine:
              raise RuntimeError("Cupcake engine not initialized. Call init_async() first.")
          result_json = await asyncio.to_thread(self._engine.evaluate, json.dumps(event))
          return json.loads(result_json)

  _default_instance = Cupcake()
  init = _default_instance.init
  eval = _default_instance.eval
  init_async = _default_instance.init_async
  eval_async = _default_instance.eval_async
  ```

### **Phase 4: Test Suite Modernization**

- **Relocation:** Core engine tests will move to `cupcake-core/tests/`. CLI tests will move to `cupcake-cli/tests/`.
- **Adaptation:** All tests will be updated to use `async fn` and `.await` for engine operations. The `justfile` test command will be updated to run tests for the entire workspace: `cargo test --workspace --features cupcake-core/deterministic-tests`.
- **Enhancement:** New test suites will be added to `cupcake-core/tests/` to explicitly validate concurrent evaluations on a shared `Engine` instance and to verify the behavior of the `BindingEngine`'s sync/async API.

### **Phase 5: Distribution and Documentation**

- **OPA Binary Management:** A shared, secure strategy will manage the `opa` CLI dependency by checking an environment variable, a local cache, or downloading a version-pinned, checksum-verified binary.
- **Distribution:** `maturin` will be used to publish cross-platform wheels to PyPI. `@napi-rs/cli` will be used to publish pre-compiled binaries to npm.
- **Documentation:** The `README.md` for each package will include a mandatory section on the concurrency model.

> ### Concurrency Model
>
> The Cupcake engine is thread-safe and designed for concurrent evaluation. You can safely call `eval()` or `eval_async()` from multiple threads or async tasks on a single, shared engine instance for parallel processing. Each evaluation is handled in an isolated memory space, ensuring correctness.

### **Phase 6: Node.js Expansion**

After a solid, in the end working MPP for Python, we should be able to implement the Node.js version rather seamlessly. Everything we learn from the pipe arm implementation should be a guarding star.

- **`cupcake-js/src/lib.rs` (Rust Binding):**

  ```rust
  use cupcake_core::bindings::BindingEngine;
  use napi::bindgen_prelude::*;
  use napi_derive::napi;

  #[napi(js_name = "Engine")]
  pub struct JsEngine {
      inner: BindingEngine,
  }

  #[napi]
  impl JsEngine {
      #[napi(constructor)]
      pub fn new(path: String) -> Result<Self> {
          let engine = BindingEngine::new(&path)
              .map_err(|e| Error::new(Status::GenericFailure, e))?;
          Ok(Self { inner: engine })
      }

      #[napi]
      pub async fn evaluate(&self, input: String) -> Result<String> {
          self.inner.evaluate_async(&input).await
              .map_err(|e| Error::new(Status::GenericFailure, e))
      }

      #[napi]
      pub fn evaluate_sync(&self, input: String) -> Result<String> {
          self.inner.evaluate_sync(&input)
              .map_err(|e| Error::new(Status::GenericFailure, e))
      }
  }
  ```
