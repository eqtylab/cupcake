We will still be able to develop powerful, integrated capabilities around context?
<>
Isolate Context Injection Logic: The special stdout behavior for UserPromptSubmit is an anomaly in the hook contract. We will contain this logic within a specific conditional block in run.rs to keep the rest of the engine focused on the primary JSON-based communication model.
</>

--

Yes, absolutely. That is an excellent and critical question. My use of the term "isolate" was meant in a precise architectural sense, not to imply that the feature would be limited or disconnected.

Let me clarify: **Isolating the _output mechanism_ is precisely what allows us to build powerful, deeply _integrated_ capabilities for context generation.**

Here’s the breakdown:

### The Anomaly: Communication Protocol vs. Internal Logic

The "anomaly" is purely at the communication layer—the final step of how Cupcake talks back to Claude Code.

- **The Norm (99% of cases):** Cupcake communicates by printing a structured **JSON** payload to `stdout`.
- **The Anomaly (UserPromptSubmit):** Cupcake communicates by printing a raw **string** to `stdout` (the context) and _maybe_ a JSON payload later.

My plan is to isolate the handling of this unique communication channel, not the logic that decides _what_ to communicate.

Think of it like a mailroom. Most packages go into a standard box (JSON). One special type of document must go into a unique red envelope (raw string `stdout`). The mailroom clerk (our `run.rs` handler) is responsible for knowing which container to use. But the department that _generates_ the complex, data-rich report inside (our core policy engine) doesn't care about the final packaging; it just produces the result.

### How This Design Enables Powerful, Integrated Capabilities

By isolating the final output step, we ensure that the logic for generating context is a first-class citizen within the main policy engine, able to leverage its full power.

Here’s how we will build powerful capabilities:

1.  **Dynamic Context Generation:** The `InjectContext { context: String }` action is not just a static string. The `context` field will support the same powerful template variables as any other action.

    ```yaml
    # A policy can generate context using live data
    action:
      type: "inject_context"
      context: "Reminder: The current build is FAILING. Last failed test was '{{state.last_failed_test}}'."
    ```

2.  **Stateful Awareness:** The decision to inject context will be driven by the same rich condition engine as every other policy. This is where the deep integration happens. We can use the `StateManager` to make intelligent decisions.

    ```yaml
    # This policy ONLY injects context if a specific violation occurred recently
    UserPromptSubmit:
      "":
        - name: "Proactive reminder after previous failure"
          conditions:
            - type: "check" # This will be a new 'state_query' type
              query: "event_exists"
              params:
                name: "DangerousCommandBlocked"
                within_minutes: 10
          action:
            type: "inject_context"
            context: "AI Safety Reminder: A dangerous command was blocked recently. Please be extra cautious with shell commands and file system operations."
    ```

3.  **Multi-Source Aggregation:** The `RunCommand` handler will be responsible for collecting the context strings from _all_ matching `InjectContext` actions and concatenating them. This means you can have multiple, smaller, independent policies that each contribute a piece of relevant context, which are then assembled into a single, comprehensive briefing for the agent.

### A Concrete Example Flow

Here is how the "isolated" logic enables an "integrated" capability:

1.  A `UserPromptSubmit` event arrives. The user's prompt is "delete the temp files."
2.  The `PolicyEvaluator` runs as normal.
3.  It evaluates a policy like the "Proactive reminder" example above.
4.  The condition evaluator queries the `StateManager` and finds that a `DangerousCommandBlocked` event _did_ occur 5 minutes ago. The condition matches.
5.  The policy's `InjectContext` action is marked for execution.
6.  The `ActionExecutor` processes this, substituting any templates, and returns an internal `EngineDecision` that contains the context string to be injected.
7.  The main `RunCommand::execute` function receives this decision.
8.  **Here is the isolated logic:** It checks `if event_type == UserPromptSubmit`. Because it is, it takes the context string from the `EngineDecision` and prints it to `stdout`.
9.  It then proceeds to construct the final JSON response (which will likely be empty, signaling "allow") and prints that.

The context _generation_ was deeply integrated with the state manager and condition engine. The context _delivery_ was handled by a small, specialized block of code that knows about this one specific communication anomaly.

By architecting it this way, we gain:

- **Engine Purity:** The core policy engine thinks in terms of abstract decisions, not `stdout` vs. JSON.
- **Maximum Power:** Context generation has access to the full state and condition evaluation system.
- **Flexibility:** If Claude Code ever changes this hook to accept context via a JSON field, we only need to change the small, isolated logic block in `run.rs` without touching the powerful generation logic at all.
