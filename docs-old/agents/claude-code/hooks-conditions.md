# When Claude Sees Output From Hooks — Final, Verified Overview

Claude receives hook feedback via **(1) exit codes** and **(2) JSON on stdout**. What Claude sees (vs. what only the user sees) depends on both the **hook event** and the **output type**.

---

## 1) Exit‑code–based feedback

### Exit code `0` (success)

- **Claude sees `stdout` only for:**

  - `UserPromptSubmit` → `stdout` is injected into Claude’s context.
  - `SessionStart` → `stdout` is injected into Claude’s context.
    _Note:_ The spec’s “Simple: Exit Code” section includes both events; a later warning mentions only `UserPromptSubmit`. Treat both as context‑injection cases.

- **All other events (except `Notification`)**: `stdout` is **not** shown to Claude; it is visible to the user in transcript mode (Ctrl‑R).
- **`Notification`**: Output is **debug‑only** per Hook Execution Details; don’t rely on transcript visibility.

### Exit code `2` (blocking error)

On exit `2`, the hook’s **`stderr`** is handled as follows:

| Hook event         | Effect with exit `2`                                                   |
| ------------------ | ---------------------------------------------------------------------- |
| `PreToolUse`       | **Blocks** the tool call; **`stderr` is shown to Claude**.             |
| `PostToolUse`      | Tool **already ran**; **`stderr` is shown to Claude**.                 |
| `Stop`             | **Blocks** stopping; **`stderr` is shown to Claude**.                  |
| `SubagentStop`     | **Blocks** subagent stopping; **`stderr` is shown to the subagent**.   |
| `Notification`     | N/A to Claude; **`stderr` shown to the user only**.                    |
| `UserPromptSubmit` | **Blocks and erases** the prompt; **`stderr` shown to the user only**. |
| `PreCompact`       | N/A to Claude; **`stderr` shown to the user only**.                    |
| `SessionStart`     | N/A to Claude; **`stderr` shown to the user only**.                    |

### Any other non‑zero exit code

- **Non‑blocking**: `stderr` is shown to the **user only**; Claude proceeds.

---

## 2) JSON‑based feedback (structured control)

If your hook prints a **single JSON object** to `stdout` (usually with exit `0`), Claude will apply these controls.

### Global JSON controls (apply to any hook)

```json
{
  "continue": true, // default true; if false, stop after hooks
  "stopReason": "string", // shown to the user when continue=false (not to Claude)
  "suppressOutput": true // hide stdout from transcript mode display
}
```

- `"continue": false` **halts processing after hooks** and **overrides** any `"decision"` (including `"block"`) for all events.
- `"suppressOutput"` affects **transcript display only**; it does **not** hide content added to **Claude’s context** (e.g., via `additionalContext` or the stdout‑to‑context special cases).

### `PreToolUse`

Control tool permissions and what Claude sees:

- `"permissionDecision": "deny"` → **blocks** the tool call; **`permissionDecisionReason` is shown to Claude**.
- `"permissionDecision": "allow"` → bypasses permission; **reason shown to the user (not Claude)**.
- `"permissionDecision": "ask"` → asks the user; **reason shown to the user (not Claude)**.
- (Deprecated but supported) `"decision": "block" | "approve"` with `reason` mirrors the above.

**Example (`deny`):**

```json
{
  "hookSpecificOutput": {
    "hookEventName": "PreToolUse",
    "permissionDecision": "deny",
    "permissionDecisionReason": "Security policy prevents writing to this directory."
  }
}
```

### `PostToolUse`

- `"decision": "block"` → **automatically prompts Claude** with `reason` (tool already ran).
- `undefined` → no special handling.

**Example:**

```json
{
  "decision": "block",
  "reason": "The file was created, but a unit test failed. Please fix the test."
}
```

> To fully halt after a post‑tool hook, use `"continue": false` (which overrides decisions).

### `Stop` / `SubagentStop`

- `"decision": "block"` → **prevents stopping**; you **must** provide `reason` so Claude knows how to proceed.
- `undefined` → allows stop.

**Example:**

```json
{
  "decision": "block",
  "reason": "Task incomplete. Verify the deployment before stopping."
}
```

### `UserPromptSubmit`

- `"decision": "block"` → **prevents prompt processing**; the submitted prompt is **erased**; `reason` is **shown to the user** (not added to context).
- `"hookSpecificOutput.additionalContext"` → appends the string to context **if not blocked**.
- **Special case:** you can inject context by plain `stdout` with exit `0` (no JSON required).

**Example (add context):**

```json
{
  "hookSpecificOutput": {
    "hookEventName": "UserPromptSubmit",
    "additionalContext": "FYI: The user is currently on the 'main' git branch."
  }
}
```

### `SessionStart`

- `"hookSpecificOutput.additionalContext"` → appends the string to context.
- **Special case:** you can also inject context by plain `stdout` on exit `0`.

**Example:**

```json
{
  "hookSpecificOutput": {
    "hookEventName": "SessionStart",
    "additionalContext": "Loaded repo context and recent changelog."
  }
}
```

---

## Practical comparisons & tips

- **Pre vs. Post tool hooks with errors**

  - `PreToolUse` + exit `2` **blocks** running the tool and sends `stderr` to Claude.
  - `PostToolUse` + exit `2` **cannot retro‑block**; it only surfaces `stderr` to Claude. To change the next step, emit JSON with `"decision": "block"` (or set `"continue": false`).

- **Who sees which “reason”**

  - `PreToolUse` **deny** → reason to **Claude**.
  - `PreToolUse` **allow/ask** → reason to the **user** (not Claude).
  - `PostToolUse` / `Stop` / `SubagentStop` **block** → `reason` is used to guide Claude’s next step.

- **Rule of thumb**
  Use **exit codes** for quick allow/deny and error signaling. Use **JSON** when you need to _explain_, _block with guidance_, or _inject context_. Remember `"continue": false` is the hard stop that overrides everything else.

---

## Quick reference matrix

| Event            | Exit 0 (stdout)                           | Exit 2 (stderr)                        | Other non‑zero (stderr) |
| ---------------- | ----------------------------------------- | -------------------------------------- | ----------------------- |
| PreToolUse       | User transcript (not Claude)              | **Blocks tool**; shown to **Claude**   | User only; continue     |
| PostToolUse      | User transcript (not Claude)              | Shown to **Claude** (tool already ran) | User only; continue     |
| Stop             | User transcript (not Claude)              | **Blocks stop**; shown to **Claude**   | User only; continue     |
| SubagentStop     | User transcript (not Claude)              | **Blocks stop**; shown to **subagent** | User only; continue     |
| Notification     | **Debug-only** (don’t rely on transcript) | N/A to Claude; **shown to user only**  | User only; continue     |
| UserPromptSubmit | **Injected into context (Claude sees)**   | Blocks + erases prompt; **user only**  | User only; continue     |
| PreCompact       | User transcript (not Claude)              | N/A to Claude; **user only**           | User only; continue     |
| SessionStart     | **Injected into context (Claude sees)**   | N/A to Claude; **user only**           | User only; continue     |

---

## Final notes

- Prefer `PreToolUse: "permissionDecision": "deny"` to block a **single** tool call; reserve `"continue": false` for halting the **entire** flow after hooks.
- `suppressOutput` only hides transcript output; it does not hide context you add.

If you want, I can fold those two clarifications directly into your original Markdown with tracked changes or provide a diff patch.
