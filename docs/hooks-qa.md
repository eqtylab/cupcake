- **What does "injecting context into Claude's awareness" technically mean?**

  - **Answer:** According to the `hooks.md` documentation, for `UserPromptSubmit` and `SessionStart` hooks, anything the hook command prints to `stdout` with an exit code of `0` is captured and added to the conversational context _before_ the agent processes the user's prompt. It's as if an assistant whispered extra instructions to the agent. For all other hooks (`PreToolUse`, `PostToolUse`, etc.), `stdout` is only displayed to the user in the transcript view and is **not** seen by the agent.

- **How does `suppressOutput` work in Claude Code Hooks?**

  - **Answer:** The `hooks.md` documentation specifies a `suppressOutput: true` boolean field in the JSON output from a hook. When set, it prevents the hook's `stdout` from appearing in the user's transcript view (Ctrl-R). This is crucial for "silent" actions.

- **How does silent auto-approval work?**

  - **Answer:** This is a combination of two features from `hooks.md`. A `PreToolUse` hook can return a JSON object with `"permissionDecision": "allow"`. If that same JSON object also includes `"suppressOutput": true`, the tool use is approved and the approval reason does not appear in the transcript.

- **What is meant by per-policy output control?**

  - **Answer:** Claude Code's hook system can execute multiple hook commands for a single event. Each command can produce its own JSON output. Cupcake, however, currently runs a single command (`cupcake run`) and produces a single, aggregated JSON response. True per-policy control would mean Cupcake could generate a complex response that represents multiple, independent decisions, but this is an advanced feature for the future. Our current two-pass evaluation model is a robust and sufficient aggregation strategy.

- **What does it mean for policies to aggregate vs. behave independently?**
  - **Answer:**
    - **Aggregation (Current Cupcake Model):** All matching policies are evaluated. All "soft" feedback is collected. Then, the first "hard" action (`block`, `allow`, `ask`) found among the matching policies is executed. This is the "two-pass evaluation" described in `docs/conditions-and-actions.md`. It ensures the user gets all relevant warnings even if an early policy blocks the action.
    - **Independent (Future Possibility):** The first matching policy that produces a "hard" action would immediately halt all further policy evaluation and return its decision. This is simpler but less comprehensive. Cupcake's current aggregation model is superior for providing complete context.
