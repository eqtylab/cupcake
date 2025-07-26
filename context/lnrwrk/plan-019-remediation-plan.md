Of course. Here is a detailed, phased remediation plan designed to guide the developer through fixing the issues correctly, with clear verification steps at each stage to ensure the work is complete and correct.

This plan follows the logic of the original `plan-019` and is designed to be executed sequentially. Each phase builds a stable foundation for the next.

---

### **Cupcake Remediation Plan: Completing the July 20 Hooks Update**

**Primary Objective:** To correctly and completely implement the Claude Code July 20 hooks update, replacing the current broken hybrid implementation with a modern, JSON-native, and fully-featured system as originally envisioned in `plan-019`.

**Guiding Principles:**

- **No Backward Compatibility:** We are targeting the new API exclusively. All old code and logic must be removed.
- **JSON is the Contract:** All communication with Claude Code hooks (except for `UserPromptSubmit` context injection) will be via structured JSON on `stdout`. Exit codes will no longer be used to communicate decisions.
- **Verify at Every Step:** Each phase concludes with a clear "Definition of Done" that includes specific tests. Do not proceed to the next phase until the current one is fully verified.

---

### **Phase 1: Fix the Core Communication Protocol**

**Objective:** Make Cupcake speak the correct JSON-based language for all policy decisions. This is the most critical fix and must be done first.

**Rationale:** The current mix of exit codes and JSON for decisions is fundamentally broken. Nothing else will work correctly until the communication layer is sound.

#### **Step-by-Step Implementation:**

1.  **Refactor the Response Handler:**

    - Navigate to `src/engine/response.rs`.
    - Locate the `ResponseHandler::send_response` method.
    - **Remove all `process::exit()` calls from this method.** Its only job should be to translate an `EngineDecision` into a `CupcakeResponse` object.
    - Modify the logic to handle **all** `EngineDecision` variants (`Allow`, `Block`, `Approve`, `Ask`) by creating the appropriate `CupcakeResponse` object using the existing helper methods (like `from_pre_tool_use_decision`).
    - Ensure this method now delegates the final sending and exit to `send_json_response`.

2.  **Implement `UserPromptSubmit` Special Case:**

    - In the `RunCommand::execute` method (`src/cli/commands/run.rs`), add logic to handle the special case for `UserPromptSubmit` events.
    - If the final decision is `Allow` and there is context to be injected (from an `InjectContext` action), the code must:
      1.  Print the collected context string to `stdout`.
      2.  Exit with code `0`.
    - This is the _only_ place where a non-JSON response to `stdout` is permitted.

3.  **Align Terminology:**
    - In `src/config/actions.rs`, rename the `Action::Approve` variant to `Action::Allow`.
    - Perform a project-wide search and replace to update all usages of `Approve` to `Allow`.

#### **Verification / Definition of Done:**

- ✅ All existing unit tests in `response.rs` and `actions.rs` must pass.
- ✅ **Create a new integration test:** In `tests/cli_integration_tests.rs`, create a test that runs `cupcake run` with simple policies that trigger `allow` and `block` decisions. The test must capture `stdout`, parse it as JSON, and assert that the `permissionDecision` field is correctly set to `"allow"` or `"deny"`. **Do not check the exit code.**
- ✅ Manually run a policy with an `InjectContext` action on a `UserPromptSubmit` event and confirm the context string is printed to `stdout` and the process exits `0`.

---

### **Phase 2: Fix the User Workflow (`sync` Command)**

**Objective:** Enable users to correctly integrate Cupcake with Claude Code by fixing the `sync` command.

**Rationale:** The tool is useless if users cannot register its hooks. This is the second most critical failure.

#### **Step-by-Step Implementation:**

