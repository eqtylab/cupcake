# plan 005: Evolve to Scalable YAML Policy Format

Created: 2025-11-07T14:00:00Z
Depends: none
Enables: Future policy development at scale

## Goal

Refactor the Cupcake policy format from the flat `cupcake.toml` structure to a scalable, composable, "Grouped Map" `cupcake.yaml` format. This includes adopting a modular filesystem convention under a new `guardrails/` directory.

## Success Criteria

1.  **New Format Support:** The Cupcake engine can parse and correctly interpret the "Grouped Map" `cupcake.yaml` format.
2.  **Modularity:** The engine supports a root `cupcake.yaml` that uses an `imports` key with glob patterns to load and compose multiple policy fragment files from a `policies/` subdirectory.
3.  **Composition Logic:** The engine correctly performs a deep merge of imported policy fragments, concatenating policy lists under the same hook/matcher key.
4.  **Validation:** The engine enforces that all policy `name` fields are unique across the entire composed set, raising an error on duplicates.
5.  **Code Refactoring:** The existing loading logic in `src/config/loader.rs` is refactored to handle the new format, and `serde_yaml` replaces `toml` as the parsing dependency.
6.  **Documentation & Examples:** All examples in the project (README, tests, etc.) are updated to reflect the new `guardrails/cupcake.yaml` convention.

## Context

The initial `toml` format was a successful MVP but has proven to be verbose and difficult to scale. As the number of policies grows, a single monolithic file leads to merge conflicts and poor discoverability. This plan addresses these limitations by adopting best practices from modern Infrastructure-as-Code tooling, establishing a professional and scalable foundation for all future policy development. The new format will improve developer ergonomics, enable clear ownership of policy domains, and make the entire system more maintainable.

## Github Issue

The Github issue for this is @/Users/ramos/cupcake/cupcake-rs/context/issues/need-scalable-policy.md
