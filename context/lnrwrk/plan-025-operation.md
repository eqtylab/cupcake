Yes, sir. Understood. There is no room for error. The lives of our soldiers—the future of this codebase—depend on the clarity and correctness of this plan.

Having reviewed the battlefield intelligence, including the current state of the code, the enemy's specifications (Claude Code docs), and our own strategic objectives (Plan 025), I have formalized the operational plan.

This is Operation **STEEL TEMPEST**.

It is named for the unyielding but elegant structure we will forge (`STEEL`) and the controlled, swift force with which we will implement it (`TEMPEST`).

The objective is victory. Victory means a codebase that is clean, resilient, extensible, and a joy for our developers to work in. Every soldier comes home. No man is left behind, bogged down by technical debt.

---

### **OPERATION STEEL TEMPEST: FINAL BATTLE PLAN**

#### **I. SITUATIONAL AWARENESS (The Battlefield)**

1.  **Enemy Doctrine (Claude Code Specs):** The enemy operates on a complex but predictable set of rules (`hooks.md`). Their event schemas are inconsistent (`PreCompact`'s `stdout` behavior, varying JSON payloads). Our system must be ableto adapt to these inconsistencies without compromising its own internal logic.
2.  **Current Force Posture (Our Codebase):** Our forces are functional but entangled.
    - `src/engine/events.rs` is a single point of failure. A hit there causes cascading damage across the entire unit.
    - `src/cli/commands/run/context.rs` is a logistical bottleneck, forcing a fragile, manual unpacking of supplies (the 9-element tuple).
    - Our testing battalions (`tests/`) are struggling to build fortifications (test data) quickly and reliably, as seen in the `PreCompact` engagement log. This slows down our advance and creates vulnerabilities.
3.  **Strategic Objective (Plan 025):** We must refactor our core event-handling logistics to be modular, type-safe, and ready for future campaigns (multi-tool support). The system must be so clear that even a soldier suffering from battle fatigue ("forgetfulness") can operate it effectively.

#### **II. MISSION PHASES & EXECUTION**

This operation will be executed in four phases. Each phase concludes with a mandatory verification checkpoint. No phase begins until the previous one is secure.

---

#### **Phase 1: SECURE THE PERIMETER (Preparation & Fortification)**

**Objective:** Establish a rock-solid, repeatable testing framework. We will not advance until our supply lines are secure.

- **Task 1.1: Establish Test Data Factory (`tests/common/event_factory.rs`)**

  - **Action:** Create a new `EventFactory` module using a builder pattern. This unit will be responsible for manufacturing perfect, spec-compliant JSON payloads for every Claude Code hook.
  - **Commander's Intent:** Eliminate the guesswork and manual errors seen in the `PreCompact` log. Make creating test data trivial and reliable.
  - **Verification:** The factory itself is unit-tested. It must produce JSON that is bit-for-bit identical to the examples in `hooks.md`.

- **Task 1.2: Deploy Canary Test (`tests/end_to_end_parsing_test.rs`)**

  - **Action:** Create a single, high-level integration test. It will use the `EventFactory` to generate a `PreToolUse` event, pipe it to `cupcake run`, and assert a successful exit code.
  - **Commander's Intent:** This is our early warning system. It runs against the _current_ codebase and must pass. It will be the final test we run at the end of the operation to confirm we haven't broken the big picture.
  - **Verification:** The test passes before any refactoring begins.

- **Task 1.3: Archive Obsolete Intelligence**

  - **Action:** Move the existing internal documentation from `docs/hooks/claude-code/` into an `docs/hooks/_archive/` directory.
  - **Commander's Intent:** We are taking the old maps off the wall to prevent confusion. We will create new, accurate maps as we secure territory.
  - **Verification:** Old documentation is safely archived, preventing operational confusion.

- **Phase 1 Gut Check:** Is it now trivially easy to create test data for any hook? Yes. Is our end-to-end functionality confirmed? Yes. Are conflicting intelligence sources neutralized? Yes. **Proceed to Phase 2.**

---

#### **Phase 2: SPEARHEAD ASSAULT (Core Refactor & Modularization)**

**Objective:** Break the enemy's monolithic front (`events.rs`) and establish a modular, defensible position. We will focus our initial assault on a single, high-value target to prove the strategy.

- **Task 2.1: Establish Forward Operating Base (`src/engine/events/`)**

  - **Action:** Create the new directory structure: `src/engine/events/` with a `mod.rs` and a `claude_code/` subdirectory.
  - **Action:** Define the abstract `AgentEvent` enum in the top-level `mod.rs`. This is our command-and-control structure.
  - **Action:** Relocate the existing `src/engine/events.rs` to `src/engine/events/claude_code/mod.rs`, renaming `HookEvent` to `ClaudeCodeEvent`.
  - **Verifiability:** Code compiles after path updates.

- **Task 2.2: Isolate and Refactor Target: `PostToolUse`**

  - **Action:** Create `src/engine/events/claude_code/post_tool_use.rs`. Define the `PostToolUsePayload` struct.
  - **Action:** In `claude_code/mod.rs`, define the `EventPayload` trait. Implement it for `PostToolUsePayload`.
  - **Action:** Modify the `ClaudeCodeEvent::PostToolUse` variant to hold the new payload struct.
  - **Commander's Intent:** This is the proof-of-concept. By isolating one hook, we validate the entire architectural pattern.

- **Task 2.3: Rebuild Logistics (`run/parser.rs` & `run/context.rs`)**

  - **Action:** The compiler is now our guide. It will report errors at every point of contact.
  - **Action (Parser):** Update `parse_from_stdin` to return `AgentEvent`.
  - **Action (Context Builder):** **This is the main engagement.** Delete the `extract_event_data` function entirely. Re-implement `build_evaluation_context` to take `&AgentEvent` and use `match` statements to cleanly extract data from the strongly-typed payloads.
  - **Verifiability:** The codebase compiles. The Canary Test (Task 1.2) still passes.

- **Task 2.4: Fortify with Knowledge (Documentation) - SINGLE SOURCE OF TRUTH**

  - **Action:** Create `src/engine/events/claude_code/README.md` as the **authoritative technical reference** for hook implementation within Cupcake.
  - **Action:** Each hook section MUST include:
    1. **Purpose** (one sentence)
    2. **Unique Data Fields** (complete list)
    3. **Behavioral Nuances** (special handling, output format, etc.)
  - **Action:** Document PostToolUse with special attention to `tool_response` field and validation capabilities.
  - **Commander's Intent:** Any soldier can open this single file and understand exactly how each hook works within our system. No cross-referencing required.

- **Phase 2 Gut Check:** Is the `context.rs` file dramatically simpler and safer? Yes. Is the `PostToolUse` logic now completely isolated? Yes. **Proceed to Phase 3.**

---

#### **Phase 3: SWEEP AND CLEAR (Full Migration & System Hardening)**

**Objective:** Methodically apply the winning strategy across the entire battlefield, eliminating all remaining pockets of resistance (brittle code).

- **Task 3.1: Roll Out Modular Payloads**

  - **Action:** One by one, create the payload files (`pre_tool_use.rs`, `pre_compact.rs`, etc.) for all remaining hooks.
  - **Action:** For each, update the `ClaudeCodeEvent` enum, implement the `EventPayload` trait, and add a section to the `README.md`.
  - **Commander's Intent:** This is a systematic sweep. We will be disciplined, migrating one unit at a time, running tests after each one. This minimizes risk.
  - **Verifiability:** All tests pass after each individual hook migration.

- **Task 3.2: Modernize the Testing Arsenal - CONSOLIDATE AND STRENGTHEN**

  - **Action:** Refactor all existing tests in the `tests/` directory to use the `EventFactory` created in Phase 1.
  - **Action:** Simultaneously **consolidate related tests** during refactoring:
    - Merge multiple `inject_context_*.rs` files into single `context_injection_test.rs`
    - Consolidate `hook_event_*.rs` files where appropriate
    - Eliminate redundant test files and create focused, comprehensive test suites
  - **Commander's Intent:** We are not just replacing old weapons; we are reorganizing our battalions into more effective, specialized units. This reduces clutter and makes it easier to assess test coverage for any given feature.
  - **Verifiability:** The entire test suite (`just test`) passes with improved organization and maintainability.

- **Task 3.3: Handle the `PreCompact` Nuance Systematically**

  - **Action:** In `claude_code/mod.rs`, define the `InjectsContext` marker trait.
  - **Action:** Implement this trait for `PreCompactPayload`, `UserPromptSubmitPayload`, and `SessionStartPayload`.
  - **Action:** In `run/mod.rs`, replace the fragile string-based `match` with a type-based check to handle the special `stdout` rendering for these hooks. This is where the developer's reverse-engineered knowledge about the `\n\n` joiner for `PreCompact` is permanently and safely encoded.
  - **Verifiability:** The `precompact_functionality_test.rs` and other context-injection tests pass, now relying on a more robust, type-safe mechanism.

- **Phase 3 Health Check:** Is `cargo clippy` 100% clean? Yes. Is the entire test suite green? Yes. **Proceed to Phase 4.**

---

#### **IV. PHASE 4: MISSION COMPLETE (Final Verification & Debrief)**

**Objective:** Confirm mission success and ensure all objectives have been met.

- **Task 4.1: Final Verification Drill**

  - **Action:** Write a final set of targeted integration tests in `tests/new_fields_extraction_test.rs` (or a similar location). These tests will use the `EventFactory` to confirm that policies can correctly match on the unique fields of each hook type (e.g., `tool_response.success`, `trigger`, `stop_hook_active`).
  - **Verifiability:** All new verification tests pass. The Canary Test from Phase 1 still passes.

- **Task 4.2: After-Action Review**
  - **Action:** Conduct a final code review of the entire set of changes.
  - **Gut Check:**
    1.  **Clarity:** Is it immediately obvious where to find the data model for any given hook? (Yes, in its own file.)
    2.  **Extensibility:** Is it clear how to add a new hook for Claude Code? (Yes, create a file, add to enum.)
    3.  **Scalability:** Is it clear how to add support for a new tool, like "GitHub Copilot"? (Yes, create a new `github_copilot` directory.)
    4.  **Maintainability:** If Claude Code changes a hook's schema, is the blast radius of that change minimized? (Yes, it's confined to that hook's file.)

- **Task 4.3: Update Public-Facing Documentation**

  - **Action:** After the refactor is complete and fully verified, create new clear, user-facing documents in `docs/`.
  - **Implementation:**
    - Create new `docs/events/claude-code.md` explaining our support for Claude Code events at high level
    - Update `docs/policy-format.md` to reference newly available fields for conditions (e.g., `tool_response.success`)
  - **Commander's Intent:** Once the war is won, we write the history. This ensures our civilian users receive clear, accurate, and up-to-date field manuals.
  - **Verifiability:** Documentation is complete, accurate, and reflects the new architecture.

---

## **INTELLIGENCE BRIEFING: OPERATIONAL ENHANCEMENTS**

### **Documentation & Testing Posture Analysis**

**Current State Assessment:**
- **Documentation**: Fragmented across multiple sources with contradictory intelligence
- **Testing**: Brittle manual JSON construction across 300+ tests causing maintenance nightmares
- **Risk Level**: High friction leading to developer casualties and operational delays

**Strategic Imperative:**
Operation STEEL TEMPEST addresses not just code architecture but complete operational modernization:

1. **Consolidated Intelligence**: Single source of truth eliminates conflicting battle plans
2. **Modernized Arsenal**: Test factory replaces manual munitions manufacturing
3. **Systematic Knowledge Capture**: Hard-won tribal knowledge (PreCompact stdout behavior) encoded permanently

### **Mission Enhancement Summary**

- **Phase 1**: Added Task 1.3 - Archive obsolete documentation to prevent confusion
- **Phase 2**: Enhanced Task 2.4 - README.md becomes authoritative technical reference with mandatory structure
- **Phase 3**: Enhanced Task 3.2 - Test consolidation during modernization eliminates redundancy
- **Phase 4**: Added Task 4.3 - Public documentation update ensuring civilian field manuals remain current

**Final Assessment:**
This is not just a code refactor. This is a complete modernization of our operational capability - from intelligence gathering to combat readiness. We clear the old way to build the new way.

**Resource Allocation**: We're going in to get the job done.  
**Rollback Procedures**: There is no rollback. We bring all men home.  
**Operational Status**: Active development, adapt and overcome.

---

This is Operation STEEL TEMPEST. It is a disciplined, phased, and verifiable plan that addresses the known weaknesses of our current position. It will leave our forces stronger, more agile, and ready for any future engagement.
