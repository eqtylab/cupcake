## Cupcake System Overview

- **Function:** A policy engine for AI coding agents (like Claude Code, Cursor, OpenCode) that intercepts tool calls, evaluates them against user-defined **OPA Rego policies**, and returns `Allow`, `Block`, or `Warn` decisions.
- **Core Flow:**
  $$\text{Event Input} \rightarrow \text{Route (O(1))} \rightarrow \text{Gather Signals} \rightarrow \text{Evaluate (WASM)} \rightarrow \text{Synthesize} \rightarrow \text{Response}$$
- **Hybrid Model:** **Rego (WASM)** declares rules/aggregates verbs; **Rust (Engine)** handles routing, signal gathering, and final decision synthesis.
- **Two-Phase Evaluation:**
  1.  **Phase 1 (Global Policies):** Evaluated first (early termination on block). Organization-wide governance from `~/.cupcake/rulebook.yml`.
  2.  **Phase 2 (Project Policies):** Evaluated only if Phase 1 allows. Project-specific rules from `.cupcake/rulebook.yml`.

## Policy Engine (WASM/Rego) Details

### 1. Routing vs. Policy Execution (CRITICAL)

- **Routing is an optimization layer**, not a selector.
  - **Controls:** Early exit if no policies match event criteria, and which signals to collect.
  - **Does NOT Control:** Which Rego rules execute inside WASM. All compiled policies run via the single entrypoint `cupcake.system.evaluate`.
- **Policy Self-Filtering (MANDATORY):** Policies **MUST** include event and tool checks in their Rego logic.
  ```rego
  deny contains decision if {
      input.hook_event_name == "PreToolUse"  # REQUIRED
      input.tool_name == "Bash"               # REQUIRED
      # ... your logic
  }
  ```

### 2. OPA Rego v1 Migration (CRITICAL)

Cupcake uses OPA v1.71.0+ where Rego v1 is the default syntax.

| Area                      | Old (Rego v0)            | New (Rego v1 / Best Practice)                                                                     | Notes                                                         |
| :------------------------ | :----------------------- | :------------------------------------------------------------------------------------------------ | :------------------------------------------------------------ |
| **Object Key Membership** | `"key" in my_object`     | `"key" in object.keys(my_object)`                                                                 | **CRITICAL:** Old syntax silently fails by returning `false`. |
| **Decision Verbs**        | `deny[decision] { ... }` | `deny contains decision if { ... }`                                                               | Use `contains` with `if`.                                     |
| **Ask Decision**          | N/A                      | Must include **`reason`** and **`question`** fields.                                              |                                                               |
| **Metadata Placement**    | Allowed anywhere         | **`scope: package` metadata MUST be the FIRST thing in the file** (before `package` declaration). | Enforced by OPA linter/compiler.                              |

### 3. Decisions & Synthesis

- **Decision Priority:** **Halt** \> **Deny/Block** \> **Ask** \> **Allow**.
- **Signal Access:** Signals are accessed via **`input.signals.*`**, not `data.*`.
- **Builtin Config Access:** Builtin policy config is accessed via **`input.builtin_config.<builtin_name>`**, not through signals.

## Testing Requirements

- **Feature Flag (MANDATORY):** All Rust tests **MUST** be run with the `--features deterministic-tests` flag for stable HMAC key generation.
  ```bash
  cargo test --workspace --features cupcake-core/deterministic-tests
  # or: just test
  ```
- **Harness Testing (CRITICAL):** When testing a specific harness (e.g., Claude Code), use the dedicated helper function to avoid compilation errors:
  ```rust
  // CORRECT
  test_helpers::create_test_project_for_harness(
      project_dir.path(),
      HarnessType::ClaudeCode
  )?;
  ```
