# Plan for plan 019

Created: 2025-07-21

## Approach

We will perform a focused, multi-phase upgrade of Cupcake's core engine to fully embrace the modern, JSON-based hook contract from the Claude Code July 20 updates. This plan prioritizes elegance and correctness over backward compatibility, allowing us to streamline our internal data models and align them perfectly with the new capabilities.

The strategy is to first re-platform our communication protocol (Phase 1), then implement the single most impactful new feature—context injection (Phase 2), make the system fully usable via the `sync` and `init` commands (Phase 3), and finally add advanced capabilities (Phase 4).

## Steps

### Phase 1: The New Contract - Modernizing the Communication Protocol

This phase is the foundation. We will refactor the core data structures and response logic to speak the new JSON-native language of Claude Code hooks.

1.  **Update the Core Decision Model (`src/engine/response.rs`)**

    - Rename the `PolicyDecision` enum to `EngineDecision` to clarify its role as an _internal_ representation.
    - Add the `Ask { reason: String }` variant to the `EngineDecision` enum.
    - Create a new serializable `CupcakeResponse` struct that mirrors the complete JSON output schema defined in `context/claude-code-docs/july20-updates/hooks.md`. This will include top-level fields like `continue` and `stopReason`, and the nested `hookSpecificOutput` for `permissionDecision`.

2.  **Refactor the Response Handler (`src/cli/commands/run.rs`)**

    - Modify the `RunCommand::send_response_safely` method (or a new replacement) to no longer rely on `process::exit()`.
    - Instead, `RunCommand::execute` will now return a final `CupcakeResponse` object. The `main` function will be responsible for serializing this response to `stdout` and exiting gracefully.
    - This centralizes response generation and removes side effects from deep within the engine, making logic cleaner and more testable.

3.  **Align Action Terminology (`src/config/actions.rs`)**
    - Rename the `Action::Approve` variant to `Action::Allow`. Since we are not maintaining backward compatibility, this change simplifies the code and aligns our domain language with Claude Code's new `permissionDecision` values.

### Phase 2: Proactive Guidance - Implementing Context Injection

With the new communication protocol in place, we will implement the most transformative feature: proactive context injection.

1.  **Create the `InjectContext` Action (`src/config/actions.rs`)**

    - Add a new `InjectContext { context: String }` variant to the `Action` enum. This will be the primitive that policies use to provide guidance.

2.  **Implement Special Handling for `UserPromptSubmit` (`src/cli/commands/run.rs`)**
    - In the `RunCommand::execute` method, introduce a dedicated logic block that runs _only_ when the incoming event is `UserPromptSubmit`.
    - Inside this block, after policy evaluation, aggregate the `context` strings from all matched `InjectContext` actions.
    - If the final decision is to allow the prompt, print the aggregated context string directly to `stdout` before sending the final (likely empty) JSON response. This correctly implements the special `exit 0 + stdout` behavior for this hook.

### Phase 3: The User Workflow - Making It Real

The core logic is now complete; this phase makes it accessible and usable for end-users.

1.  **Implement the `sync` Command (`src/cli/commands/sync.rs`)**

    - Replace the stub implementation with a robust function that:
      - Locates the project's `.claude/settings.local.json` file.
      - Reads and parses the existing JSON, creating the file if it doesn't exist.
      - Defines a canonical Cupcake hook configuration (e.g., a `PreToolUse` hook that calls `cupcake run --event PreToolUse`).
      - Merges this configuration into the existing JSON, preserving other user settings.
      - Writes the updated JSON back to the file.

2.  **Update the TUI (`src/cli/tui/init/`)**
    - Modify `claude_settings.rs` to use the same logic as the `sync` command, ensuring the TUI generates a correct and modern hook configuration from the start.
    - Update `yaml_writer.rs` to generate example policies that use the new `Allow` action and include a sample `UserPromptSubmit` policy with `InjectContext`.

### Phase 4: Power-Ups - Advanced Integrations

With the core system fully functional, we will add support for the remaining advanced features.

1.  **Integrate `$CLAUDE_PROJECT_DIR` (`src/engine/command_executor/mod.rs`)**

    - The `ActionContext` struct (`src/engine/actions.rs`) already ingests environment variables.
    - Ensure that `CLAUDE_PROJECT_DIR` is correctly captured and made available for template substitution within `check` and `run_command` actions via a variable like `{{env.CLAUDE_PROJECT_DIR}}`.

