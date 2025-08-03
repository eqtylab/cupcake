Plan 026: Continued Hook Refactor

### **OPERATION PHOENIX FIRE: TECHNICAL EXECUTION PLAN**

#### **I. OVERVIEW**

This is a phased, verifiable plan to remediate the critical failure in `PolicyEvaluator` and harden the Cupcake architecture for true, spec-compliant alignment with Claude Code. The guiding principles are **modularity, type safety, and integrated verification.**

---

#### **Phase 1: SECURE THE IGNITION (Policy Evaluation Remediation)**

**Commander's Intent:** Achieve a 100% green test suite by repairing the core policy filtering and matching logic to be fully compliant with the Claude Code hook specification.

##### AMENDED 1.1!!

- **Task 1.1: Refactor `PolicyEvaluator` for Spec-Compliant Matching.**

<pre>DEPRECATED: This task has been amended to reflect the latest understanding of the Claude Code hook specification. The original `build_ordered_policy_list` function is being replaced with a more robust and spec-compliant matching mechanism.

- **Location:** `src/engine/evaluation.rs`
- **Action:** Delete the existing `build_ordered_policy_list` function.
- **Action:** Create a new private helper function within `PolicyEvaluator`:
  ```rust
  /// Extracts the string to be used for policy matching from a given event.
  fn get_match_query(event: &ClaudeCodeEvent) -> Option<String> {
      match event {
          ClaudeCodeEvent::PreToolUse(p) => Some(p.tool_name.clone()),
          ClaudeCodeEvent::PostToolUse(p) => Some(p.tool_name.clone()),
          ClaudeCodeEvent::PreCompact(p) => Some(p.trigger.to_string()), // e.g., "manual"
          ClaudeCodeEvent::SessionStart(p) => Some(p.source.to_string()), // e.g., "startup"
          _ => None, // UserPromptSubmit, Stop, etc., have no query string.
      }
  }
  ```
- **Action:** Re-implement the policy filtering logic at the beginning of the main `evaluate` function. It will now use `get_match_query` to correctly filter the `policies` list. The new logic must handle:
  1.  Filtering by `policy.hook_event`.
  2.  Calling `get_match_query` to get the string for matching.
  3.  If `query` is `Some(string)`, apply `*` and regex matching against `policy.matcher`.
  4.  If `query` is `None`, only policies with `matcher` of `"*"` or `""` should be considered.
- **Verification:** The failing tests in `tests/features/context_injection/yaml.rs` (specifically `test_empty_matcher_with_pre_tool_use` and its variants) must now pass.

</pre>

- **LOCATION:** `src/engine/evaluation.rs` (specifically, the `build_ordered_policy_list` function or where policy filtering logic occurs).
- **CHANGE:** **MODIFICATION** of the policy filtering logic.
- **INSTRUCTION:**

  - **DELETE** the existing `build_ordered_policy_list` function.
  - **ADD** a new private helper function `get_match_query` to `PolicyEvaluator`:
    ```rust
    // In src/engine/evaluation.rs, within PolicyEvaluator impl
    /// Extracts the string to be used for policy matching from a given event,
    /// per the Claude Code hook specification.
    fn get_match_query(event: &ClaudeCodeEvent) -> Option<String> {
        match event {
            ClaudeCodeEvent::PreToolUse(p) => Some(p.tool_name.clone()),
            ClaudeCodeEvent::PostToolUse(p) => Some(p.tool_name.clone()),
            ClaudeCodeEvent::PreCompact(p) => Some(p.trigger.to_string()), // e.g., "manual"
            ClaudeCodeEvent::SessionStart(p) => Some(p.source.to_string()), // e.g., "startup"
            _ => None, // UserPromptSubmit, Stop, etc., have no spec-defined query string.
        }
    }
    ```
  - **MODIFY** the main `evaluate` function: Re-implement the policy filtering logic at its beginning. It must now use `get_match_query` to correctly extract the matching string (or `None`) and filter policies based on `policy.hook_event` and `policy.matcher`.

