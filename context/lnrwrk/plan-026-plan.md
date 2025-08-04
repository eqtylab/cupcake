### **OPERATION PHOENIX FIRE: FINAL TECHNICAL EXECUTION PLAN (PLAN 026)**

#### **I. OVERVIEW**

This operation will remediate the critical failure in the `PolicyEvaluator`, harden the Cupcake architecture to achieve 100% spec-compliance with Claude Code, and eliminate identified security and maintenance vulnerabilities. Execution will be phased and continuously verified. The principles are **modularity, spec-compliance, and integrated verification.**

---

#### **Phase 1: RESTORE LETHALITY (Core Functionality & Spec-Alignment)**

**Commander's Intent:** Achieve a 100% green test suite (`cargo test --workspace`) by repairing the policy filtering logic and aligning our validation rules with the complete Claude Code specification.

- **Task 1.1: Neutralize the `PolicyEvaluator` Flaw.**

  - **Location:** `src/engine/evaluation.rs`
  - **Action:** Delete the existing `build_ordered_policy_list` function.
  - **Action:** Add the following new private helper function to the `impl PolicyEvaluator` block. This function is the **single source of truth** for determining what a policy's `matcher` field should be compared against for any given event.
    ```rust
    /// Extracts the string to be used for policy matching from a given event,
    /// per the nuanced Claude Code hook specification.
    fn get_match_query(event: &ClaudeCodeEvent) -> Option<String> {
        match event {
            ClaudeCodeEvent::PreToolUse(p) => Some(p.tool_name.clone()),
            ClaudeCodeEvent::PostToolUse(p) => Some(p.tool_name.clone()),
            ClaudeCodeEvent::PreCompact(p) => Some(p.trigger.to_string().to_lowercase()), // e.g., "manual"
            ClaudeCodeEvent::SessionStart(p) => Some(p.source.to_string().to_lowercase()), // e.g., "startup"
            _ => None, // UserPromptSubmit, Stop, etc., have no spec-defined query string.
        }
    }
    ```
  - **Action:** At the beginning of the `evaluate` function, implement new filtering logic that replaces the deleted function. This new logic must:
    1.  Filter the incoming `policies` list by `policy.hook_event` to match the current `hook_event.event_name()`.
    2.  Call `get_match_query(hook_event)` to get the `query_string`.
    3.  For each policy in the filtered list, apply the following matching logic:
        - If `query_string` is `Some(q)`, the policy matches if its `matcher` is `"*"` OR if its `matcher` is a valid regex that matches `q`.
        - If `query_string` is `None`, the policy matches only if its `matcher` is `"*"` or `""`.
  - **Verification:** Create a new contract test file `tests/features/policy_matching.rs`. Add specific unit tests that create `PreCompact` and `SessionStart` events and assert that policies with the correct `matcher` (e.g., `"manual"`, `"startup"`) are correctly filtered and evaluated.

- **Task 1.2: Correct `InjectContext` Validation.**

  - **Location:** `src/config/loader.rs`, within the `validate_inject_context_event` function.
  - **Action:** Modify the `match hook_event` statement to include `HookEventType::PreCompact` as a valid event for the `InjectContext` action. The error message must also be updated to reflect this.
    ```rust
    // in src/config/loader.rs
    match hook_event {
        HookEventType::UserPromptSubmit | HookEventType::SessionStart | HookEventType::PreCompact => Ok(()),
        _ => Err(CupcakeError::Config(format!(
            "inject_context action is only valid for UserPromptSubmit, SessionStart, and PreCompact events, not {hook_event}."
        ))),
    }
    ```
  - **Verification:** Add a test in `tests/features/context_injection/yaml.rs` that loads a policy with `PreCompact` and an `inject_context` action. It must load successfully.

- **Task 1.3: Full System Verification.**
  - **Action:** Run `cargo test --workspace`.
  - **Commander's Intent:** No soldier proceeds until the entire test suite is green. This is the go/no-go for this phase.
  - **Verification:** The command exits with code 0.

---

After phase 1, read `./plan-026-phase1-after-message.md`

#### **Phase 2: REINFORCE THE FORTRESS (Architectural Hardening & Security)**

**Commander's Intent:** Eliminate architectural vulnerabilities related to data flow, response generation, and environment variable handling. The system will be made safer and more maintainable.

- **Task 2.1: Refactor the `EngineRunner` Contract.**

  - **Location:** `src/cli/commands/run/engine.rs`
  - **Action:** Modify the `EngineRunner::run` function signature. It will no longer accept `evaluation_context` and `action_context` as arguments. The new signature will be:
    ```rust
    pub fn run(
        &mut self,
        policies: &[ComposedPolicy],
        hook_event: &ClaudeCodeEvent,
    ) -> Result<EngineResult>
    ```
  - **Action:** Move the `ExecutionContextBuilder` _inside_ the `EngineRunner`. The `run` function will now be responsible for creating its own contexts, ensuring they can never be desynchronized from the event.
  - **Location:** `src/cli/commands/run/mod.rs`
  - **Action:** Update the call site in `RunCommand::execute` to pass only the `policies` and `claude_event` to `engine.run()`.
  - **Verification:** `cargo test --workspace` passes.

