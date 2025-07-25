### The Strategic Flaw in Deferring `state_query`

Deferring the implementation of a native `state_query` condition would create a "hollow victory." We would successfully implement the _technical contract_ of the July 20 updates but fail to deliver the _transformative value_ that those updates are meant to enable.

1.  **It Neuters Context Injection:** The single most powerful new feature is `inject_context` via the `UserPromptSubmit` hook. Without a `state_query` condition, policy authors can only trigger this injection based on static analysis of the user's prompt (e.g., using a `pattern` match). This is useful, but it's a tiny fraction of the feature's potential. The true power—the "intelligent guidance"—comes from injecting context based on what has _already happened_ in the session. Deferring `state_query` is like delivering a new car with a powerful engine but locking the transmission in first gear.

2.  **It Creates a Poor First Impression:** The first experience users have with Cupcake's support for the new hooks will define their mental model of its capabilities. If the only examples we can provide are stateless, they will perceive the tool as a simple, reactive linter. We will miss the opportunity to immediately establish Cupcake as a sophisticated, state-aware guidance system.

3.  **It Creates a Documentation-Implementation Mismatch:** As identified, our documentation will need to be updated with compelling examples to explain the new features. The most compelling examples (like the "read-then-write" workflow or the "escalating enforcement" policy) are only possible with `state_query`. Shipping without it would mean our documentation describes a powerful future state, while the tool itself remains limited, leading to user confusion and frustration.

### The Pragmatic Path: Incorporate it as the Final Phase of Plan 019

While the feature is critical, I agree with the implicit concern about scope. The solution is not to defer it, but to integrate it intelligently into the existing plan as a distinct, final phase. This acknowledges its importance while maintaining a structured, manageable workflow.

Here is the refined, five-phase plan for Plan 019:

**Phase 1: The New Contract** (As defined previously)
_Goal: Speak the new JSON protocol._

- Update `EngineDecision` and `CupcakeResponse` structs.
- Refactor the `run` command to return a `CupcakeResponse`.

**Phase 2: Proactive Guidance** (As defined previously)
_Goal: Implement the core context injection feature._

- Add the `InjectContext` action.
- Implement the special `stdout` handling for `UserPromptSubmit`.

**Phase 3: The User Workflow** (As defined previously)
_Goal: Make the system usable end-to-end._

- Implement the `sync` command.
- Update the TUI's configuration generation.

---

**Phase 4 (NEW): Intelligent Conditions - Activating Stateful Guidance**
_Goal: Connect the state manager to the policy engine._

1.  **Define the `StateQuery` Condition (`src/config/conditions.rs`):**

    - Add a new `StateQuery` variant to the `Condition` enum. This variant will contain fields for the `query` type (e.g., "has_read_file", "count_tool_usage") and its `params`.

2.  **Integrate `StateManager` into the `ConditionEvaluator` (`src/engine/conditions.rs`):**

    - The `ConditionEvaluator::evaluate` method will need access to the `StateManager`. This can be done by passing the manager into the evaluation context.
    - Implement the evaluation logic for the new `StateQuery` condition. This logic will call methods on the `StateManager`'s `StateQuery` engine (`src/state/query.rs`).

3.  **Expand the State Query Engine (`src/state/query.rs`):**
    - Ensure the `StateQuery` struct has all the necessary methods to support the queries needed for our envisioned policies (e.g., `count_events_within_timespan`, `get_last_event_of_type`, etc.).

---

**Phase 5: Power-Ups & Documentation** (Formerly Phase 4)
_Goal: Add final features and document everything._

- Integrate `$CLAUDE_PROJECT_DIR`.
- Add documentation for MCP tool matching.
- **Crucially, write the documentation and examples for the new `state_query` condition.**

### Conclusion

Adding `state_query` is not scope creep; it is the **necessary step to complete the core feature** introduced by the Claude Code updates. By sequencing it as a distinct phase _within_ Plan 019, we manage risk while ensuring that the final deliverable is a cohesive, powerful, and truly intelligent upgrade that lives up to the project's vision. Deferring it would leave the most exciting part of the new functionality on the table, crippling the user experience and undermining the very purpose of this update cycle.