- **REASONING:**
  - **WHY:** The original filtering logic was based on incomplete intelligence from Claude Code's specification, leading to the critical `empty_matcher_test.rs` failure. Claude Code's `matcher` field means different things for different hooks. This amendment implements the precise, nuanced matching rules for all event types.
  - **HOW:** By creating `get_match_query`, we centralize the logic for extracting the "thing to match against" for any given hook. The `evaluate` function then applies a unified matching strategy. This eliminates the core bug.
  - **WHERE:** `PolicyEvaluator` is the central brain for policy matching. This fix directly targets its core function.

* **Task 1.2: Full Test Suite Verification.**

#### AMENDED 1.2!!

NEW:

- **LOCATION:** `src/config/loader.rs` (specifically, the `validate_inject_context_event` function).
- **CHANGE:** **MODIFICATION** of an existing validation rule.
- **INSTRUCTION:**

  - **MODIFY** the `match hook_event` statement within `validate_inject_context_event`.
  - **ADD** `HookEventType::PreCompact` to the list of allowed events for `InjectContext` action.
    ```rust
    // In src/config/loader.rs, within validate_inject_context_event function
    // ...
    match hook_event {
        HookEventType::UserPromptSubmit
        | HookEventType::SessionStart
        | HookEventType::PreCompact => Ok(()), // <--- ADDED PreCompact here
        _ => Err(CupcakeError::Config(format!(
            "inject_context action is only valid for UserPromptSubmit, SessionStart, and PreCompact events, not {hook_event}. ..."
        ))),
    }
    // ...
    ```

- **REASONING:**
  - **WHY:** Our own reconnaissance (`PreCompact` log) proved `PreCompact` supports `stdout` injection. The previous validation blocked a legitimate and powerful use case. This unlocks full capability.
  - **HOW:** This direct modification to the `loader` allows policies with `inject_context` for `PreCompact` to be loaded and validated, aligning our internal rules with the external specification.
  - **WHERE:** `PolicyLoader` is the gatekeeper for policy validity. This ensures only valid policies, per the full spec, are loaded.

DO AFTER:

- **Action:** Run `cargo test --workspace`.
- **Maneuver:** Systematically debug and fix any remaining test failures. The root cause will likely be in the new `evaluate` logic. Do not proceed until the entire suite is green.
- **Verification:** Command `cargo test --workspace` exits with code 0.

* **Task 1.3: Integrated Documentation.**
  - **Action:** Add comprehensive `///` comments to the `evaluate` function in `evaluation.rs`, detailing the spec-compliant matching logic and the role of `get_match_query`.
  - **Verification:** Documentation is clear and committed alongside the code changes.

---

#### **Phase 2: REINFORCE THE FORTRESS (Architectural Hardening)**

**Commander's Intent:** Eradicate architectural ambiguities and centralize hook-specific logic into modular, self-contained units. The codebase will become more explicit and easier to reason about.