2.  **Add MCP Tool Matching Support (`src/engine/evaluation.rs`)**
    - The `PolicyEvaluator`'s matcher logic already uses regex, which is sufficient.
    - The primary task here is to add documentation and examples to `docs/policy-format.md` demonstrating how to write policies that target MCP tools using patterns like `"mcp__memory__.*"`.

## Technical Decisions

- **Abandon Exit-Code-Based Communication:** We will fully commit to the JSON output format. The `run` command will no longer use different exit codes to signal outcomes; it will always exit 0 and communicate decisions (allow, deny, ask) via a structured JSON payload on `stdout`. This is a cleaner, more extensible, and less error-prone contract.

- **Rename `Approve` to `Allow`:** To maintain conceptual integrity with the new Claude Code documentation, we will rename the `Approve` action and related internal types to `Allow`. This avoids confusion between Cupcake's internal logic and the `permissionDecision` values.

- **Centralize Final Response Generation:** The `RunCommand::execute` function will be the single source of truth for constructing the final `CupcakeResponse`. Actions will return internal decisions (`EngineDecision`), which are then composed into a single, final response at the end of the execution flow.

- **Isolate Context Injection Logic:** The special `stdout` behavior for `UserPromptSubmit` is an anomaly in the hook contract. We will contain this logic within a specific conditional block in `run.rs` to keep the rest of the engine focused on the primary JSON-based communication model.

---

Required documentation updates:
Excellent question. The implementation of `plan-019` will have significant and positive implications for your documentation. The core value proposition of Cupcake is shifting from a purely reactive enforcer to a proactive guide, and the documentation must be updated to reflect this powerful new paradigm.

Yes, your documentation will need to change. Here is a detailed breakdown of the required updates, with a special focus on the critical `conditions-and-actions.md` file.

### Summary of Documentation Changes

The primary changes will be to:

1.  Introduce the new **`inject_context`** and **`ask`** actions.
2.  Rename the `approve` action to **`allow`** for clarity and consistency.
3.  Update the description of the response mechanism from an exit-code model to the new **JSON-based communication protocol**.
4.  Add **`UserPromptSubmit`** as a primary, top-tier hook event for proactive guidance.
5.  Incorporate the new **`{{env.CLAUDE_PROJECT_DIR}}`** template variable.

---

### 1. `docs/conditions-and-actions.md` (Critical Updates)

**Why it Changes:** This is the most important document for policy authors. It defines the vocabulary of your policies. It must be updated to include the new actions and accurately describe the new communication model.

**Specific Changes:**

- **Update Action Types Quick Reference:**

  - Add `inject_context` - "Proactively add context to the agent's prompt (`UserPromptSubmit` only)."
  - Add `ask` - "Ask the user for permission before proceeding."
  - Rename `approve` to `allow`.

- **Update Visual Overview Diagrams:**

  - The "Hook Lifecycle and Policy Execution" Mermaid diagram is now incorrect. The response path needs to be updated:
    - **Before:** `Cupcake->>Hook: Exit 2 + Feedback`
    - **After:** `Cupcake->>Hook: Return JSON Response (e.g., {"permissionDecision": "deny", ...})`
  - The "Action Execution Types" diagram needs to be updated to include `InjectContext` (as a soft action) and `Ask` (as a hard action).

- **Update Detailed Action Sections:**

  - **Rename `Approve` Section:** Change the heading from "Approve" to "**Allow**" and update the `type` in the YAML example from `approve` to `allow`.
  - **Add New Section: `Inject Context`:** This is a brand new, critical section.

    ````markdown
    #### Inject Context (UserPromptSubmit Only)

    Proactively guides the agent by adding information to its context _before_ it processes the user's prompt. This is the primary mechanism for providing proactive guidance, reminders, and just-in-time information. This action only has an effect when used with the `UserPromptSubmit` hook event.

    ```yaml
    action:
      type: "inject_context"
      context: "Reminder: All new components must include unit tests and be registered in the main component library."
    ```
    ````

  - **Add New Section: `Ask for Confirmation`:**

    ````markdown
    #### Ask for Confirmation

    Pauses the operation and prompts the end-user for explicit permission to continue. This is useful for actions that are potentially sensitive but not always prohibited, such as editing a configuration file.

    ```yaml
    action:
      type: "ask"
      reason: "You are attempting to edit a production configuration file. Are you sure you want to proceed?" # Shown to user
    ```
    ````

