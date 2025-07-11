# plan 001: Implement Core Domain and Type-Safe Foundation

Created: 2025-07-10T10:00:00Z
Depends: none
Enables: plan-002, plan-003, plan-004

## Goal

Establish the foundational Rust data structures and project scaffolding for the Cupcake MVP. This phase focuses on creating a stable, type-safe "language" for the application by translating all design schemas into concrete Rust code, ensuring all subsequent development is built on a robust and correct foundation.

## Success Criteria

- A compilable Rust project exists with a clear, logical module structure (`cli`, `engine`, `config`, `state`, `io`).
- All policy schema definitions from `policy-schema.md` are fully implemented as Rust structs and enums using `serde`, with no deferred types for the V1 scope.
- All Claude Code hook event payloads from `hooks.md` are represented as type-safe Rust structs for deserialization.
- The complete public CLI interface (`init`, `run`, `sync`, `validate`, `audit`) is defined using `clap`, with each subcommand existing as an empty, runnable shell.
- The project's core dependencies (`serde`, `clap`, `toml`, `anyhow`) are integrated and the project builds successfully without any business logic.
