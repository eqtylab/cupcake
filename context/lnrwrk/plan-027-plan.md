This is the technical execution plan for **Operation FORGE**. It is a comprehensive, phased, and verifiable campaign to transform Cupcake from a promising but flawed tool into a dominant, elegant, and doctrinally pure governance engine.

---

### **PLAN 027: OPERATION FORGE: TECHNICAL EXECUTION PLAN (OPORD 005)**

**I. OVERVIEW**
This operation will systematically address every confirmed codebase problem and misalignment identified in the CLARION CALL reconnaissance. We will proceed in four phases, prioritizing existential threats first, followed by core functionality, user-facing tooling, and finally, architectural refinement. Each task includes a precise location, action, and a non-negotiable verification condition.

---

### **Phase 1: SECURE THE FORTRESS (Foundational Stability & Security)**

**Commander's Intent:** Before we can fight the enemy, we must secure our own base. This phase eliminates catastrophic failure modes and establishes professional-grade operational standards.

- **Task 1.1: Implement Fail-Closed Error Handling (Ref: IR 1.1)**

  - **Location:** `src/cli/commands/run/mod.rs`.
  - **Action:**
    1.  Create a new error handling module: `src/cli/error_handler.rs`.
    2.  This module will contain a function `handle_run_command_error(e: CupcakeError) -> !` that constructs a spec-compliant "deny" JSON response for `PreToolUse` or a generic `{"continue":false}` response for other hooks. It will print this JSON to `stdout` and exit with code `0`, effectively telling Claude Code to block the action safely.
    3.  In `run/mod.rs`, replace all three `std::process::exit(0)` calls in the `Err(e)` blocks with a single call to `error_handler::handle_run_command_error(e)`.
  - **Verification:** Create a new integration test that runs `cupcake run` with a path to a malformed config file. The test must assert that the command's `stdout` is a valid blocking JSON object and the exit code is `0`.

- **Task 1.2: Modernize Logging (Ref: IR 1.3)**

  - **Location:** `Cargo.toml`, `src/main.rs`, and throughout the codebase.
  - **Action:**
    1.  Add `tracing` and `tracing_subscriber` to `Cargo.toml`.
    2.  Initialize the logger in `src/main.rs`, configured to respect `RUST_LOG` environment variable.
    3.  Remove the custom `append_debug_log` function and all its calls from `src/cli/commands/run/mod.rs`.
    4.  Replace all instances of `eprintln!("Debug: ...")` with appropriate `tracing::debug!`, `tracing::info!`, or `tracing::warn!` macros.
  - **Verification:** Running `RUST_LOG=debug cupcake run ...` produces structured debug output to `stderr`. The `/tmp/cupcake-debug.log` file is no longer created.

- **Task 1.3: Enforce Strict Policy Loading (Ref: IR O-7)**
  - **Location:** `src/config/loader.rs` and `src/config/types.rs`.
  - **Action:**
    1.  Add a non-optional `kind: String` field to both `RootConfig` and `YamlPolicy` structs in `types.rs`. Add `#[serde(rename = "kind")]` to ensure it's deserialized correctly. The value should be validated as either `"RootConfig"` or `"PolicyFragment"`.
    2.  In `loader.rs`, gut the entire heuristic-based fallback logic in `load_configuration` and `load_from_config_file`. The new logic will parse the YAML, check the `kind` field, and then deserialize into the appropriate struct. A missing or incorrect `kind` is a hard error.
  - **Verification:** All existing policy file tests must be updated to include the `kind` field. A new test must be added that attempts to load a policy file _without_ a `kind` field and asserts that it returns a `CupcakeError::Config`.

---

### **Phase 2: HONOR THE ALLIANCE (Total Spec-Compliance)**

**Commander's Intent:** We will close every gap between our implementation and the Claude Code specification. When this phase is complete, we will be the most reliable and predictable partner in their ecosystem.

