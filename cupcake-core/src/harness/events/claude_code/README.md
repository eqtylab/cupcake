# Claude Code Event Payloads

This directory contains the Rust data structures for every hook event sent by the Claude Code agent. It serves as the authoritative, internal reference for how Cupcake parses and understands incoming events from this specific tool.

## Core Principles

1.  **Modularity:** Each hook event has its own dedicated file and `struct`, ensuring that changes to one hook's schema do not affect others.
2.  **Type Safety:** `serde` deserializes incoming JSON directly into these strongly-typed structs, eliminating guesswork and runtime errors.
3.  **Single Source of Truth:** This document, alongside the structs themselves, is the definitive guide for how a Cupcake developer should interact with Claude Code event data.

## Common Data & Traits

All event payloads are unified by a common structure and traits defined in `src/engine/events/claude_code/mod.rs`.

- **`CommonEventData` struct:** Contains fields present in every hook event:
  - `session_id: String`
  - `transcript_path: String`
  - `cwd: String`
- **`EventPayload` trait:** A trait implemented by all payload structs, guaranteeing access to the `CommonEventData`.
- **`InjectsContext` trait:** A marker trait implemented _only_ by payloads for hooks that have special `stdout` handling for context injection (`SessionStart`, `UserPromptSubmit`, `PreCompact`).

---

## Hook Payload Reference

### `PreToolUse` ✅ MIGRATED

- **File:** `pre_tool_use.rs`
- **Struct:** `PreToolUsePayload`
- **Purpose:** Intercept a tool call before it executes to allow, deny, or ask for user permission.
- **Unique Data Fields:**
  - `tool_name: String`
  - `tool_input: serde_json::Value`
- **Helper Methods:**
  - `is_tool(name: &str) -> bool` - Check if this is a specific tool
  - `get_command() -> Option<String>` - Get command from tool input if present
  - `get_file_path() -> Option<String>` - Get file path from tool input if present
  - `parse_tool_input<T>() -> Result<T>` - Parse tool input as specific type
- **Behavioral Nuances:**
  - This is the primary security gate. A `BlockWithFeedback` action from a policy on this event will prevent the tool from running and feed the reason back to the agent for self-correction.

### `PostToolUse` ✅ MIGRATED

- **File:** `post_tool_use.rs`
- **Struct:** `PostToolUsePayload`
- **Purpose:** React to the successful completion of a tool, often for validation, logging, or follow-up actions.
- **Unique Data Fields:**
  - `tool_name: String`
  - `tool_input: serde_json::Value`
  - `tool_response: serde_json::Value`
- **Helper Methods:**
  - `was_successful() -> Option<bool>` - Check if tool execution succeeded
  - `get_output() -> Option<&str>` - Get tool output text
  - `get_error() -> Option<&str>` - Get error message if failed
- **Behavioral Nuances:**
  - The `tool_response` field is critical for policies that need to validate the _outcome_ of a command (e.g., checking for `success: true` or parsing `stdout` for error messages).
  - A `BlockWithFeedback` action here does not prevent the tool from having run, but it injects feedback into the agent's next turn, prompting it to correct its work.
  - This event only fires on successful tool execution - failed tools don't trigger PostToolUse.

### `UserPromptSubmit` ✅ MIGRATED

- **File:** `user_prompt_submit.rs`
- **Struct:** `UserPromptSubmitPayload`
- **Implements:** `InjectsContext`
- **Purpose:** Validate a user's prompt or inject dynamic, prompt-relevant context before the agent processes it.
- **Unique Data Fields:**
  - `prompt: String`
- **Helper Methods:**
  - `prompt() -> &str` - Get the user's prompt
  - `contains(substring: &str) -> bool` - Check if prompt contains text
  - `preview(n: usize) -> &str` - Get first N characters of prompt
- **Behavioral Nuances:**
  - Output from an `InjectContext` action is printed to `stdout` and joined by `\n`. Claude Code consumes this as additional context for the agent's turn.

### `SessionStart` ✅ MIGRATED

- **File:** `session_start.rs`
- **Struct:** `SessionStartPayload`
- **Implements:** `InjectsContext`
- **Purpose:** Load initial, session-wide context for the agent (e.g., project status, standing instructions).
- **Unique Data Fields:**
  - `source: SessionSource` (enum: `Startup`, `Resume`, `Clear`)
- **Helper Methods:**
  - `is_startup() -> bool` - Check if normal startup
  - `is_resume() -> bool` - Check if resumed session
  - `is_clear() -> bool` - Check if after clear command
  - `source_str() -> &'static str` - Get source as string
