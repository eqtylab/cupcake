Plan 026: Continued Hook Refactor

### **STRATEGIC OUTLINE: OPERATION PHOENIX FIRE**

#### **I. CRITICAL CONTEXT: THE BATTLEFIELD**

Comrades, Operation STEEL TEMPEST was a strategic success in architecture but a tactical failure in implementation. We successfully built a world-class, modular event system designed for a multi-agent future. However, in our rapid advance, a critical flaw was overlooked: **the new architecture is not correctly wired into our core policy evaluation engine.** Our beautiful new fortress has no power.

This is a total mission failure until rectified.

Operation PHOENIX FIRE is our counter-offensive. Its purpose is not just to fix the bug, but to rise from the ashes of the last engagement stronger, more disciplined, and with a system that is not just elegant in theory, but flawless in practice. We will not just win this battle; we will set a new standard for how all future operations are conducted.

#### **II. STRATEGIC OBJECTIVES**

This operation is guided by three core objectives that address every weakness identified in the previous campaign.

1.  **RESTORE LETHALITY:** The immediate, non-negotiable objective is to repair the connection between our new event architecture and the policy evaluation engine. **The system must work.** All tests must pass.
2.  **ESTABLISH DOCTRINAL SUPERIORITY:** We will eliminate all remaining architectural inconsistencies and technical debt identified during the last engagement. This includes cleaning up command handling, removing redundant type aliases, and ensuring all logic is clean, modular, and discoverable.
3.  **INTEGRATE AND VERIFY RELENTLESSLY:** We will permanently eliminate sprawl and drift. Documentation and test organization will no longer be end-of-mission cleanup tasks. They will be integrated, continuous actions performed as part of every tactical maneuver. We will leave no man behind, and we will leave no file disorganized.

#### **III. PHASES OF OPERATION**

Operation PHOENIX FIRE will be executed in three disciplined phases. Each phase includes integrated documentation and testing tasks.

---

#### **Phase 1: SECURE THE IGNITION (Critical Bug Fix)**

**Objective:** Restore core functionality. Make the engine fire.

- **Primary Action: Repair the Policy Evaluator.**

  - **Intel:** The core failure is in `src/engine/evaluation.rs`. The `build_ordered_policy_list` function is failing to correctly filter policies (especially those with `""` and `"*"` matchers) against our new, strongly-typed `ClaudeCodeEvent` payloads.
  - **Maneuver:** The fire team will conduct a deep diagnostic, tracing data from the `EventFactory`, through the `ExecutionContextBuilder`, and into the `PolicyEvaluator`. They will identify and neutralize the logical flaw in the matching algorithm.

- **Integrated Verification:**

  - **Testing:** The _only_ metric for success in this phase is a **100% green test suite**. The primary targets are the failing tests in `empty_matcher_test.rs`, but all 116+ tests must pass.

- **Integrated Documentation:**
  - **Action:** Code comments will be added to the `PolicyEvaluator` to clarify the now-correct logic for handling wildcard and empty matchers against tool and non-tool events.

---

#### **Phase 2: REINFORCE THE FORTRESS (Architectural Hardening)**

**Objective:** Eliminate all identified secondary weaknesses and inconsistencies.

- **Primary Action: Decouple Command Handling.**

  - **Intel:** The `run/mod.rs` command handler has become a complex, centralized dispatcher. This is a future bottleneck.
  - **Maneuver:** We will push hook-specific logic down. The `EngineResult` will be enhanced to describe the required output type (e.g., `StandardJson`, `StdoutContext`). The `ResponseHandler` in `src/engine/response.rs` will be upgraded to handle the rendering logic, simplifying the `run` command to a clean, high-level orchestrator.

- **Secondary Action: Eliminate Redundancy.**

  - **Intel:** The `HookEvent` alias is a relic of the refactor that now causes confusion.
  - **Maneuver:** A systematic, codebase-wide sweep will be conducted. All instances of the `HookEvent` alias will be replaced with the explicit `ClaudeCodeEvent`. This will make our new `AgentEvent -> ClaudeCodeEvent` hierarchy unambiguous.

- **Integrated Verification:**

  - **Testing:** New unit tests will be created for the enhanced `ResponseHandler` to verify its new rendering logic for each special hook type (`PreCompact`, `UserPromptSubmit`, etc.). The full test suite must remain 100% green after the `HookEvent` alias removal.

- **Integrated Documentation:**
  - **Action:** The internal `README.md` in `src/engine/events/claude_code/` will be updated to reflect the `InjectsContext` trait and its connection to the new `ResponseHandler` logic. The `run/mod.rs` file will be commented to explain its new, simplified role as an orchestrator.

---

#### **Phase 3: WRITE THE DOCTRINE (Final Polish & Public Communication)**

**Objective:** Ensure the victory is permanent by creating flawless documentation and conducting a final, exhaustive review.

- **Primary Action: Exhaustive Documentation Sweep.**

  - **Intel:** Our public-facing docs have drifted and may contain inconsistencies.
  - **Maneuver:** A full review of all files in `docs/` will be conducted. `policy-format.md`, `conditions-and-actions.md`, and the new `events/claude-code.md` will be scrutinized to ensure they are 100% consistent with the final, working implementation. All examples will be re-validated.

- **Integrated Verification:**

  - **Testing (The Final Stand):** This is our final check. We will run the entire test suite one last time. We will also manually run the examples from our newly updated documentation to ensure they work exactly as described.

- **Final Strategic Review:**
  - We will conduct a final "after-action review" against our strategic objectives. Does the final codebase achieve dominance, elegance, and extensibility? Is the developer experience seamless? Is the foundation ready for future tools?

---

#### **IV. CONCLUSION: THE PATH TO VICTORY**

Operation PHOENIX FIRE is our response to a near-disaster. It is a plan born from hard lessons, designed to be executed with unwavering discipline. By integrating our actions, documentation, and verification at every step, we will not only fix the critical failures but also forge a stronger, more resilient codebase and a more effective operational doctrine.

We will close all gaps. We will push the war back in our favor. This is how we achieve total victory.