- **Update Field Reference:**

  - In the "Common fields" section, emphasize that the `prompt` field is the primary input for policies running on the `UserPromptSubmit` hook.

- **Add a New Example:**
  - Create a new top-level example showcasing a proactive policy using `UserPromptSubmit` and `inject_context`.

---

### 2. `docs/policy-format.md`

**Why it Changes:** This document defines the high-level structure of policy files. It needs to be updated to include the new actions and hook capabilities.

**Specific Changes:**

- **Update Hook Events List:** The list of events is mostly correct. Ensure `UserPromptSubmit` is described not just as an evaluation point, but as the primary hook for **proactive guidance**.

- **Update Tool Matchers:** Add a note clarifying that for tool-based events (`PreToolUse`, `PostToolUse`), the `*` wildcard can now be used to match all tools, in addition to the existing regex capabilities.

- **Update Actions Section:**

  - Rename the "Auto-approve" section to "**Auto-allow**" and change the action type in the example to `allow`.
  - Add a new entry for "**Inject Context**" with a brief description and a link to the main documentation.
  - Add a new entry for "**Ask for Confirmation**" with a brief description.

- **Update Examples:**

  - The "Prompt Security" example is excellent. Add another example for `UserPromptSubmit` that demonstrates `inject_context` to showcase its guidance capabilities.

    ```yaml
    # New Example for docs/policy-format.md

    ### Proactive Guidance
    UserPromptSubmit:
      "":
        - name: "Remind about testing on new features"
          conditions:
            - type: "pattern"
              field: "prompt"
              regex: "(add|create|implement)\\s.*(feature|component|service)"
          action:
            type: "inject_context"
            context: "AI Reminder: You are creating a new feature. Ensure that you also create corresponding unit and integration tests."
    ```

---

### 3. `README.md`

**Why it Changes:** This is the project's front door. It must reflect the most powerful new capabilities to accurately convey the project's value.

**Specific Changes:**

- **Update Core Features:** Add a new bullet point:

  - "**Proactive Guidance**: Injects context into the agent's prompt to guide behavior before actions are taken, not just block them after."

- **Update Policy Configuration Example:** The current example is good but focuses on blocking. Revise it to include a `UserPromptSubmit` policy. This is the single best way to showcase the new philosophy.

  - **Before:** The example only shows `PreToolUse` and `Write|Edit` policies.
  - **After:** Keep the `PreToolUse` example for `Require passing tests...` but replace the `Write|Edit` example with the "Prompt Security" or "Proactive Guidance" example from `policy-format.md`. This immediately demonstrates the new capabilities.

- **Update Integration Section:** The "Response handling" subsection is now incorrect.
  - **Before:**
    > - **Exit code 0**: Soft feedback (transcript only)
    > - **Exit code 2**: Hard block (Claude sees feedback)
  - **After:**
    > - **JSON Output**: Cupcake communicates all decisions (allow, deny, ask) and feedback to Claude Code via a structured JSON payload, enabling richer interactions.
    > - **Context Injection**: For the `UserPromptSubmit` hook, Cupcake can directly add text to the agent's context, providing proactive guidance.

---

### 4. `docs/command-execution.md` & `docs/secure-command-execution.md`

**Why it Changes:** These documents are largely correct, but the context available to commands has expanded.

**Specific Changes:**

- **Update Template Variables Table:** In the table of available template variables, add `{{env.CLAUDE_PROJECT_DIR}}`.
  ```markdown
  | Variable                     | Description                                      |
  | ---------------------------- | ------------------------------------------------ |
  | ...                          | ...                                              |
  | `{{env.CLAUDE_PROJECT_DIR}}` | The absolute path to the project root directory. |
  ```
- This change should be made in both `command-execution.md` and `secure-command-execution.md` for consistency.

**backwards compatibility is NOT required - no migrations necessary - full update granted, remove old/unused code**
