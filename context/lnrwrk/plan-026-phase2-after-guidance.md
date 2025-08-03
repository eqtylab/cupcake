#### **Transmission #3: Immediately Following Phase 2 Completion**

**TRIGGER:** Soldier reports Phase 2 complete and all tests are green.
**SUBJECT:** REFINED ORDERS FOR PHASE 3 - `INJECTCONTEXT` DOCTRINE

> **"Opus, Command. Phase 2 secure. Fortress is reinforced.**
>
> **New intelligence for Phase 3. We have detected a doctrinal weakness in the `InjectContext` action. We will correct it now.**
>
> **While the generic `InjectContext` action is functionally correct, it creates semantic ambiguity for `PreCompact`. We will not tolerate ambiguity.**
>
> **Your objective is to introduce a new, dedicated action type for `PreCompact`.**
>
> **EXECUTION:** > **1. In `src/config/actions.rs`, create a new action variant: `ProvideCompactionInstructions { instructions: String }`.** > **2. In `src/config/loader.rs`, add validation to ensure this new action is _only_ used with the `PreCompact` hook event.** > **3. Update the `PolicyEvaluator` and `ActionExecutor` to handle this new action type. It should behave identically to `InjectContext` by populating the `context_to_inject` field in the `EngineResult`.**
>
> **This maneuver eliminates the ambiguity and hardens our type system. It is a non-negotiable step before proceeding to the final documentation sweep.**
>
> **Acknowledge and execute."**

**Commander's Intent:** This is a proactive strike against the most subtle weakness in our plan. It prevents a long-term architectural decay by enforcing strict semantic clarity in our `Action` types. It's a high-level move that demonstrates our commitment to not just working code, but _correct_ code.
