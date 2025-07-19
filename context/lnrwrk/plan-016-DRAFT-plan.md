# Plan for plan 016: Support Claude Code Hook Updates

Created: 2024-10-27T15:00:00Z

## Approach

The core of this plan is to update Cupcake to be fully compatible with the July 2025 Claude Code hook updates. The approach involves three main pillars:

1.  **Extend Core Data Structures:** Modify the existing `HookEvent` and `CommonEventData` enums/structs to incorporate the new `UserPromptSubmit` event and the `cwd` field. This will be done in a backward-compatible way where possible.
2.  **Adapt Policy Loading:** Rework the policy loader to handle the new "matcher-less" YAML structure for events like `UserPromptSubmit`. This will likely involve a custom deserializer for the `PolicyFragment` type to gracefully handle both `Matcher -> [Policies]` and `[Policies]` structures under a hook event.
3.  **Implement New Logic:** Add the specific evaluation and response logic for `UserPromptSubmit`, including condition evaluation against the `prompt` field and implementing the correct blocking behavior (exit code 2 and JSON `decision: "block"`).

Throughout the process, we will prioritize maintaining the security model of Cupcake, ensuring that new features like prompt inspection are handled safely by the policy engine.

## Steps

### 1. Update Core Data Structures (`src/engine/events.rs`)

- [ ] **Add `cwd` field:** Add a `cwd: String` field to the `CommonEventData` struct.
- [ ] **Add `UserPromptSubmit` event:**
  - Create a new `UserPromptSubmit` variant in the `HookEvent` enum.
  - This variant will contain `common: CommonEventData` and `prompt: String`.
- [ ] **Update `HookEvent` helpers:** Modify `event_name()`, `tool_name()`, and `tool_input()` methods in `HookEvent` to correctly handle the new `UserPromptSubmit` variant.

### 2. Adapt Policy Format and Loading

- [ ] **Update `HookEventType` (`src/config/types.rs`):** Add `UserPromptSubmit` to the `HookEventType` enum.
- [ ] **Adapt `PolicyFragment` (`src/config/types.rs`):**
  - Define an intermediate enum, `PolicyGroup`, that can deserialize as either a `Vec<YamlPolicy>` (for matcher-less events) or a `HashMap<String, Vec<YamlPolicy>>` (for tool-matching events).
  - Update `PolicyFragment` to be `HashMap<String, PolicyGroup>`.
- [ ] **Update `PolicyLoader` (`src/config/loader.rs`):**
  - Modify the `deep_merge_fragment` and `validate_and_flatten` methods to handle the new `PolicyGroup` structure.
  - Internally, matcher-less policies will be treated as having a universal matcher (e.g., an empty string `""`) for consistency within the engine.

### 3. Enhance Policy Engine

- [ ] **Update `EvaluationContext` (`src/engine/conditions.rs`):**
  - Ensure `current_dir` is populated from the hook event's `cwd` field, not the process's current directory.
  - Add an `Option<String>` for `prompt` to the context.
- [ ] **Update `ConditionEvaluator` (`src/engine/conditions.rs`):**
  - Modify the `extract_field` method to recognize and extract the `prompt` field from the `EvaluationContext`.
- [ ] **Update `PolicyEvaluator` (`src/engine/evaluation.rs`):**
  - Ensure the `build_ordered_policy_list` method correctly identifies and includes `UserPromptSubmit` policies when the incoming hook event is `UserPromptSubmit`.

### 4. Implement `UserPromptSubmit` Response Logic

- [ ] **Update `RunCommand` (`src/cli/commands/run.rs`):**
  - In `build_evaluation_context`, correctly populate the new `prompt` field for `UserPromptSubmit` events and the `cwd` field for all events.
  - In `send_response_safely`, add logic to handle the specific exit code and JSON output behaviors for `UserPromptSubmit`.
    - An `exit code 2` should be used for blocking.
    - A `PolicyDecision::Block` for `UserPromptSubmit` should result in `exit code 2` and output the feedback to `stderr`.
- [ ] **Update `PolicyDecision` & `CupcakeResponse` (`src/engine/response.rs`):**
  - Modify `CupcakeResponse` to support the `"decision": "block"` JSON output for `UserPromptSubmit`.
  - Ensure the `ResponseHandler` can generate this specific JSON structure when blocking a `UserPromptSubmit` event.

### 5. Testing and Documentation

- [ ] **Unit Tests:**
  - Add tests in `tests/hook_event_tests.rs` for deserializing `UserPromptSubmit` events and verifying the `cwd` field is present in all events.
  - Add tests for the updated `PolicyLoader` to ensure it correctly parses both matcher-based and matcher-less policy structures in YAML.
- [ ] **Integration Tests:**
  - Create a new test file (`tests/user_prompt_submit_test.rs`) or extend an existing one.
  - Add tests that simulate `UserPromptSubmit` hooks for:
    1.  Blocking a prompt containing secrets (verifying exit code 2 and stderr feedback).
    2.  Allowing a valid prompt (verifying exit code 0).
    3.  Adding context to a prompt (verifying exit code 0 and stdout output).
- [ ] **Update Documentation:**
  - Update `docs/policy-format.md` to document the `UserPromptSubmit` event and the matcher-less YAML structure.
  - Add examples for `UserPromptSubmit` policies.
  - Update the `cupcake init` command (`src/cli/commands/init.rs`) to include an example `UserPromptSubmit` policy in the generated `00-base.yaml`.

## Technical Decisions

- **YAML Parsing for Matcher-less Events:** We will use an intermediate `PolicyGroup` enum with a custom `serde` deserializer. This provides a robust and type-safe way to handle two different YAML structures under the same `HookEvent` key, avoiding complex and fragile logic inside the loader itself.
- **Authoritative `cwd`:** The `cwd` field from the Claude Code hook event will be treated as the authoritative current working directory for policy evaluation. This is more accurate than relying on `std::env::current_dir()`, as the hook's context may differ from where the `cupcake` binary is executed.
- **`UserPromptSubmit` Response Handling:** The logic for handling `UserPromptSubmit` will be centralized in the `RunCommand` and `ResponseHandler`. A `PolicyDecision::Block` for this specific event will trigger the special behavior (exit code 2, stderr output, and potentially a JSON response) required by Claude Code to correctly block and erase the user's prompt.