- **Task 2.1: Implement Modular, Spec-Compliant Response Generation.**

  - **Intel:** The official Claude Code specification for JSON output is complex and highly nuanced per hook. Refer to `context/claude-code-docs/hooks.md`, specifically the "Advanced: JSON Output" section, as your ground truth for this entire task. Our current `ResponseHandler` at `src/engine/response.rs` is too generic to handle this complexity reliably.

  - **Action 2.1.1: Dismantle the Monolithic `CupcakeResponse` Methods.**

    - **Location:** `src/engine/response.rs`.
    - **Maneuver:** **DELETE** the following static methods from the `impl CupcakeResponse` block. They are being replaced by a superior, modular system:
      - `from_pre_tool_use_decision`
      - `with_context_injection`
      - `stop`
      - `from_decision_block`
      - `from_user_prompt_decision`
      - `from_generic_decision`
    - **Verification:** The project will fail to compile. This is expected.

  - **Action 2.1.2: Establish the Modular Response Arsenal.**

    - **Maneuver:** Create directory `src/engine/response/claude_code/`.
    - **Maneuver:** Create file `src/engine/response/claude_code/mod.rs`.
    - **Maneuver:** In `src/engine/response/mod.rs`, add the line `pub mod claude_code;`.

  - **Action 2.1.3: Construct Specialized Response Builders.**

    - **Maneuver:** Create the following three new files with the exact contents provided. This code is the direct, spec-compliant implementation for each unique JSON contract.

    - **(File 1) `src/engine/response/claude_code/pre_tool_use.rs`:**

      ```rust
      // Builds the hookSpecificOutput.permissionDecision structure for PreToolUse events.
      use crate::engine::response::{CupcakeResponse, EngineDecision, HookSpecificOutput, PermissionDecision};

      pub fn build_response(decision: &EngineDecision) -> CupcakeResponse {
          let (permission_decision, reason) = match decision {
              EngineDecision::Allow { reason } => (PermissionDecision::Allow, reason.clone()),
              EngineDecision::Block { feedback } => (PermissionDecision::Deny, Some(feedback.clone())),
              EngineDecision::Ask { reason } => (PermissionDecision::Ask, Some(reason.clone())),
          };

          CupcakeResponse {
              hook_specific_output: Some(HookSpecificOutput::PreToolUse {
                  permission_decision,
                  permission_decision_reason: reason,
              }),
              ..CupcakeResponse::empty()
          }
      }
      ```

    - **(File 2) `src/engine/response/claude_code/feedback_loop.rs`:**

      ```rust
      // Builds the top-level "decision":"block" and "reason" fields for PostToolUse, Stop, and SubagentStop.
      use crate::engine::response::{CupcakeResponse, EngineDecision};

      pub fn build_response(decision: &EngineDecision) -> CupcakeResponse {
          match decision {
              EngineDecision::Block { feedback } => CupcakeResponse {
                  decision: Some("block".to_string()),
                  reason: Some(feedback.clone()),
                  ..CupcakeResponse::empty()
              },
              _ => CupcakeResponse::empty(), // Allow/Ask results in an empty {} response.
          }
      }
      ```

    - **(File 3) `src/engine/response/claude_code/context_injection.rs`:**

      ```rust
      // Builds the unique JSON for UserPromptSubmit and SessionStart, handling "additionalContext" and "continue":false.
      use crate::engine::response::{CupcakeResponse, EngineDecision, HookSpecificOutput};

      pub fn build_response(decision: &EngineDecision, context: Option<String>) -> CupcakeResponse {
          match decision {
              EngineDecision::Block { feedback } => {
                  CupcakeResponse {
                      continue_execution: Some(false),
                      stop_reason: Some(feedback.clone()),
                      ..CupcakeResponse::empty()
                  }
              }
              _ => { // Allow or Ask
                  let hook_specific_output = context.map(|ctx| {
                      HookSpecificOutput::UserPromptSubmit { additional_context: Some(ctx) }
                  });
                  CupcakeResponse {
                      hook_specific_output,
                      ..CupcakeResponse::empty()
                  }
              }
          }
      }
      ```

    - **Maneuver:** In `src/engine/response/claude_code/mod.rs`, add the following lines to make the builders accessible:
      ```rust
      pub mod pre_tool_use;
      pub mod feedback_loop;
      pub mod context_injection;
      ```

  - **Action 2.1.4: Refactor the Central `ResponseHandler`.**

    - **Location:** `src/engine/response.rs`.
    - **Maneuver:** The `ResponseHandler` is now a clean dispatcher. Replace the logic in `send_response_for_hook_with_suppress` with the following. Note that this function is now only for standard JSON responses. The dual-mode `stdout`/JSON hooks will be handled directly in `run/mod.rs`.

      ```rust
      // in src/engine/response.rs, inside ResponseHandler impl
      pub fn send_standard_response(&self, decision: EngineDecision, hook_event: &str, suppress_output: bool) -> ! {
          let mut response = match hook_event {
              "PreToolUse" => claude_code::pre_tool_use::build_response(&decision),
              "PostToolUse" | "Stop" | "SubagentStop" => claude_code::feedback_loop::build_response(&decision),
              // Generic fallback for simple hooks like Notification.
              _ => if let EngineDecision::Block { feedback } = decision {
                  CupcakeResponse::stop(feedback) // Assumes a generic 'stop' method is re-added for simplicity.
              } else {
                  CupcakeResponse::empty()
              },
          };

          if suppress_output {
              response.suppress_output = Some(true);
          }

          self.send_json_response(response);
      }
      ```

    - **Maneuver:** Refactor `run/mod.rs` to call `send_standard_response` for all standard hooks, and to directly call the new builders (e.g., `claude_code::context_injection::build_response`) for the special-case hooks when a JSON response is required.

  - **Verification:**
    - Create `tests/features/response_generation.rs`.
    - Add unit tests that call each new builder and `assert_eq!` the serialized JSON against the literal examples from `context/claude-code-docs/hooks.md`.

