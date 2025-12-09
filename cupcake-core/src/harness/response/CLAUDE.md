## Cursor vs Claude Code Stop/Continue designs

While Cursor allows you to submit a new "follow-up message" after the agent stops, Claude Code allows you to **"block" the stop event** and provide a `reason`. This `reason` field functions exactly like a follow-up message—it is fed back to the agent to tell it why it isn't allowed to finish and what it needs to do next.

### How it works in Claude Code

In Claude Code, you use the `Stop` hook. If your hook determines that work is incomplete, it returns a JSON object with `"decision": "block"`.

#### The Mechanism

1.  **Cursor** ends the turn, then the hook submits a **new user message** (`followup_message`).
2.  **Claude Code** intercepts the attempt to finish, cancels the stop, and gives the agent the **reason** as immediate feedback to continue the current workflow.

### Configuration Example

You can implement this using a standard script or Claude Code's native "Prompt-based" hooks (which are specifically designed for this use case).

#### Option 1: Using a Prompt-Based Hook (Recommended)

This is the most powerful method. Instead of writing a script to parse the conversation, you ask an LLM to decide if the agent should continue.

```json
{
  "hooks": {
    "Stop": [
      {
        "hooks": [
          {
            "type": "prompt",
            "prompt": "Evaluate if Claude has actually completed the user's request: $ARGUMENTS. If there are pending TODOs or broken tests, block the stop.",
            "timeout": 30
          }
        ]
      }
    ]
  }
}
```

If the LLM decides to block, it generates the `reason` automatically, which Claude Code uses to force the agent to continue working.

#### Option 2: Using a Script (Manual Control)

If you write a script (e.g., to check if a specific file exists or a test passes), you output JSON to standard out:

```python
# Example logic for a Stop hook script
print(json.dumps({
  "decision": "block",
  "reason": "You haven't updated the README yet. Please update it before stopping."
}))
```

### Comparison Table

| Feature             | Cursor Agent                                                                            | Claude Code                                                                                                                                                   |
| :------------------ | :-------------------------------------------------------------------------------------- | :------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| **Hook Event**      | `stop`                                                                                  | `Stop`                                                                                                                                                        |
| **Action**          | Submit `followup_message`                                                               | Return `decision: "block"`                                                                                                                                    |
| **Content**         | Acts as a new User message                                                              | Acts as a System/Context instruction                                                                                                                          |
| **Loop Prevention** | **`loop_count`**: Input field provided to hook (max 5 auto-follows enforced by Cursor). | **`stop_hook_active`**: Boolean input field. True if Claude is already continuing due to a hook. You must implement your own logic to prevent infinite loops. |
| **Intelligence**    | You must write the logic in a script.                                                   | Can use `type: "prompt"` to have a fast LLM decide if work should continue.                                                                                   |

### Summary regarding Loop Prevention

Cursor handles loop prevention automatically (max 5). In Claude Code, the `Stop` hook receives a boolean field `stop_hook_active` in the input JSON.

If you are writing a custom script, you should check this variable. If it is `true`, it means the agent has _already_ been blocked from stopping once. You should likely allow it to stop the second time to prevent an infinite loop where the agent and the hook fight forever.

---

## Cupcake's Unified Approach

Cupcake provides a unified policy interface that abstracts away these harness differences.
Policy authors use the same `deny` verb, and Cupcake handles the harness-specific translation:

### Policy (works for both harnesses)

```rego
deny contains decision if {
    input.hook_event_name in ["Stop", "stop"]  # Case varies by harness
    # Check loop prevention (harness-specific field names)
    not should_allow_stop(input)
    decision := {
        "rule_id": "CONTINUE-WORK",
        "reason": "Tests are still failing. Please fix them.",
        "severity": "MEDIUM"
    }
}

# Helper to check loop prevention
should_allow_stop(input) if input.stop_hook_active == true      # Claude Code
should_allow_stop(input) if input.loop_count >= 5               # Cursor
```

### Cupcake Translation

| Policy Decision | Claude Code Response | Cursor Response |
|-----------------|---------------------|-----------------|
| `deny` (block) | `{"decision": "block", "reason": "..."}` | `{"followup_message": "..."}` |
| allow (default) | `{}` | `{}` |

This is similar to how Terraform provides a unified HCL interface while translating to
AWS, GCP, or Azure-specific APIs underneath.