1.  **Implement Correct JSON Structure:**

    - Navigate to `src/cli/commands/sync.rs`.
    - Modify the JSON generation logic to produce the correct, modern structure as defined in `context/claude-code-docs/july20-updates/hooks.md`.
    - The structure for each hook event (e.g., `PreToolUse`) must be an **array of matcher objects**. Each matcher object must contain a `hooks` key with an **array of hook command objects**.
    - Example of the correct structure to generate:
      ```json
      {
        "hooks": {
          "PreToolUse": [
            {
              "matcher": "*",
              "hooks": [
                {
                  "type": "command",
                  "command": "cupcake run --event PreToolUse"
                }
              ]
            }
          ]
        }
      }
      ```

2.  **Fix Command and Timeout:**
    - Ensure the generated command is `cupcake run --event <EventName>`.
    - Ensure any `timeout` field is specified in **seconds**, not milliseconds.

#### **Verification / Definition of Done:**

- ✅ **Create a new integration test for `sync`:**
  1.  The test should create a temporary directory.
  2.  Run `cupcake sync`.
  3.  Read the generated `.claude/settings.json` file.
  4.  Parse the file content as JSON.
  5.  Assert that the JSON structure is valid and matches the July 20 specification exactly.
- ✅ Verify that running `sync` on a file with existing user settings correctly merges the Cupcake hooks without destroying the other settings.

---

### **Phase 3: Implement the Missing `Ask` Action**

**Objective:** Fully implement the `ask` permission feature, a key part of the July 20 update.

**Rationale:** This feature was left half-finished. Completing it delivers on the promise of more nuanced policy control.

#### **Step-by-Step Implementation:**

1.  **Update `Action` Enum:**
    - In `src/config/actions.rs`, add the `Ask { reason: String }` variant to the `Action` enum.
2.  **Update Action Classifier:**
    - In the `action_type()` method in the same file, ensure that `Action::Ask` is classified as a `Hard` action.
3.  **Update Policy Evaluator:**
    - In `src/engine/evaluation.rs`, update `execute_pass_2_cached` to handle the new `Action::Ask`. When it encounters a matching `Ask` action, it should produce a `HardDecision::Ask`.
4.  **Update Response Handler:**
    - In `src/engine/response.rs`, ensure that an `EngineDecision::Ask` is correctly translated into a `CupcakeResponse` with `permissionDecision: "ask"`.

#### **Verification / Definition of Done:**

- ✅ Add a unit test in `src/config/actions.rs` to verify the serialization and classification of the new `Action::Ask`.
- ✅ **Create a new integration test:** Write a simple policy using `action: ask`. Run it against a matching hook event and verify that the JSON output on `stdout` contains `"permissionDecision":"ask"`.

---

### **Phase 4: Documentation Overhaul and Code Cleanup**

**Objective:** Align all user-facing documentation with the new implementation and remove obsolete code.

**Rationale:** Outdated documentation is a critical bug. Dead code from the old implementation creates confusion and maintenance overhead.

#### **Step-by-Step Implementation:**

1.  **Update All Documentation:**
    - Thoroughly review and update `README.md` and all files in the `docs/` directory.
    - **Crucially:** Replace all references to the old exit-code model (`Exit code 0`, `Exit code 2`) with explanations of the new JSON response protocol.
    - Update all diagrams in `docs/conditions-and-actions.md` to reflect the JSON flow.
    - Add documentation for the new `allow` (renamed from `approve`), `ask`, and `inject_context` actions with clear examples.
2.  **Remove Dead Code:**
    - The old logic in `ResponseHandler::send_response` that uses `process::exit()` should now be entirely unused. Delete it.
    - Search the codebase for any other logic related to the old exit-code decision mechanism and remove it.

#### **Verification / Definition of Done:**

- ✅ A peer review has been conducted on all updated documentation files to confirm accuracy and clarity.
- ✅ A global search for `process::exit(2)` in the `src/` directory finds no results related to decision handling.
- ✅ A global search for the old `approve` action name in the documentation yields no results.

With these four phases completed, Cupcake will be a functional, modern, and correctly implemented tool that fully supports the Claude Code July 20 hooks update.
