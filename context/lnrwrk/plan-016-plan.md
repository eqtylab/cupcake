# Plan for plan 016: Support Claude Code Hook Updates

Created: 2025-01-18T12:00:00Z
Revised: 2025-01-18T15:30:00Z

## Approach

The core of this plan is to update Cupcake to be fully compatible with the July 2025 Claude Code hook updates. The approach involves three main pillars:

1. **Extend Core Data Structures:** Modify the existing `HookEvent` and `CommonEventData` enums/structs to incorporate the new `UserPromptSubmit` event and the `cwd` field. This will be done in a backward-compatible way where possible.
2. **Adapt Policy Loading:** Modify the policy loader to handle the new "matcher-less" YAML structure for events like `UserPromptSubmit`. We'll use the existing HashMap structure with an empty string `""` as the universal matcher.
3. **Implement New Logic:** Add the specific evaluation and response logic for `UserPromptSubmit`, including condition evaluation against the `prompt` field and implementing the correct blocking behavior (exit code 2 with stderr feedback).

Throughout the process, we will prioritize maintaining the security model of Cupcake, ensuring that new features like prompt inspection are handled safely by the policy engine.

## Steps

### 1. Update HookEventType Enum First (`src/config/types.rs`)

- [ ] **Add `UserPromptSubmit`:** Add `UserPromptSubmit` to the `HookEventType` enum to ensure type safety throughout the implementation.

### 2. Update Core Data Structures (`src/engine/events.rs`)

- [ ] **Add `cwd` field:** Add a `cwd: String` field to the `CommonEventData` struct.
- [ ] **Add `UserPromptSubmit` event:**
  - Create a new `UserPromptSubmit` variant in the `HookEvent` enum.
  - This variant will contain `common: CommonEventData` and `prompt: String`.
- [ ] **Update `HookEvent` helpers:** Modify `event_name()`, `tool_name()`, and `tool_input()` methods in `HookEvent` to correctly handle the new `UserPromptSubmit` variant.

### 3. Adapt Policy Format and Loading

- [ ] **Simplify matcher-less handling (`src/config/loader.rs`):**
  - For events like `UserPromptSubmit`, `Notification`, `Stop`, `SubagentStop` that don't use matchers, use an empty string `""` as the matcher key.
  - This fits the existing `HashMap<String, HashMap<String, Vec<YamlPolicy>>>` structure without needing a new `PolicyGroup` enum.
  - Update `deep_merge_fragment` and `validate_and_flatten` to handle empty string matchers gracefully.

### 4. Enhance Policy Engine

- [ ] **Update `EvaluationContext` (`src/engine/conditions.rs`):**
  - Add `prompt: Option<String>` field to the context.
  - Ensure `current_dir` is populated from the hook event's `cwd` field, not the process's current directory.
- [ ] **Update `ConditionEvaluator` (`src/engine/conditions.rs`):**
  - Modify the `extract_field` method to recognize and extract the `prompt` field from the `EvaluationContext`.
  - Add case for "prompt" field extraction at the top level (not in tool_input).
- [ ] **Update `PolicyEvaluator` (`src/engine/evaluation.rs`):**
  - Ensure the `build_ordered_policy_list` method correctly identifies and includes `UserPromptSubmit` policies when the incoming hook event is `UserPromptSubmit`.
  - Handle empty string matchers properly in the matching logic.

### 5. Implement `UserPromptSubmit` Response Logic

- [ ] **Update `RunCommand` (`src/cli/commands/run.rs`):**
  - In `build_evaluation_context`, correctly populate:
    - The new `prompt` field for `UserPromptSubmit` events
    - The `cwd` field for ALL events from the hook data (not `std::env::current_dir()`)
  - In `send_response_safely`, add specific logic for `UserPromptSubmit`:
    - `PolicyDecision::Block` → exit code 2 with feedback to stderr
    - No JSON response needed for simple blocking case
    - Advanced JSON decision control only if explicitly using that feature
- [ ] **Verify Response Handling (`src/engine/response.rs`):**
  - Current structure should handle UserPromptSubmit blocking correctly
  - No changes needed unless implementing advanced JSON decision control

### 6. Testing and Documentation

- [ ] **Unit Tests:**
  - Add tests in `tests/hook_event_tests.rs` for deserializing `UserPromptSubmit` events
  - Verify the `cwd` field is present and correctly parsed in all event types
  - Test that empty string matchers work correctly in policy loading
- [ ] **Integration Tests:**
  - Create `tests/user_prompt_submit_test.rs` with tests for:
    1. Blocking a prompt containing secrets (verifying exit code 2 and stderr feedback)
    2. Allowing a valid prompt (verifying exit code 0)
    3. Adding context to a prompt (verifying exit code 0 and stdout output)
  - Test cwd field is used correctly for path-based conditions
- [ ] **Update Documentation:**
  - Update `docs/policy-format.md` to document the `UserPromptSubmit` event
  - Add examples showing matcher-less YAML structure:
    ```yaml
    UserPromptSubmit:
      - name: "Block Secrets"
        conditions: [...]
        action: {...}
    ```
  - Update `00-base.yaml` to include a `UserPromptSubmit` example

## Technical Decisions

- **YAML Parsing Simplification:** Instead of creating a new `PolicyGroup` enum, we'll use empty string `""` as the matcher for events that don't have tool matchers. This fits the existing structure and is simpler to implement.
- **Authoritative `cwd`:** The `cwd` field from the Claude Code hook event will be treated as the authoritative current working directory for policy evaluation. All places that create `EvaluationContext` must use the hook's cwd, not `std::env::current_dir()`.
- **Prompt Field Location:** The `prompt` field will be added as a top-level field in `EvaluationContext`, not nested in `tool_input`, since UserPromptSubmit is not a tool event.
- **Simple Response Handling:** For UserPromptSubmit blocking, we'll use the simple exit code 2 + stderr approach. JSON response is only needed for advanced decision control features.

## Key Differences from Draft

1. **Simplified YAML handling** - No new `PolicyGroup` enum needed
2. **Clear cwd authority** - Explicit that hook's cwd overrides process cwd everywhere
3. **Prompt as top-level field** - Not in tool_input since it's not a tool
4. **Simplified response logic** - Focus on exit code 2 + stderr for blocking
5. **HookEventType first** - Update enum before other changes for type safety