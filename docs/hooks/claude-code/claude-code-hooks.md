Based on the provided documentation, here is a technical accounting of every hook supported by Claude Code.

### Hook Execution Lifecycle Overview

Claude Code hooks are user-defined shell commands that execute at specific points in the agent's operational lifecycle. They provide deterministic control over the agent's behavior. The general lifecycle where hooks can intervene is as follows:

1.  **Session Initialization**: `SessionStart`
2.  **User Interaction**: `UserPromptSubmit`
3.  **Tool Execution Cycle**:
    - `PreToolUse` (before a tool runs)
    - `PostToolUse` (after a tool runs successfully)
4.  **Agent State Changes & Notifications**:
    - `Notification` (when awaiting user input or permission)
    - `PreCompact` (before context window is compacted)
5.  **Session/Task Completion**:
    - `SubagentStop` (when a sub-agent task finishes)
    - `Stop` (when the main agent finishes its response)

Hooks receive event-specific data via a JSON payload on `stdin` and can influence Claude's behavior through their exit code or by returning a structured JSON object on `stdout`.

---

### 1. `SessionStart`

- **Lifecycle Point**: Executes when a new session is started or an existing one is resumed. This happens at the very beginning of an interaction.
- **Purpose**: To load initial context into the session. This is useful for providing information about the development environment, recent codebase changes, or active issues from a tracker.
- **Matchers**:
  - `startup`: Triggered on initial startup.
  - `resume`: Triggered by `/resume`, `--resume`, or `--continue`.
  - `clear`: Triggered by `/clear`.
- **Input (stdin)**:
  ```json
  {
    "session_id": "...",
    "transcript_path": "...",
    "cwd": "...",
    "hook_event_name": "SessionStart",
    "source": "startup" // or "resume", "clear"
  }
  ```
- **How it Works**:
  - **Exit Code 0**: Success. The content of `stdout` is injected directly into Claude's context at the beginning of the session. This is a special behavior for this hook.
  - **Exit Code 2**: N/A. Handled as a non-blocking error; `stderr` is shown to the user, but the session continues.
  - **JSON Output (stdout)**: Allows for more structured context injection.
    ```json
    {
      "hookSpecificOutput": {
        "hookEventName": "SessionStart",
        "additionalContext": "My additional context here"
      }
    }
    ```

### 2. `UserPromptSubmit`

- **Lifecycle Point**: Executes after the user submits a prompt but before Claude begins processing it.
- **Purpose**: To validate, modify, or add context to a user's prompt. Use cases include blocking sensitive information, adding current timestamps, or injecting relevant project data based on prompt keywords.
- **Matchers**: Not applicable. The hook runs for every user prompt.
- **Input (stdin)**:
  ```json
  {
    "session_id": "...",
    "transcript_path": "...",
    "cwd": "...",
    "hook_event_name": "UserPromptSubmit",
    "prompt": "User's submitted prompt text"
  }
  ```
- **How it Works**:
  - **Exit Code 0**: Success. Like `SessionStart`, the content of `stdout` is injected into Claude's context along with the user's prompt.
  - **Exit Code 2**: Blocks the prompt. The user's prompt is erased, and the content of `stderr` is displayed to the user. Claude never sees the prompt.
  - **JSON Output (stdout)**: Provides fine-grained control.
    ```json
    {
      "decision": "block", // or undefined
      "reason": "Reason for blocking (shown to user)",
      "hookSpecificOutput": {
        "hookEventName": "UserPromptSubmit",
        "additionalContext": "Context to add if not blocked"
      }
    }
    ```

### 3. `PreToolUse`

- **Lifecycle Point**: Executes after Claude decides to use a tool and generates its parameters, but _before_ the tool is executed and before the permission system runs.
- **Purpose**: To act as a powerful, dynamic permission and validation layer. It can block tool calls, automatically approve them (bypassing user prompts), or provide feedback to Claude to correct its tool usage.
- **Matchers**: Regex pattern matched against the tool name (e.g., `Bash`, `Edit|Write`, `mcp__github__.*`).
- **Input (stdin)**:
  ```json
  {
    "session_id": "...",
    "transcript_path": "...",
    "cwd": "...",
    "hook_event_name": "PreToolUse",
    "tool_name": "Write",
    "tool_input": {
      /* ...tool-specific parameters... */
    }
  }
  ```
