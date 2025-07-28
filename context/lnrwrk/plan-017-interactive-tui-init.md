# plan 017: Implement Interactive Init Wizard

Created: 2025-07-22T14:00:00Z
Depends: none
Enables: User-friendly onboarding, Interactive policy configuration

## Goal

Replace the current static `cupcake init` command with a full-featured, interactive Terminal User Interface (TUI) wizard. This wizard will guide users through discovering rule sources, extracting rules, reviewing/editing them, and generating the final policy configuration.

## Success Criteria

1.  The `cupcake init` command launches a `ratatui`-based TUI application.
2.  The wizard successfully discovers potential rule sources (e.g., `CLAUDE.md`, `.cursor/rules`, etc.) from the user's repository, as detailed in the design documents.
3.  Users can select/deselect which sources to include in a file-tree view with a live preview pane.
4.  The wizard provides an optional step for users to add custom instructions to guide the LLM extraction process.
5.  The wizard shows real-time progress as it uses an LLM to extract rules from the selected files in parallel.
6.  Users can review, edit, and select/deselect the extracted rules in a hierarchical list with search and filtering capabilities.
7.  The final step shows compilation and sync progress, creating the `guardrails/` directory structure and a success summary.
8.  The wizard gracefully handles user cancellation and potential errors during any phase.
9.  The final implementation adheres to the detailed designs laid out in `context/lnrwrk/plan-017-design-full.md` and the architecture in `plan-017-ratatui-arch.md`.

## Context

The current `cupcake init` command is a basic scaffolding tool that creates a default directory structure and example files (see `src/cli/commands/init.rs`). This is functional but lacks the core value proposition of automatically converting a project's existing natural language conventions into enforceable policies.

This plan implements the primary user-facing feature of Cupcake, as envisioned in `cupcake.md`, transforming `init` from a simple command into a rich, guided onboarding experience. The complete UX flow and technical architecture for this TUI are already designed and documented in the various `plan-017-*` files within `context/lnrwrk/`.

The implementation will heavily leverage the `ratatui` crate for rendering the TUI, `tui-input` for text entry fields, and `tokio` for handling asynchronous operations like file scanning and LLM calls. The existing `init` command logic will be entirely replaced by this new interactive flow.
