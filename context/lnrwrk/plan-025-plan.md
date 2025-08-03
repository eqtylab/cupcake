### **Final Implementation Blueprint: Refactoring for a Multi-Tool Policy Engine**

This document outlines the phased implementation plan to refactor Cupcake's event handling system. The primary goals are to eliminate existing architectural friction, improve developer ergonomics, and strategically position the codebase for future support of multiple AI agent ecosystems.

#### **Phase 0: Preparation & Groundwork (The "Measure Twice" Phase)**

This phase is about setting up for success. We do not write any new feature code here.

1.  **Create a Test Data Factory:**

    - **Action:** Create a new module `tests/common/event_factory.rs`.
    - **Goal:** Build a simple, ergonomic way to create valid hook event JSON for testing. This will solve the "testing nightmare" seen in the logs.
    - **Implementation:** Use a builder pattern.

      ```rust
      // Example Usage in a test:
      use crate::common::event_factory::EventFactory;

      let json = EventFactory::pre_tool_use()
          .tool_name("Bash")
          .tool_input_command("ls -la")
          .session_id("test-123")
          .build_json();
      ```

    - **Verifiability:** The factory builds successfully. We can write a single test within the factory module itself that asserts its output matches a known-good JSON string from the Claude Code docs.

2.  **Establish a Baseline Integration Test:**
    - **Action:** Create a new integration test file, `tests/end_to_end_parsing_test.rs`.
    - **Goal:** Create a single test that feeds a known-good `PreToolUse` JSON string (from our new factory) into the `cupcake run` command and asserts that the program exits gracefully (code 0).
    - **Verifiability:** The test runs and passes against the _current, unmodified codebase_. This test will be our canary; if it's green at the end of the refactor, we know we haven't broken the end-to-end data flow.

#### **Phase 1: Building the New Modular Architecture (The "Pour the Foundation" Phase)**

This is the core architectural work. We will build the new structure and migrate a single, representative hook.

1.  **Create the Abstract Event Layer:**

    - **Action:** Create `src/engine/events/mod.rs`. Define the top-level `AgentEvent` enum with only the `ClaudeCode` variant for now.
    - **Verifiability:** The file compiles.

2.  **Create the Claude Code Event Module:**

    - **Action:** Create the `src/engine/events/claude_code/` directory.
    - **Action:** Move `src/engine/events.rs` to `src/engine/events/claude_code/mod.rs`.
    - **Action:** In this new `mod.rs`, define the `EventPayload` trait and the `CommonEventData` struct.
    - **Verifiability:** The project compiles after adjusting `mod` declarations in `src/engine/mod.rs`.

3.  **Migrate the First Hook: `PostToolUse`**

    - **Action:** Create `src/engine/events/claude_code/post_tool_use.rs`. Define the `PostToolUsePayload` struct and implement the `EventPayload` trait.
    - **Action:** In `claude_code/mod.rs`, change the `ClaudeCodeEvent::PostToolUse` variant from `PostToolUse { ... }` to `PostToolUse(PostToolUsePayload)`.
    - **Action:** Add a `README.md` inside the `claude_code` directory. Start with a simple explanation of the `PostToolUse` hook's purpose and any specific nuances (e.g., "Note: The `tool_response` field is critical for validating the outcome of a tool's execution."). This fulfills your documentation suggestion.
    - **Verifiability:** The project will fail to compile. This is expected and desired. The compiler errors are now our to-do list.

4.  **Fix the Compiler Errors (The Guided Refactor):**

    - **Parser (`run/parser.rs`):** Update `parse_from_stdin` to return `Result<AgentEvent>`. It will deserialize to `ClaudeCodeEvent` and wrap it in `AgentEvent::ClaudeCode`.
    - **Context Builder (`run/context.rs`):**
      - This is the most significant change. The giant `extract_event_data` function and its tuple return type are **deleted entirely.**
      - The `build_evaluation_context` function now takes `&AgentEvent`. It will have a `match` on `AgentEvent`. Inside the `AgentEvent::ClaudeCode(event)` arm, it will have another `match` on the `ClaudeCodeEvent` variants.
      - It will extract data directly from the typed payloads (e.g., `ClaudeCodeEvent::PostToolUse(payload) => Some(payload.tool_response.clone())`).
      - This resolves the `clippy` warning and the core architectural flaw.
    - **Verifiability:** The project compiles again. Our baseline integration test from Phase 0 should still pass (as it uses `PreToolUse`, which we haven't touched yet).

5.  **Gut Check:**
    - Does the new structure feel cleaner?
    - Is the logic in `context.rs` easier to read and reason about?
    - Was adding the `README.md` a low-effort, high-value addition?
    - **The answer to all three should be a resounding "yes."**

#### **Phase 2: Full Migration and Cleanup (The "Build the Walls" Phase)**

This phase is methodical and lower-risk, applying the pattern established in Phase 1 to the remaining hooks.

1.  **Migrate Remaining Hooks:**

    - **Action:** One by one, create the payload files for the other hooks (`pre_tool_use.rs`, `stop.rs`, etc.), define their structs, and update the `ClaudeCodeEvent` enum.
    - **Action:** As each hook is migrated, update the `README.md` in the `claude_code` directory with a section for that hook.
    - **Verifiability:** After each hook is migrated, the project should continue to compile and all tests should pass. The developer can commit after each successful hook migration, making the process incremental and safe.

2.  **Update All Tests:**

    - **Action:** Go through the `tests/` directory and update all test data constructors to use the new `EventFactory` from Phase 0. This pays off the initial investment immediately.
    - **Verifiability:** The test suite is now simpler, more readable, and less brittle. All tests pass. The baseline integration test from Phase 0 must still pass.

3.  **Final Health Check:**
    - **Action:** Run `cargo clippy --all-targets --all-features -- -D warnings`.
    - **Verifiability:** The build is 100% clean. The `type_complexity` warning is gone. There are no new warnings.

#### **Phase 3: Final Verification (The "Final Inspection" Phase)**

This phase confirms that we have not only maintained functionality but actively improved it.

1.  **Write New, Targeted Tests:**

    - **Action:** Using the now-simple `EventFactory`, add new tests to `tests/new_fields_extraction_test.rs` (or a similar file) that specifically assert the correct extraction of data for each hook type.
    - **Goal:** Verify that policies can correctly match on fields unique to each hook (e.g., `tool_response.success` for `PostToolUse`, `trigger` for `PreCompact`).
    - **Verifiability:** The new, high-value tests pass. This confirms our `EvaluationContext` is being built correctly from the new modular event structs.

2.  **Final Sanity Check and Review:**
    - **Action:** The developer presents the final Pull Request.
    - **Gut Check:**
      - Is the new `src/engine/events/` structure clear and intuitive?
      - Is the `run/context.rs` file dramatically simpler?
      - Is it obvious how to add a new hook or a new tool in the future?
      - Is the test suite easier to read and maintain?

This phased approach provides verifiability at every step, directly addresses the developer's pain points, and results in a final product that is not just working, but is professionally engineered for the long haul. It is the path to winning.
