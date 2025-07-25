# `plan-019-README.md`

This directory contains the analysis, planning, and reference materials for integrating the Claude Code July 20 hooks update into Cupcake.

## Recommended Reading Order

To get up to speed quickly, review the documents in this order:

1.  **`plan-019-new-claude-code-updates.md`**: Start here for a high-level summary and confidence check.
2.  **`plan-019-reference-cupcake-implications.md`**: Understand the strategic value of these changes and _why_ they are important for Cupcake's mission.
3.  **`plan-019-plan.md`** & **`plan-019-plan-ammendment-phase5.md`**: Read these together. This is the **final, five-phase implementation plan** and the primary guide for the work.
4.  **`plan-019-reference-claude-code-july20-complete.md`**: The definitive technical reference for the Claude Code hook contract. Keep this open during implementation.
5.  **`plan-019-plan-question-about-context.md`**: Read this for an important clarification on the context injection architecture.
6.  **`plan-019-discovery-report.md`** & **`plan-019-plan-careful-considerations.md`**: Review these for the full background on the "why" behind the plan and potential risks.

## Document Directory Map

- **`plan-019-plan.md`**: The core four-phase implementation plan.
- **`plan-019-plan-ammendment-phase5.md`**: **CRITICAL AMENDMENT.** Expands the plan to five phases, arguing for the inclusion of `state_query`.
- **`plan-019-discovery-report.md`**: The output of the initial analysis, detailing the current state, gaps, and technical recommendations.
- **`plan-019-new-claude-code-updates.md`**: A high-level assessment of the plan and Cupcake's readiness for the updates.
- **`plan-019-plan-careful-considerations.md`**: Highlights risks and implementation complexities (e.g., the `sync` command, testing strategy).
- **`plan-019-plan-question-about-context.md`**: A Q&A clarifying the architectural decision to isolate the context injection output mechanism.
- **`plan-019-reference-claude-code-july20-complete.md`**: The complete, nuanced technical reference for the new hook contract. **This is the source of truth.**
- **`plan-019-reference-context-injection.md`**: A technical deep-dive on the `UserPromptSubmit` context injection feature.
- **`plan-019-reference-cupcake-implications.md`**: A strategic document explaining how the updates amplify Cupcake's value proposition.

---

### Superseded / Historical Documents

These documents are not required reading for implementation but are kept for historical context.

- **`plan-019-discovery-todos.md`**: The initial checklist used to generate the discovery report. The report itself is the final output.
- **`plan-019-reference-claude-code-july20-changes.md`**: An earlier, less complete version of the technical reference. Use `...-july20-complete.md` instead.

---

Here are five guiding principles to keep the `plan-019` implementation in check:

### 1. The Hook Contract is King

**What it means:** Cupcake's primary job is to be a compliant and powerful client for the Claude Code hook system. Every decision must serve the goal of producing the correct, well-formed JSON output or exit code that the hook contract specifies. We are not inventing a new protocol; we are mastering an existing one.

**In Practice:**

- When in doubt, refer to `context/claude-code-docs/july20-updates/hooks.md`. It is the source of truth.
- The `CupcakeResponse` struct in `src/engine/response.rs` must be a perfect, serializable representation of the JSON output schema.
- Avoid adding internal features or decision types that cannot be expressed through the official hook contract. If Claude Code can't understand the result, it's a wasted effort.

### 2. Secure by Default, Powerful by Choice

**What it means:** The security of the user's system is non-negotiable. The default mode of operation must be completely secure against command injection. More powerful, potentially less secure features (like `shell` mode) must be an explicit, audited, and deliberate choice by the user.

**In Practice:**

- The `array` mode in `CommandSpec` is the gold standard. All new features and internal logic should prioritize it.
- The `string` parser must remain a safe, limited translation to the `array` model. Resist the temptation to add more shell features to it.
- Never compromise the security guarantee that templates (`{{...}}`) cannot be substituted into command paths. This is a bright red line.
- The `allow_shell: true` setting should always be treated as a security-sensitive decision that requires user opt-in.

### 3. The Policy is the API

**What it means:** The YAML policy format is the primary interface for our users. It should be expressive enough to solve common problems but simple enough to be readable and auditable. We should empower policy authors, not force them to become programmers in YAML.

**In Practice:**

- If a condition requires complex branching, loops, or multi-line logic, it belongs in a script that can be called with a `check` condition. Do not try to build a full programming language in the YAML schema.
- The one necessary expansion is the `state_query` condition, as it unlocks the core value of the state manager. Beyond that, new condition types should be viewed with extreme skepticism.
- Every field in the policy schema should have a clear, unambiguous purpose that directly maps to a feature in the evaluation engine.

### 4. State is for _What_, Not _How_

**What it means:** The `StateManager`'s job is to be a simple, reliable log of _what happened_ (facts). The policy engine's job is to contain the logic for _how to react_ to those facts. This separation of concerns is critical to preventing the state manager from becoming overly complex and business-logic-aware.

**In Practice:**

- The `StateManager` should store simple, factual events: `ToolUsage`, `CustomEvent`, etc.
- It should provide simple query methods like `has_read_file()` or `count_tool_usage()`.
- It should **not** contain complex logic like `shouldBlockCommit()`. That logic belongs in a policy that _uses_ the state manager's simple query results. This keeps the state layer clean and the logic layer transparent.

### 5. The User Workflow Must Be Seamless

**What it means:** Cupcake should feel like a magical, "it just works" extension of the developer's environment. The complexity of the engine should be completely hidden from the user during setup and normal operation.

**In Practice:**

- The `sync` command is not an edge case; it is a critical part of the core user experience. It **must** be robust, safely merging its configuration without destroying the user's existing `settings.local.json`.
- The `init` TUI should produce clear, modern, and useful example policies that immediately showcase the power of the new features (especially `inject_context`).
- Error messages, whether from validation (`validate`) or runtime (`run`), must be clear, actionable, and help the user fix their policies.
