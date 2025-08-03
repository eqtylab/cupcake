### Claude Code Hooks: An Engineering Overview

Claude Code hooks are user-defined shell commands that execute at specific points in the agent's operational lifecycle. They provide a powerful mechanism to observe, validate, and deterministically influence the agent's behavior.

Hooks operate on a principle of **influence through feedback and decision-making, not direct modification.** They cannot rewrite a user's prompt or alter a tool's parameters in-flight. Instead, they work by:

1.  **Injecting** new information (context or instructions) for Claude to consider.
2.  **Blocking** an action (like a prompt, tool call, or compaction) from proceeding.
3.  **Providing** natural language feedback to Claude when an action is blocked, guiding it to self-correct.
4.  **Forcing** a decision, such as auto-approving a tool call or forcing the agent to continue working.

Hooks receive event-specific data via a JSON payload on `stdin` and communicate back through their exit code or by returning a structured JSON object on `stdout`.

### Hook Reference

#### 1. `SessionStart`

- **Lifecycle Point:** When a new session is started or resumed.
- **Purpose:** To load initial context into the session (e.g., project status, recent changes, standing instructions).
- **Matchers:** `startup`, `resume`, `clear`, `compact`.
- **Interaction:**
  - **Exit Code 0:** The content of `stdout` is injected directly into Claude's context.
  - **Exit Code 2:** Blocking errors are ignored; `stderr` is shown to the user, but the session continues.
  - **JSON Output:** Can provide context via the `additionalContext` field.

#### 2. `UserPromptSubmit`

- **Lifecycle Point:** After a user submits a prompt but before Claude processes it.
- **Purpose:** To validate prompts or add dynamic, prompt-relevant context.
- **Matchers:** Not applicable.
- **Interaction:**
  - **Exit Code 0:** The content of `stdout` is injected into Claude's context along with the user's prompt.
  - **Exit Code 2:** Blocks the prompt from being processed. The original prompt is erased, and `stderr` is shown to the user.
  - **JSON Output:** Can block the prompt or add context via the `additionalContext` field.

#### 3. `PreToolUse`

- **Lifecycle Point:** Before a tool is executed and before the permission system runs.
- **Purpose:** To act as a dynamic permission and validation layer.
- **Matchers:** Regex pattern against the tool name (e.g., `Bash`, `Edit|Write`).
- **Interaction:**
  - **Exit Code 0:** Success. The hook's output is not shown, and the tool call proceeds to the permission check.
  - **Exit Code 2:** Blocks the tool call. The content of `stderr` is fed back to Claude as an instruction on why the action was blocked.
  - **JSON Output:** Can bypass permissions (`allow`), force a user prompt (`ask`), or block the tool call (`deny`) with feedback.

#### 4. `PostToolUse`

- **Lifecycle Point:** Immediately after a tool completes successfully.
- **Purpose:** For logging, notifications, or follow-up actions like code formatting.
- **Matchers:** Regex pattern against the tool name.
- **Interaction:**
  - **Exit Code 0:** Success. The content of `stdout` is shown to the user in the transcript view (Ctrl-R) for auditing but is not sent to Claude.
  - **Exit Code 2:** Feeds back to Claude. The content of `stderr` is sent to Claude as feedback, influencing its next step.
  - **JSON Output:** Can provide feedback to Claude via `decision: "block"` or add new context for Claude via the `additionalContext` field.

#### 5. `Stop` & `SubagentStop`

- **Lifecycle Point:** Right before the main agent or a sub-agent concludes its response.
- **Purpose:** To force the agent to continue working, creating loops for iterative tasks.
- **Matchers:** Not applicable.
- **Interaction:**
  - **Exit Code 0:** Success. The agent stops as normal. The hook's output is not shown.
  - **Exit Code 2:** Blocks stoppage. The content of `stderr` is fed to Claude as an instruction on what to do next.
  - **JSON Output:** Can block stoppage with a `reason` that becomes the new prompt for the agent.

#### 6. `PreCompact`

- **Lifecycle Point:** Before the conversation history is summarized (compacted).
- **Purpose:** To programmatically control and influence the conversation summarization process.
- **Matchers:** `manual` or `auto`.
- **Interaction:**
  - **Exit Code 0:** Success. The content of `stdout` is appended as custom instructions for Claude on how to perform the compaction, helping to preserve critical information.
  - **Exit Code 2:** Blocks the compaction process from running.
  - **JSON Output:** Can provide instructions via a `compactionInstructions` field.

#### 7. `Notification`

- **Lifecycle Point:** When the agent sends a notification (e.g., awaiting permission or input).
- **Purpose:** To trigger custom notification systems (e.g., desktop or Slack notifications).
- **Matchers:** Not applicable.
- **Interaction:** This hook is for triggering side effects only. Its output does not influence the agent.

---

### Important Implementation Details

- **Parallel Execution:** For any single event, all matching hooks run in parallel, not sequentially. Hooks must be written as independent, atomic operations and cannot rely on the side effects or execution order of other hooks for the same event.
- **Command Deduplication:** If multiple matchers for a single event trigger hooks with the exact same command string, that command will only be executed _once_.
- **Permission Precedence:** If multiple `PreToolUse` hooks return conflicting permission decisions, there is a strict order of precedence: **`deny` > `ask` > `allow`**. A single `deny` will always win.
- **Environment Inheritance:** Hooks execute with the full environment of the running Claude Code process, including any sensitive variables. Hook scripts should be reviewed and trusted accordingly.
