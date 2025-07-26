The good news is that the stateful logic is well-encapsulated. Removing it can be done cleanly without disturbing the core, stateless functionality that has been successfully remediated. The `StateManager` itself can and should remain, as it's still used by the `update_state` action and for general session tracking. We will only be removing the ability to _query_ that state from within a policy condition.

Here is the detailed, surgical plan to remove the `StateQuery` feature.

---

### **Remediation Plan: Removing the `StateQuery` Feature**

**Objective:** To completely remove the `state_query` condition and its associated logic, simplifying the policy engine to be purely stateless for the time being.

**Guiding Principle:** The removal will be compiler-driven. By deleting the core `StateQuery` definition first, the Rust compiler will guide us to every dependent piece of code that needs to be removed, ensuring a clean and complete operation.

---

### **Phase 1: Core Logic Removal (The Surgical Cut)**

We will start by removing the feature's definition, which will cause a cascade of compilation errors that we will then fix sequentially.

#### **Step 1.1: Delete the `StateQuery` Condition**

- **File:** `src/config/conditions.rs`
- **Action:**

  1.  Delete the `StateQuery` variant from the `Condition` enum.
  2.  Delete the entire `StateQueryFilter` struct.
  3.  Delete the `default_expect_exists` helper function.

- **Result:** The project will no longer compile. This is expected and desired.

#### **Step 1.2: Remove the `StateQuery` Evaluation Logic**

- **File:** `src/engine/conditions.rs`
- **Action:**
  1.  The compiler will now show a "non-exhaustive patterns" error in the `evaluate` method's `match` statement. Remove the `Condition::StateQuery` match arm.
  2.  Delete the entire `evaluate_state_query` method, as it is now dead code.
  3.  In the `EvaluationContext` struct, delete the `full_session_state: Option<crate::state::types::SessionState>` field. It was added solely for this feature.

#### **Step 1.3: Remove the State Loading Logic in the `run` Command**

- **File:** `src/cli/commands/run.rs`
- **Action:**
  1.  In the `RunCommand::execute` method, find the block of code responsible for loading the session state. It starts with a check like `let needs_state = ...`.
  2.  Delete this entire block of logic, including the call to `state_manager.get_session_state()`.
  3.  Delete the `condition_uses_state_query` helper method at the end of the file. It is now obsolete.

#### **Step 1.4: Remove the Query Engine Module**

- **File:** `src/state/query.rs`
- **Action:** Delete this entire file. It contains the `StateQuery` engine, which is no longer used.
- **File:** `src/state/mod.rs`
- **Action:** Remove the line `pub use query::StateQuery;`.

---

### **Phase 2: Test Suite Cleanup**

With the feature removed, we must also remove its tests to keep the test suite accurate.

#### **Step 2.1: Delete Stateful Test Files**

- **Action:** Delete the following files from the `tests/` directory:
  - `tests/stateful_context.rs`
  - `tests/stateful_policies.rs`

#### **Step 2.2: Clean Up Remaining Test Code**

- **File:** `tests/july20_features_test.rs`
- **Action:** Delete the `test_state_query_condition_parsing()` test function.
- **Action:** Run `cargo test`. The compiler will flag any other remaining usages of the deleted code in the test suite. Fix any compilation errors by removing the obsolete test code.

---

### **Phase 3: Documentation Scrub**

Finally, we must remove all user-facing documentation for the feature.

#### **Step 3.1: Update `README.md`**

- **File:** `README.md`
- **Action:**
  1.  In the "Core Features" list, remove the line: `- **Stateful Workflows**: Track tool usage history and enforce time-based policies`.
  2.  Remove the entire "Stateful Workflows with StateQuery" section, including its YAML example.

#### **Step 3.2: Update `docs/` Directory**

- **File:** `docs/conditions-and-actions.md`
- **Action:** In the "Condition Types" quick reference list, remove the `state_query` item.

- **File:** `docs/mcp-tool-patterns.md`
- **Action:** Remove the "State-Aware MCP Policies" section and its example.

- **Action (Global):** Perform a full-text search for `state_query` across the `docs/` directory and `README.md` to ensure no references remain.

---

### **Verification / Definition of Done**

The removal is complete when all of the following are true:

- ✅ The project compiles without errors (`cargo check`).
- ✅ The entire test suite passes (`cargo test`).
- ✅ A project-wide search for `StateQuery` in the `src/` directory yields zero results.
- ✅ The test files `tests/stateful_context.rs` and `tests/stateful_policies.rs` have been deleted.
- ✅ A project-wide search for `state_query` in the `docs/` directory and `README.md` yields zero results.

By following this plan, you will have successfully simplified the codebase by removing the stateful query feature while keeping the core state tracking mechanism in place for potential future use.