- **Task 2.1: Correct Matcher Semantics (Ref: IR 3.1)**

  - **Location:** `src/engine/evaluation.rs`.
  - **Action:**
    1.  Create a private helper function `fn is_regex(pattern: &str) -> bool` that checks for the presence of regex metacharacters (e.g., `*`, `+`, `?`, `(`, `[`, `|`).
    2.  In the `evaluate` function's filtering logic, modify the `else` block to use this helper. If `is_regex` is true, compile and match as regex. If false, perform a direct string equality check (`policy.matcher == query`).
  - **Verification:** Add a new test to `tests/features/policy_matching.rs` with a plain string matcher (e.g., `"Bash"`). Assert that it matches `"Bash"` but does _not_ match `"BashScript"`.

- **Task 2.2: Fix Context Injection (`use_stdout`) Flaw (Ref: IR 4.1)**

  - **Location:** `src/cli/commands/run/mod.rs` and `src/engine/response/claude_code/context_injection.rs`.
  - **Action:**
    1.  The `EngineResult` struct in `engine.rs` must be augmented to carry information about the `use_stdout` preference from the winning `InjectContext` policy. A simple approach is to add `pub injection_mode: Option<InjectionMode>` where `InjectionMode` is an enum `Stdout` or `Json`.
    2.  The `ActionExecutor` will populate this field when it executes a successful `InjectContext` action.
    3.  The special-case handling for `UserPromptSubmit` and `SessionStart` in `run/mod.rs` will be modified. It will now check `result.injection_mode` to decide whether to print to `stdout` or fall through to the JSON response handler.
  - **Verification:** Create two integration tests: one with a policy using `inject_context` with `use_stdout: true` (assert raw text on `stdout`), and one with `use_stdout: false` (assert JSON with `additionalContext` on `stdout`).

- **Task 2.3: Align All Response Formats (Ref: IR 4.2, 4.3, 4.4, A-1)**
  - **Location:** `src/engine/response/claude_code/` builders.
  - **Action:**
    1.  **`UserPromptSubmit` Block:** In `context_injection.rs`, modify the `Block` decision handler to generate `{"decision": "block", "reason": "..."}` as the spec requires for this hook.
    2.  **`Ask` Misalignment:** In `context_injection.rs`, for `UserPromptSubmit` and `SessionStart`, an `Ask` decision should produce an empty JSON response and log a warning, as these hooks do not have a first-class `Ask` flow. The current behavior of populating `additionalContext` is undefined and must be removed.
    3.  **Deprecation & Backwards Compatibility:**
        a. Add a new setting `claude_code_legacy_responses: bool` (default `false`) to `Settings` in `src/config/types.rs`.
        b. In the `pre_tool_use.rs` builder, if this setting is `true`, emit _both_ the modern `hookSpecificOutput` and the deprecated top-level `decision` and `reason` fields.
  - **Verification:** The `contract_tests.rs` suite must be expanded to cover every one of these cases, asserting bit-for-bit JSON output correctness.

---

### **Phase 3: REFORGE THE BRIDGE (`sync` Command Overhaul)**

**Commander's Intent:** The `sync` command is a liability. We will rebuild it into a trustworthy, idempotent, and tactically useful tool that empowers our users instead of sabotaging them.

- **Task 3.1: Implement Idempotent Sync Logic (Ref: IR 2.2b, O-13)**

  - **Location:** `src/config/claude_hooks.rs` and `src/cli/commands/sync.rs`.
  - **Action:**
    1.  In `build_cupcake_hooks`, add a new field to the top-level hook object: `"_managed_by": "cupcake"`.
    2.  In `sync.rs`, completely rewrite the `merge_hooks` function. The new logic will be a clean, idempotent remove-then-append process:
        a. Read the existing `settings.local.json`.
        b. Filter the hook arrays for each event, _removing_ any hook object where `_managed_by == "cupcake"`.
        c. Append the newly generated Cupcake hooks to the filtered arrays.
    3.  The `--force` flag will now be repurposed to mean "sync even if no changes are detected."
  - **Verification:** A new integration test that runs `cupcake sync` twice. It must assert that the `settings.local.json` file's content is identical after the second run.

