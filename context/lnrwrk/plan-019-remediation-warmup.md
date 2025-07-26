Excellent idea. That's a perfect "documentation-driven" warm-up task. It forces the developer to internalize the new contract before writing a single line of implementation code, which is exactly what's needed to correct their mental model.

Here is a simple, focused, two-part task. This will act as **Phase 0** of the main remediation plan.

---

### **Phase 0: Mindset Shift - Documenting the New Reality**

**Objective:** Update the single most important documentation file, `docs/conditions-and-actions.md`, to accurately reflect the new JSON-based communication protocol. This document will then serve as the "specification" for the first code fix.

#### **Part 1: Update the Documentation (`docs/conditions-and-actions.md`)**

The developer should perform these documentation changes _first_.

1.  **Fix the Core Diagram:** The "Hook Lifecycle and Policy Execution" Mermaid diagram is currently incorrect. It visually reinforces the old, wrong exit-code model.

    - **Find this code block:**
      ```mermaid
      sequenceDiagram
          ...
          alt Hard Action (Block)
              Action->>Cupcake: Block Decision
              Cupcake->>Hook: Exit 2 + Feedback  # <-- THIS IS WRONG
              Hook->>Claude: Tool Blocked + Reason
          ...
      ```
    - **Replace it with this correct version:**
      ```mermaid
      sequenceDiagram
          ...
          alt Hard Action (Block, Ask, Allow)
              Action->>Cupcake: Hard Decision
              Cupcake->>Hook: Return JSON Response (e.g., {"permissionDecision": "deny", ...}) # <-- THIS IS CORRECT
              Hook->>Claude: Tool Blocked/Proceeds + Reason
          ...
      ```

2.  **Update the Action Descriptions:** The text descriptions for the core actions are misleading.

    - **Find the `Block with Feedback` section.**

      - **Current (Wrong):** "Prevents the operation and tells Claude why. Claude will try to correct and continue."
      - **Update to (Correct):** "Prevents the operation by returning a JSON response with `permissionDecision: \"deny\"`. The feedback message is sent to Claude, which will try to correct its action."

    - **Find the `Approve` section.**
      - **Rename the section to `Allow`**.
      - **Current (Wrong):** "Auto-approves the tool use, bypassing Claude Code's permission prompt."
      - **Update to (Correct):** "Auto-allows the tool use by returning a JSON response with `permissionDecision: \"allow\"`. This bypasses Claude Code's permission prompt. The optional `reason` is shown to the user."

3.  **Add the New `Ask` Action:**

    - Create a new section in the documentation for the `Ask` action. This forces engagement with the new capabilities.
    - **Add this new section:**

      ````markdown
      #### Ask for Confirmation

      Pauses the operation and prompts the end-user for explicit permission by returning a JSON response with `permissionDecision: "ask"`. This is useful for actions that are potentially sensitive but not always prohibited.

      ```yaml
      action:
        type: "ask"
        reason: "You are attempting to edit a production configuration file. Are you sure?"
      ```
      ````

      ```

      ```

#### **Part 2: Make the Documentation True (The Code Fix)**

Now that the documentation is correct, the developer's task is simple: **make the code do what the new documentation says.**

1.  **Implement the `Block` Action's New Behavior:**

    - Go to `src/engine/response.rs` in the `ResponseHandler::send_response` method.
    - Look at the `EngineDecision::Block` match arm. It currently calls `eprintln!` and `process::exit(2)`.
    - **Change it** to instead create a `CupcakeResponse` using `CupcakeResponse::from_pre_tool_use_decision(&decision)` and then call `self.send_json_response(response)`. This directly implements what the new documentation describes.

2.  **Implement the `Allow` Action's New Behavior:**
    - In the same method, look at the `EngineDecision::Allow` match arm. It currently calls `process::exit(0)`.
    - **Change it** to also create a `CupcakeResponse` and call `self.send_json_response(response)`.
    - Rename the `EngineDecision::Approve` variant to `EngineDecision::Allow`.

#### **Verification / Definition of Done for Phase 0:**

- ✅ The file `docs/conditions-and-actions.md` has been updated as described above.
- ✅ The `run` command, when a policy triggers a `block_with_feedback` action, now prints a JSON object to `stdout` and exits with code `0`.
- ✅ **A new integration test proves this:** The test runs a `block` policy, captures `stdout`, and asserts that the output is a valid JSON string containing `"permissionDecision":"deny"`.

---

This small, focused task is the perfect starting point. It's low-risk, directly addresses the core conceptual misunderstanding, and uses the documentation as a concrete specification. Completing this will build the right foundation and momentum for the developer to successfully execute the rest of the remediation plan.

---

The documentation changes proposed in "Phase 0" are **perfectly aligned** with the Claude Code July 20 updates.

I can confirm this by cross-referencing the proposed changes directly against the official `july20-updates/hooks.md` documentation you provided. Here is the point-by-point validation:

### 1. The Shift from Exit Codes to JSON for Decisions

- **Proposed Change:** Stop documenting `Exit 2` for blocking and instead document a JSON response with `permissionDecision: "deny"`.
- **Claude Code Docs (`july20-updates/hooks.md`):** Under "Hook Output", the documentation now has two sections: "Simple: Exit Code" and the new "Advanced: JSON Output". The advanced section is the key. For `PreToolUse` hooks, it explicitly defines the new contract:

  ```json
  {
    "hookSpecificOutput": {
      "hookEventName": "PreToolUse",
      "permissionDecision": "allow" | "deny" | "ask",
      "permissionDecisionReason": "My reason here (shown to user)"
    }
  }
  ```

  The documentation states that `permissionDecision: "deny"` **prevents the tool call from executing** and shows the reason to Claude. This is the modern, correct way to block an action.

- **Conclusion:** The proposed change is **correct**. While Claude Code maintains support for `exit 2` for backward compatibility, your project's plan (`plan-019`) explicitly states **"Abandon Exit-Code-Based Communication"**. Therefore, updating your documentation to reflect the JSON-native approach is precisely the right thing to do.

### 2. The New `ask` Action

- **Proposed Change:** Add a new `ask` action to the documentation that returns a JSON response with `permissionDecision: "ask"`.
- **Claude Code Docs (`july20-updates/hooks.md`):** The `permissionDecision` field shown above explicitly includes `"ask"` as a valid option. The documentation describes its behavior as: `"`ask"` asks the user to confirm the tool call in the UI.`

- **Conclusion:** The proposed documentation for an `ask` action is **100% aligned** with this new feature. It's a core part of the July 20 update that Cupcake should be exposing to its users.

### 3. Renaming `approve` to `allow`

- **Proposed Change:** Rename the `approve` action to `allow` to match the new terminology.
- **Claude Code Docs (`july20-updates/hooks.md`):** The new `permissionDecision` field uses the term `"allow"`. The documentation also includes a note clarifying the old terminology is deprecated but supported: `_Deprecated `"approve"`value +`reason` has the same behavior._`

- **Conclusion:** Renaming the action to `allow` is **correct**. It aligns Cupcake's terminology with the new, official terminology from Claude Code, reducing confusion for your users.

In short, the "Phase 0" documentation task is not just a good idea—it's a perfect, small-scale implementation of the new contract. It forces the developer to internalize that decisions are now communicated via a structured JSON object (`{"permissionDecision": "..."}`) sent to `stdout`, not via different process exit codes. This is the single most important conceptual shift required to fix the implementation.