- **Task 2.2: Eliminate the `HookEvent` Alias.**

  - **Intel:** The alias `HookEvent` for `ClaudeCodeEvent` exists in `src/engine/events/mod.rs`. It obscures our new, clear `AgentEvent -> ClaudeCodeEvent` hierarchy.
  - **Action:** Delete the `HookEvent` type alias from `src/engine/events/mod.rs`.
  - **Action:** Perform a case-sensitive, whole-word search and replace across the entire project (`src/` and `tests/`) for `HookEvent` -> `ClaudeCodeEvent`.
  - **Action:** Pay special attention to the `run` function signature in `src/cli/commands/run/engine.rs`. It must change from `hook_event: &HookEvent` to `hook_event: &ClaudeCodeEvent`.
  - **Verification:**
    - `cargo check --workspace` must compile cleanly.
    - `cargo test --workspace` must pass with 100% success.

- **Task 2.3: Integrated Documentation.**
  - **Action:** Add `///` comments to the new files in `src/engine/response/claude_code/`, explaining their purpose is to ensure 100% spec-compliant JSON generation.
  - **Action:** Update `src/engine/events/claude_code/README.md`. For each hook, add a "Response Generation" bullet point that links to its corresponding builder function. E.g., `Response Generation: See src/engine/response/claude_code/pre_tool_use.rs`.
  - **Verification:** Documentation is clear, accurate, and provides a traceable path from event definition to response generation.

---

#### **Phase 3: WRITE THE DOCTRINE (Final Consolidation & Communication)**

**Commander's Intent:** Ensure the victory is permanent through exhaustive documentation updates and a final, disciplined review of our force posture.

- **Task 3.1: Exhaustive Documentation Sweep.**

  - **Action:** Perform a side-by-side review and update of:
    1.  `docs/policy-format.md`
    2.  `docs/conditions-and-actions.md`
    3.  The new `docs/events/claude_code.md`
  - **Maneuver:** Verify every field, every example, and every explanation against the final, working codebase. Ensure concepts like `tool_response.*` and the nuances of matchers are explained with perfect clarity for the end-user. Add a `## Last Verified: YYYY-MM-DD` timestamp to the top of each of these files.
  - **Verification:** A second engineer (or Command) reviews the documentation changes for clarity and accuracy.

- **Task 3.2: Final Health and Quality Check.**

  - **Action:** Run `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
  - **Action:** Run `cargo fmt --all -- --check`.
  - **Verification:** Both commands must exit with code 0. The codebase is not just functional; it is professional.

- **Task 3.3: Final Mission Review.**
  - **Action:** Review the `plan-025-completed.md` document. Update it to reflect the execution of Operation PHOENIX FIRE.
  - **Verification:** The document tells a true and complete story of the entire campaign, including the initial failure and the successful recovery. It serves as a valuable piece of institutional knowledge.

```

```
