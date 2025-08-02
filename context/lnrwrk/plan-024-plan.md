## Plan for plan-024: Full Feature Parity & Guidance Enablement

### **Vision: Beyond Enforcement, Towards Proactive Guidance**

Cupcake's purpose is not merely to be a policy engine that says "no." Its true power lies in being an **integration harness** that enables advanced, context-aware agent behavior. We will evolve Cupcake from a reactive guardrail system into a proactive guidance framework.

This is achieved by focusing on three pillars:

1.  **Advanced Evaluation:** Using `run_command` with custom scripts to make intelligent, dynamic decisions based on the live state of the codebase, external APIs, or other data sources.
2.  **Proactive Guidance:** Using `UserPromptSubmit` and `SessionStart` hooks to inject critical, just-in-time context into the agent's awareness, shaping its reasoning _before_ it acts.
3.  **Intelligent Action:** Executing complex, multi-step actions that automate workflows, not just block them.

This plan refines the tool to fully realize that vision.

---

### **Phase 1: Achieve Full Feature Parity with Claude Code Hooks**

First, we must close the documented gaps between Cupcake and Claude Code's native hook capabilities.

**1. Implement `SessionStart` Hook Support:**

- **Action:** Add `SessionStart` to the `HookEventType` enum in `src/config/types.rs`.
- **Action:** Update the `run` command's context builder in `src/cli/commands/run/context.rs` to correctly parse `SessionStart` events, similar to `UserPromptSubmit`.
- **Behavior:** Like `UserPromptSubmit`, this hook's output can inject context into the agent's awareness at the very beginning of a session, perfect for loading project-wide guidelines, recent ticket summaries, or architectural overviews.

**2. Implement Silent Operations (`suppressOutput`):**

- **Problem:** Cupcake actions are always verbose. Claude Code allows hooks to be silent.
- **Action:** Add an optional `suppress_output: bool` field to all action types in the YAML format (`src/config/actions.rs`).
- **Action:** In `src/engine/response.rs`, the `CupcakeResponse` struct will gain the `suppressOutput` field.
- **Action:** The `ResponseHandler` will set this field on the final JSON output if the winning policy's action has `suppress_output: true`.

**3. Implement Silent Auto-Approval:**

- **Problem:** The `allow` action is always logged in the transcript.
- **Action:** A silent approval is achieved by combining the `allow` action with `suppress_output: true`.
- **Example Use Case:** A policy that auto-allows edits to files in a `test/` directory can now do so without cluttering the agent's transcript.

---

### **Phase 2: Finalize the YAML Policy Format & Action Model**

To eliminate ambiguity and increase power, we will finalize the YAML format and action model.

**1. Implement the Final, Unambiguous Action Model:**

- **Action:** Finalize the implementation of `provide_feedback` to ensure it only ever communicates with the user via the transcript.
- **Action:** Implement the new `inject_context` action, including its dynamic `from_command` capability.
- **Action:** Implement a **strict validator** in the policy loader (`src/config/loader.rs`). This validator will cause `cupcake validate` and `cupcake run` to fail if any policy attempts to use `inject_context` in an event other than `UserPromptSubmit` or `SessionStart`. This makes the separation of concerns a hard, un-ignorable rule of the system.

**2. Finalized Action Summary:**

| Action Name        | Category | Purpose & Description                                                                                                                     | Valid Events                                     |
| ------------------ | -------- | ----------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------ |
| `block`            | Hard     | Blocks the operation. The `feedback` field provides the reason to the agent.                                                              | All                                              |
| `allow`            | Hard     | Explicitly allows the operation, bypassing user prompts. Can be made silent with `suppress_output: true`.                                 | All                                              |
| `ask`              | Hard     | Pauses and asks the user for confirmation via the agent UI.                                                                               | All                                              |
| `provide_feedback` | Soft     | Shows a message to the **user** in the transcript. **Does not affect agent reasoning.** Can be made silent with `suppress_output: true`.  | All                                              |
| `inject_context`   | Soft     | Injects context into the **agent's** awareness, affecting its next response. Can be static (`content`) or dynamic (`from_command`).       | **`UserPromptSubmit`** & **`SessionStart` ONLY** |
| `run_command`      | Variable | Executes a command. Can be a "soft" or "hard" action depending on its `on_failure` behavior. The command's `stdout` is shown to the user. | All                                              |

**3. Finalized `inject_context` Syntax:**

```yaml
action:
  type: "inject_context"
  suppress_output: true # Optional: keeps the injected context out of the user transcript
  # One of the following is required:
  content: "Static context string to inject."
  from_command:
    spec:
      mode: array
      command: ["./scripts/get-ticket-context.sh", "{{prompt}}"]
    on_failure: "continue" # or "block"
```

---

### **Phase 3: Enhance Developer Experience (DX)**

**1. Improve `cupcake validate`:**

- **Action:** Integrate a YAML schema validator to provide IDE support (e.g., for VS Code).
- **Action:** Enhance error messages to include file paths and line numbers, especially for the new `inject_context` event validation.

**2. Enhance `cupcake init`:**

- **Action:** Implement the logic for the TUI to parse a `CLAUDE.md` file and suggest initial policies, fulfilling the vision described in `agent-ecosystem-understanding.md`. This turns developer conventions directly into enforceable rules.

**3. Enhance `cupcake inspect`:**

- **Action:** Add a `--verbose` flag to `inspect` that prints the full conditions and action details for each policy, not just a summary.

---

### **Answers to Outstanding Questions (For Reference)**

_(This section is preserved from the original plan for its comprehensive detail.)_
