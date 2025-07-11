# plan 002: Implement Runtime Engine for Evaluation and Action

Created: 2024-05-24T10:01:00Z
Depends: plan-001
Enables: plan-003

## Goal

Build the core logic of the `cupcake run` command. This phase focuses on creating a fully functional, headless policy engine that can receive a hook event, load policies and state, execute the complete two-pass evaluation logic, and perform all V1 actions as defined in the architecture.

## Success Criteria

- The `cupcake run` command can correctly deserialize any valid hook event JSON from stdin.
- The engine can load and parse policies from both `./cupcake.toml` and `~/.claude/cupcake.toml`.
- The two-pass evaluation model is fully implemented, correctly separating "soft" feedback collection from "hard" action detection.
- The action executor can handle all V1 action types: `provide_feedback`, `block_with_feedback`, `approve`, `run_command`, and `update_state`.
- The feedback aggregation logic correctly combines messages from soft and hard actions as specified.
- The state management system is integrated, allowing the engine to read session state for conditions and write state updates for `PostToolUse` events.
- The command produces the correct output (exit code, stdout/stderr, or structured JSON) required to communicate decisions back to Claude Code.