- **Behavioral Nuances:**
  - Output from an `InjectContext` action is printed to `stdout` and joined by `\n`. Claude Code consumes this as the very first context in a new session.

### `PreCompact` ✅ MIGRATED

- **File:** `pre_compact.rs`
- **Struct:** `PreCompactPayload`
- **Implements:** `InjectsContext`
- **Purpose:** Programmatically influence the conversation summarization process.
- **Unique Data Fields:**
  - `trigger: CompactTrigger` (enum: `Manual`, `Auto`)
  - `custom_instructions: Option<String>`
- **Helper Methods:**
  - `is_manual() -> bool` - Check if manual compaction
  - `is_auto() -> bool` - Check if automatic compaction
  - `instructions() -> Option<&str>` - Get custom instructions if present
- **Behavioral Nuances:**
  - **CRITICAL:** Output from an `InjectContext` action is printed to `stdout`. Claude Code collects all `stdout` from `PreCompact` hooks and **joins them with `\n\n` (double newline)** to form `newCustomInstructions` for the summarizer model. This was verified from the Claude Code SDK source.
  - A `BlockWithFeedback` action will prevent the compaction from running.

### `PermissionRequest` ✅ MIGRATED

- **File:** `permission_request.rs`
- **Struct:** `PermissionRequestPayload`
- **Purpose:** Runs when user is shown a permission dialog. Allows policies to auto-approve or auto-deny on behalf of the user.
- **Unique Data Fields:**
  - `tool_name: String`
  - `tool_input: serde_json::Value`
  - `tool_use_id: String` - Unique identifier for this tool invocation
- **Helper Methods:**
  - `is_tool(name: &str) -> bool` - Check if this is a specific tool
  - `get_command() -> Option<String>` - Get command from tool input if present
  - `get_file_path() -> Option<String>` - Get file path from tool input if present
  - `parse_tool_input<T>() -> Result<T>` - Parse tool input as specific type
  - `tool_use_id() -> &str` - Get the unique tool use identifier
- **Response Format:**
  - Uses nested `decision` object structure:
    ```json
    {
      "hookSpecificOutput": {
        "hookEventName": "PermissionRequest",
        "decision": {
          "behavior": "allow",     // or "deny"
          "updatedInput": {...},   // optional, for allow
          "message": "...",        // optional, for deny (shown to model)
          "interrupt": true        // optional, for deny (stops Claude)
        }
      }
    }
    ```
- **Behavioral Nuances:**
  - PermissionRequest fires when the user is being shown a permission dialog.
  - This is an opportunity to **bypass** the user prompt by auto-approving or auto-denying.
  - **No `ask` behavior** - this hook IS the ask dialog, so asking doesn't make sense.
  - For `allow`: optionally pass `updatedInput` to modify tool parameters before execution.
  - For `deny`: optionally pass `message` (shown to model) and `interrupt` (stops Claude entirely).
  - Supported decision verbs: `halt`, `deny`, `block`, `modify` (no `ask`, no `add_context`).

### `Stop` & `SubagentStop` ✅ MIGRATED

- **Files:** `stop.rs`, `subagent_stop.rs`
- **Structs:** `StopPayload`, `SubagentStopPayload`
- **Purpose:** Force the agent to continue working when it would otherwise conclude its turn, enabling iterative loops.
- **Unique Data Fields:**
  - `stop_hook_active: bool`
- **Helper Methods:**
  - `should_allow_stop() -> bool` - Check if we should allow stop (when active)
  - `is_first_attempt() -> bool` - Check if this is the first stop attempt
- **Behavioral Nuances:**
  - The `stop_hook_active` field is crucial for preventing infinite loops. A policy should typically check if this is `true` and allow the agent to stop if so.
  - A `BlockWithFeedback` action here prevents the agent from stopping, and the feedback message becomes the new prompt for the next turn.

### `Notification` ✅ MIGRATED

- **File:** `notification.rs`
- **Struct:** `NotificationPayload`
- **Purpose:** Trigger external, out-of-band notification systems (e.g., desktop notifications, Slack messages).
- **Unique Data Fields:**
  - `message: String`
- **Helper Methods:**
  - `message() -> &str` - Get the notification message
  - `contains(substring: &str) -> bool` - Check if message contains text
- **Behavioral Nuances:**
  - This hook is for side-effects only. Its output does not influence the agent's behavior. A `BlockWithFeedback` action will be ignored by Claude Code.
