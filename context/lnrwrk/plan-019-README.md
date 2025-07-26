# `plan-019-README.md`

This directory contains the analysis, planning, and reference materials for integrating the Claude Code July 20 hooks update into Cupcake.

**backwards compatibility is NOT required - no migrations necessary - full update granted, remove old/unused code**

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
