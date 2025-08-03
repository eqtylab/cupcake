This is another excellent document. The framing around the _type of interactivity and control_ is a highly effective way to explain the hook system's philosophy. The core principles and the breakdown for most hooks are precise and well-articulated.

Your analysis is **95% correct and validated.** There is one key area that needs correction, which is the functionality of the `PreCompact` hook. Based on our previously validated overview, its capabilities are more significant than described in this draft.

Here is the breakdown of the validation and the final, corrected version of the document.

### Validation Result

- **Core Philosophy ("Overview of Hook Interactivity"):** **Perfect.** This is the best explanation of the hook system's design.
- **`SessionStart`:** **Correct.**
- **`UserPromptSubmit`:** **Correct.**
- **`PreToolUse`:** **Correct.** The clarification on "indirect modification" is crucial and accurate.
- **`PostToolUse`:** **Correct.**
- **`Stop`/`SubagentStop`:** **Correct.**
- **`Notification` & `PreCompact`:** **Partially Incorrect.**
  - The description for `Notification` is accurate.
  - The description for `PreCompact` is inaccurate. It **can block** the compaction process (with exit code 2) and it **can add context** in the form of instructions for the summarization model (`stdout` or the `compactionInstructions` JSON field).

I have corrected this section and the final summary table to reflect the validated behavior.

---

### Corrected and Validated Document

Here is the final version of the overview, incorporating the necessary corrections for `PreCompact`.

### Overview of Hook Interactivity and Control

Claude Code hooks allow you to intercept the agent's lifecycle to observe, validate, and influence its behavior. The core principle of this interactivity is **influence through feedback and decision-making, not direct modification.**

You cannot use a hook to directly rewrite a user's prompt or change the parameters of a tool call in-flight. Instead, hooks work by:

1.  **Injecting** new information (context or instructions) for Claude to consider.
2.  **Blocking** an action (like a prompt, tool call, or compaction) from proceeding.
3.  **Providing** natural language feedback to Claude when an action is blocked, guiding it to self-correct and try again with a different approach.
4.  **Forcing** a decision, such as auto-approving a tool call or forcing the agent to continue working when it would otherwise stop.

This feedback loop mechanism is what enables "indirect modification" of Claude's behavior.

---

### Breakdown of Control by Hook Type

Here is a detailed look at the specific controls each hook provides:

#### 1. `SessionStart`

- **What It Controls:** The initial context of a new session.
- **Can it Add Context?** **Yes.** This is its primary purpose. The entire `stdout` of the hook script is injected as the very first piece of context in the conversation.
- **Can it Block?** **No.** An error in this hook will be displayed to the user, but the session will still start.
- **Can it Modify?** **N/A.** There is no pre-existing data to modify.

#### 2. `UserPromptSubmit`

- **What It Controls:** The processing of a user's prompt.
- **Can it Add Context?** **Yes.** The `stdout` of the hook script (or `additionalContext` in JSON output) is added to the context _along with_ the user's original prompt.
- **Can it Block?** **Yes.** It can block the user's prompt from ever being sent to Claude.
- **Can it Modify the User Prompt?** **No, not directly.** You cannot rewrite the user's prompt string. You can only _add context to it_ or _block it entirely_.

#### 3. `PreToolUse`

- **What It Controls:** The execution of a tool _before_ it runs. This is the most powerful gatekeeper hook.
- **Can it Add Context?** **No.** Its output is for decision-making, not for adding conversational context.
- **Can it Block?** **Yes.** It can block a specific tool call from executing. The feedback (`stderr` or JSON `reason`) is then sent to Claude.
- **Can it Modify Tool Parameters?** **No, not directly.** You block the call and provide feedback, prompting Claude to generate a _new, corrected tool call_.
- **Other Powers:** It can bypass the user permission system by returning `"permissionDecision": "allow"`.

#### 4. `PostToolUse`

- **What It Controls:** The agent's flow _after_ a tool has already run successfully.
- **Can it Add Context?** **Yes, via feedback.** By returning a JSON object with `"decision": "block"`, the `reason` string is fed to Claude as a new instruction or observation.
- **Can it Block?** **Not the tool itself** (it has already run), but it can interrupt the agent's normal flow by providing corrective feedback.
- **Can it Modify?** **No.** It cannot undo or change the result of the tool that just ran. Its purpose is to react to the outcome.

#### 5. `Stop` & `SubagentStop`

- **What It Controls:** The agent's lifecycle, specifically whether it is allowed to stop processing.
- **Can it Add Context?** **Yes.** If you block the agent from stopping, the `reason` you provide becomes the new prompt that forces it to continue working.
- **Can it Block?** **Yes.** It can block the agent from finishing its turn, effectively creating a loop.
- **Can it Modify?** **N/A.** It only controls the continuation of the agent's loop.

#### 6. `PreCompact`

- **What It Controls:** The conversation summarization process.
- **Can it Add Context?** **Yes.** The `stdout` of the hook script (or `compactionInstructions` in JSON output) is added as instructions for the model on _how_ to perform the summary, allowing you to preserve critical details.
- **Can it Block?** **Yes.** It can block the compaction process from running entirely (with exit code 2).
- **Can it Modify?** **Yes, indirectly.** By providing instructions, it modifies the final summarized text.

#### 7. `Notification`

- **What It Controls:** Triggers external actions (side-effects) in response to a notification event.
- **Can it Add Context?** **No.**
- **Can it Block?** **No.**
- **Can it Modify?** **No.** Its sole purpose is to trigger an external command.

### Summary of Interactivity by Hook

| Hook                      | Adds Context?            | Blocks Actions?            | "Modifies" via Feedback? | Bypasses Permissions? | Triggers Side Effects? |
| :------------------------ | :----------------------- | :------------------------- | :----------------------- | :-------------------- | :--------------------- |
| **`SessionStart`**        | **Yes** (Primary)        | No                         | No                       | N/A                   | Yes                    |
| **`UserPromptSubmit`**    | **Yes**                  | **Yes** (the prompt)       | No                       | N/A                   | Yes                    |
| **`PreToolUse`**          | No                       | **Yes** (the tool)         | **Yes** (indirectly)     | **Yes**               | Yes                    |
| **`PostToolUse`**         | **Yes** (via feedback)   | **Yes** (the agent's flow) | **Yes** (indirectly)     | N/A                   | **Yes** (Primary)      |
| **`Stop`/`SubagentStop`** | **Yes** (as new prompt)  | **Yes** (the agent stop)   | No                       | N/A                   | Yes                    |
| **`PreCompact`**          | **Yes** (as instruction) | **Yes** (the compaction)   | **Yes** (the summary)    | N/A                   | Yes                    |
| **`Notification`**        | No                       | No                         | No                       | N/A                   | **Yes** (Primary)      |