- **Phase 2, Task 2.2: Implement Centralized, Secure Environment Handling (REVISED)**

  - **Action:** Create a new module: `src/engine/environment.rs`.
  - **Action:** Inside this module, create a struct `SanitizedEnvironment` and a function `new()` that:
    1.  Collects all variables from `std::env::vars()`.
    2.  Filters them against a hardcoded allow-list. **This allow-list MUST include `CLAUDE_PROJECT_DIR`.** The initial recommended allow-list is:
        - `PATH`
        - `HOME`
        - `USER`
        - `TMPDIR`
        - `LANG` and `LC_*` variables for localization.
        - **`CLAUDE_PROJECT_DIR`**
    3.  Returns a `HashMap<String, String>` containing only the sanitized variables.
  - **Location:** `src/cli/commands/run/context.rs`
  - **Action:** Modify `ExecutionContextBuilder::build_claude_code_context`. Instead of calling `std::env::vars().collect()`, it will now call `SanitizedEnvironment::new()` to get the filtered environment.
  - **Verification:** Add a unit test to `src/engine/environment.rs` that asserts that `CLAUDE_PROJECT_DIR` is **preserved** by the filter, in addition to the other tests for filtering out sensitive variables.

After phase 2.2, read `./plan-026-phase2.2-after-message.md`

- **Task 2.3: Implement Modular, Spec-Compliant Response Generation.**

  - **Action:** Create the module `src/engine/response/claude_code/` with sub-modules `pre_tool_use.rs`, `feedback_loop.rs`, and `context_injection.rs` as detailed in the previous plan. These will contain the dedicated `build_response` functions.
  - **Action:** In `src/engine/response.rs`, refactor `ResponseHandler` to be a clean dispatcher that delegates to these new builders. Gut the special-case logic from `send_response_for_hook_with_suppress` and create a new `send_standard_json_response` function.
  - **Location:** `src/cli/commands/run/mod.rs`
  - **Action:** Gut the entire `match hook_event.event_name()` block. The `execute` function's final action will be a single call to the appropriate `ResponseHandler` method, passing the `EngineResult`. The logic for handling dual-mode `stdout`/JSON hooks will now live entirely within the `ResponseHandler`.
  - **Verification:** Create a new test file `tests/features/contract_tests.rs`. Add tests that call the `ResponseHandler` for each hook type and assert the serialized JSON output is bit-for-bit identical to the examples in `context/claude-code-docs/hooks.md`.

After phase 2.3, read `./plan-026-phase2.3-after-message.md`

- **Task 2.4: Eliminate the `HookEvent` Alias.**
  - **Action:** Delete the `HookEvent` type alias from `src/engine/events/mod.rs`.
  - **Action:** Perform a codebase-wide search and replace of `HookEvent` with `ClaudeCodeEvent`.
  - **Verification:** `cargo check --workspace` compiles cleanly. The command `grep -r "HookEvent" src/ tests/` returns zero results.

---

#### **Phase 3: WRITE THE DOCTRINE (Final Consolidation & Communication)**

**Commander's Intent:** Codify our victory in impeccable documentation and a clean, organized codebase.

- **Task 3.1: Execute "Operation TIDY HOUSE".**

  - **Action:** Reorganize the `tests/` directory into the `tests/features/` modular structure as previously ordered. Consolidate scattered test files into logical, feature-based modules.
  - **Verification:** The `tests/` directory is clean, logical, and easy to navigate. `cargo test --workspace` passes.

- **Task 3.2: Reconcile All Documentation.**

  - **Action:** Update the internal `src/engine/events/claude_code/README.md`. It is our ground truth. Ensure it reflects the new `get_match_query` logic and the modular response builders.
  - **Action:** Update all public-facing documentation (`docs/`). Pay special attention to `policy-format.md` (matcher behavior) and `events/claude-code.md` (the `PreCompact` stdout nuance). Add `## Last Verified: YYYY-MM-DD` to each.
  - **Verification:** A second engineer or Command reviews all documentation for accuracy and clarity.

- **Task 3.3: Final Quality and Mission Review.**
  - **Action:** Run `cargo clippy --workspace --all-targets --all-features -- -D warnings` and `cargo fmt --all -- --check`.
  - **Action:** Update `plan-025-completed.md` to reflect the full, true story of both STEEL TEMPEST and PHOENIX FIRE.
  - **Verification:** Zero warnings. Zero formatting issues. The mission log is complete and accurate.
