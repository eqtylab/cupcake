# Cupcake: Deterministic Guardrails for Your AI Coding Agent

Cupcake is a high-speed policy enforcement engine that integrates directly with Claude Code to act as its deterministic conscience. It ensures that every action the AI agent takes—from editing a file to running a command—adheres to your project's specific rules, conventions, and safety requirements. It bridges the critical gap between providing helpful suggestions to an AI and enforcing unbreakable guarantees.

The core problem Cupcake solves is that while `CLAUDE.md` is excellent for giving an AI context and memory, it provides suggestions, not mandates. An AI, like any developer, can misinterpret, forget, or fail to apply these guidelines, leading to inconsistent or incorrect results. For professional development, and especially in enterprise environments, teams need certainty. You need to know that tests will always be run before a commit, that a critical file will never be edited without the proper context, and that your team's coding standards will be followed every single time. Cupcake provides this certainty.

The process is designed to be seamless. You continue to write your project's rules and conventions in natural language within your `CLAUDE.md` files. Then, you run a single command: `cupcake init`. Cupcake uses Claude Code's own intelligence to read your rules and automatically translate them into a set of machine-readable policies. From that moment on, Cupcake works silently in the background. It intercepts every action Claude Code attempts, evaluates it against your policies in milliseconds, and ensures compliance.

What makes Cupcake so valuable is its intelligent and helpful approach to enforcement. It understands that not all rules are equal. If Claude Code violates multiple stylistic guidelines, Cupcake doesn't play a frustrating game of whack-a-mole; it aggregates all the feedback and presents it in a single, comprehensive list so the agent can fix everything at once. If a critical security or workflow rule is violated, Cupcake immediately blocks the action and provides clear, actionable feedback on the show-stopping issue, while still including the other stylistic notes for context. This "two-pass" system ensures that the AI is both efficient and safe, correcting all minor issues in one go while being stopped by what truly matters.

Furthermore, Cupcake is designed with stateful awareness, allowing it to enforce complex, multi-step workflows. It can remember if a specific file was read or if a test suite was successfully run earlier in the session, enabling policies like "you must read the architecture document before you can edit this core engine file." For anything beyond its built-in checks, Cupcake can execute your project's existing scripts—linters, test runners, custom validators—and use their results to inform its decisions. This makes it an incredibly powerful and extensible tool that integrates with your workflow, rather than forcing you into a new one.

In essence, Cupcake provides the trust layer for agentic software development. It empowers developers and organizations to confidently deploy AI coding agents, knowing that a fast, reliable, and intelligent set of guardrails is always active, ensuring every line of code produced is not just functional, but also compliant, safe, and correct.

## Map of the `design_phase` Directory for Cupcake

This directory contains the complete and final architectural blueprint for the Cupcake policy enforcement engine. It is organized into logical sections covering the core design, its relationship to Claude Code, and deprecated initial concepts.

**`context/design_phase`**

- **`context/design_phase/architecture.md`**: The master blueprint. This is the single most important document, defining the high-level vision, core principles, system components (CLI, state, cache, audit), data flows (`init` and `run`), the two-pass aggregation model, and the technology stack. It serves as the central reference for the entire project.

- **`context/design_phase/policy-schema.md`**: The definitive technical specification for the `cupcake.toml` file. It details every valid field, condition type (`command_regex`, `state_exists`), and action type (`provide_feedback`, `block_with_feedback`, `run_command`). It distinguishes between "soft" and "hard" actions, which is fundamental to the two-pass evaluation model, and provides numerous examples. While it describes a powerful "North Star" schema, the MVP will implement a focused subset of these features.

- **`context/design_phase/feedback-aggregation.md`**: A critical clarification document that resolves a key design ambiguity. It explicitly states the rule for combining feedback: when a "hard block" and "soft feedback" are both triggered, Cupcake presents _all_ feedback to the agent, prioritizing the hard block's message while including the soft feedback for maximum context and efficiency.

- **`context/design_phase/command-execution-patterns.md`** & **`context/design_phase/command-interception-patterns.md`**: These two documents work together to showcase the power and flexibility of the `run_command` action. They provide practical, real-world examples of how developers can use Cupcake to enforce policies by running existing linters, test suites, and custom validation scripts. They demonstrate how to handle file matching, command templating, and exit code logic, and prove that complex logical checks (like `and`/`or`) can be handled by external scripts, simplifying the core policy schema for the MVP.

- **`context/design_phase/memory-discovery.md`**: A detailed technical specification for the `cupcake init` command's discovery process. It outlines the precise algorithm for finding all relevant `CLAUDE.md` files, perfectly mirroring Claude Code's own documented behavior by including upward recursive search, subtree discovery, and `@import` resolution.

- **`context/design_phase/hook-events.md`**: This document maps Cupcake's functionality to the entire Claude Code lifecycle. It details how Cupcake will handle each specific hook event (`PreToolUse`, `PostToolUse`, `Stop`, etc.), outlining the common use cases and decision logic appropriate for each timing.

- **`context/design_phase/meta-prompt.md`**: Defines the "soul" of the `cupcake init` command. It contains the carefully crafted system prompt given to Claude Code to translate natural language rules from `CLAUDE.md` into the structured `cupcake.toml` format. It also includes the logic for the self-correction loop, where validation errors are fed back to the AI.

- **`context/design_phase/claude-code-docs/`**: A local copy of the essential Claude Code documentation. This ensures that the Cupcake design remains perfectly aligned with the platform it integrates with, particularly `hooks.md` (for the integration mechanism) and `memory.md` (for the configuration hierarchy).

<do not read>
- **`context/design_phase/phase0_deprecated/`**: A historical archive. This directory contains the initial brainstorming documents, Q&A sessions, and early ideas. As per the project's `CLAUDE.md`, these files are considered outdated and should not be used for implementation, but they provide valuable context on how the final design evolved.
</do not read>
