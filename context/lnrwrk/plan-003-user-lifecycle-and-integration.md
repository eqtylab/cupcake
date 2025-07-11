# plan 003: Implement User Lifecycle and Integration

Created: 2024-05-24T10:02:00Z
Depends: plan-002
Enables: plan-004

## Goal

Develop the user-facing "wizard" commands (`init`, `sync`, `validate`) that make Cupcake easy to adopt and integrate into a developer's workflow. This phase builds the seamless onboarding experience on top of the powerful engine from the previous phase.

## Success Criteria

- The `cupcake validate` command can successfully parse a `cupcake.toml` file and report clear, actionable errors.
- The `cupcake init` command correctly implements the full memory discovery algorithm from `memory-discovery.md`, including upward traversal, subtree discovery, and `@import` resolution.
- `cupcake init` can spawn a `claude` process, pipe the meta-prompt, and handle the interactive session.
- The `init` command's self-correction loop, which uses `validate` to ensure the AI produces a valid policy file, is fully functional.
- The `cupcake sync` command can safely read, modify, and write to `.claude/settings.json`, injecting the `cupcake run` hooks without destroying existing user settings.
- The full `Project > User` policy hierarchy is now correctly respected by the runtime engine's policy loader.