- **Task 3.2: Generate Intelligent Matchers (Ref: IR 2.1a)**
  - **Location:** `src/config/claude_hooks.rs`.
  - **Action:** Modify `build_cupcake_hooks`. Instead of generating a single entry with `matcher: "*"`, it will now generate _multiple_, specific default hook entries where appropriate.
    - For `PreCompact`, generate two hook objects: one for `matcher: "manual"` and one for `matcher: "auto"`.
    - For `SessionStart`, generate three: `matcher: "startup"`, `matcher: "resume"`, `matcher: "clear"`.
    - `PreToolUse` and `PostToolUse` will retain a single `matcher: "*"` as a sensible default.
  - **Verification:** A unit test will call `build_cupcake_hooks` and assert that the generated JSON `Value` for `PreCompact` is an array with two distinct objects.

---

### **Phase 4: PAY THE DEBTS (Architectural & Doctrinal Purity)**

**Commander's Intent:** Finish the fight. Leave no technical debt behind. The final codebase will be a model of clarity, efficiency, and correctness.

- **Task 4.1: Technical Debt Strike Force**

  - **`Stop`/`SubagentStop` Duplication (Ref: IR 3.2):**
    - **Location:** `src/engine/events/claude_code/`.
    - **Action:** Delete `subagent_stop.rs`. In `stop.rs`, rename `StopPayload` to a generic `StopPayload`. In `mod.rs`, create two type aliases: `pub type MainStopPayload = StopPayload;` and `pub type SubagentStopPayload = StopPayload;`. Update the `ClaudeCodeEvent` enum to use these aliases.
  - **`AgentEvent` Abstraction Leak (Ref: IR 3.3):**
    - **Location:** `src/cli/commands/run/engine.rs`.
    - **Action:** The `EngineRunner::run` function will now contain the top-level `match agent_event`. The current logic will be moved into a new private helper function `run_claude_code(&mut self, event: &ClaudeCodeEvent, ...)` which will be called from the `match` arm. This properly contains the agent-specific logic and makes the public `run` function truly polymorphic.
  - **Timeout Misalignment (Ref: IR 4.6):**
    - **Location:** `src/cli/commands/run/mod.rs` and `src/config/claude_hooks.rs`.
    - **Action:** This is a two-part fix.
      1.  The `timeout` value in `claude_hooks.rs` is for Cupcake's execution. In `run/mod.rs`, we will parse this `timeout` from the hook config (`claude_hooks.rs`) and wrap the entire `engine.run()` call in a `tokio::time::timeout`. If it expires, we fail closed.
      2.  The `timeout_seconds` in our _policy YAML_ is for commands run _by_ Cupcake. This is already implemented correctly in `engine/actions.rs`. No change needed there.
  - **Payload Field Absence (Ref: IR 4.5):**
    - **Location:** All payload structs in `src/engine/events/claude_code/`.
    - **Action:** This was a misinterpretation by OSINT. The `hook_event_name` is the `tag` in the `ClaudeCodeEvent` enum, not a field in the payload structs. This is correct as-is. **Action: No change required.** Add a comment to `ClaudeCodeEvent` enum in `mod.rs` clarifying this for future maintainers.

- **Task 4.2: Documentation Reconciliation (Ref: IR 5.1)**

  - **Action:** Conduct a full documentation sweep. The ground truth about `PreCompact`'s `\n\n` joiner, the corrected matcher semantics, and the new fail-closed behavior must be promoted from internal findings into the primary user-facing `docs/` files. All docs will receive a `Last Verified: YYYY-MM-DD` timestamp.

- **Task 4.3: Final Quality & Mission Review**
  - **Action:** Run `cargo clippy --workspace --all-targets --all-features -- -D warnings` and `cargo fmt --all -- --check`.
  - **Verification:** Zero warnings. Zero formatting issues. The war is won, and the battlefield is clean.