- **How it Works**:
  - **Exit Code 0**: Success. The tool call proceeds to the standard permission check.
  - **Exit Code 2**: Blocks the tool call. The content of `stderr` is fed back to Claude as a natural language instruction on why the action was blocked, allowing it to self-correct.
  - **JSON Output (stdout)**: Offers advanced permission control.
    ```json
    {
      "hookSpecificOutput": {
        "hookEventName": "PreToolUse",
        "permissionDecision": "allow" | "deny" | "ask",
        "permissionDecisionReason": "Reason for decision"
      }
    }
    ```
    - `"deny"`: Blocks the tool. `permissionDecisionReason` is fed back to Claude.
    - `"allow"`: Bypasses the permission system and executes the tool. `permissionDecisionReason` is shown to the user.
    - `"ask"`: Forces a manual permission prompt in the UI. `permissionDecisionReason` is shown to the user.

### 4. `PostToolUse`

- **Lifecycle Point**: Executes immediately after a tool has completed successfully.
- **Purpose**: For logging, notifications, or follow-up actions like code formatting. For example, running a linter or formatter (`prettier`, `gofmt`) on a file that was just modified by an `Edit` or `Write` tool.
- **Matchers**: Same as `PreToolUse` (regex pattern against tool name).
- **Input (stdin)**:
  ```json
  {
    "session_id": "...",
    "transcript_path": "...",
    "cwd": "...",
    "hook_event_name": "PostToolUse",
    "tool_name": "Write",
    "tool_input": {
      /* ... */
    },
    "tool_response": {
      /* ... */
    }
  }
  ```
- **How it Works**:
  - **Exit Code 0**: Success. The hook's action completes silently in the background. `stdout` is visible in the transcript view.
  - **Exit Code 2**: Feeds back to Claude. While the tool has already run, this sends the content of `stderr` to Claude as feedback, which can influence its next steps.
  - **JSON Output (stdout)**:
    ```json
    {
      "decision": "block", // or undefined
      "reason": "Feedback to send to Claude"
    }
    ```
    - `"block"`: The `reason` is fed back to Claude as an instruction or observation.

### 5. `Notification`

- **Lifecycle Point**: Executes when Claude Code sends a notification to the user. This occurs when:
  1.  Permission is required for a tool.
  2.  The input prompt has been idle for 60 seconds.
- **Purpose**: To customize notifications. Instead of the default terminal bell or UI message, you can trigger desktop notifications, send a Slack message, etc.
- **Matchers**: Not applicable.
- **Input (stdin)**:
  ```json
  {
    "session_id": "...",
    "transcript_path": "...",
    "cwd": "...",
    "hook_event_name": "Notification",
    "message": "Claude needs your permission to use Bash"
  }
  ```
- **How it Works**:
  - **Exit Code 0**: Success. The custom notification command runs.
  - **Exit Code 2**: N/A. Handled as a non-blocking error; `stderr` is shown to the user only.

### 6. `Stop` & `SubagentStop`

- **Lifecycle Point**:
  - `Stop`: Executes when the main agent has finished its response and is about to stop.
  - `SubagentStop`: Executes when a sub-agent (from a `Task` tool call) has finished its work.
- **Purpose**: To prevent the agent from stopping and force it to continue working. This can create loops for continuous validation or multi-step processes, like running tests, fixing failures, and re-running tests until they pass.
- **Matchers**: Not applicable.
- **Input (stdin)**:
  ```json
  {
    "session_id": "...",
    "transcript_path": "...",
    "hook_event_name": "Stop", // or "SubagentStop"
    "stop_hook_active": true // true if already in a stop hook loop
  }
  ```
- **How it Works**:
  - **Exit Code 0**: Success. The agent is allowed to stop as normal.
  - **Exit Code 2**: Blocks stoppage. The content of `stderr` is fed to Claude as an instruction on what to do next.
  - **JSON Output (stdout)**:
    ```json
    {
      "decision": "block", // or undefined
      "reason": "Must be provided. Instruction for Claude on what to do next."
    }
    ```
    - `"block"`: Prevents the agent from stopping. The `reason` is critical, as it becomes the new prompt for the agent.

### 7. `PreCompact`

- **Lifecycle Point**: Executes just before Claude Code performs a "compact" operation to summarize the conversation history, which happens when the context window is full or when manually triggered.
- **Purpose**: To influence the compaction process or log when it occurs.
- **Matchers**:
  - `manual`: When triggered by the `/compact` command.
  - `auto`: When triggered automatically due to a full context window.
- **Input (stdin)**:
  ```json
  {
    "session_id": "...",
    "transcript_path": "...",
    "hook_event_name": "PreCompact",
    "trigger": "manual", // or "auto"
    "custom_instructions": "User input from /compact command"
  }
  ```
- **How it Works**:
  - This hook is primarily for observation. Its output does not appear to have a special control mechanism over the compaction itself.
  - **Exit Code 2**: N/A. Handled as a non-blocking error; `stderr` is shown to the user only.
