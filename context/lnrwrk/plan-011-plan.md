# Plan for Plan 011: Refine Timeout Configuration

Created: 2025-01-16T12:00:00Z

## Approach

Remove unnecessary CLI timeout argument and fix command execution timeout hierarchy to properly use configurable settings.

## Steps

### 1. Remove CLI Timeout Argument
- Remove `--timeout` argument from `RunCommand` in `cli/app.rs`
- Remove `timeout` field from `RunCommand` struct in `cli/commands/run.rs`
- Update `RunCommand::new()` to not take timeout parameter
- Update all tests that reference the CLI timeout

### 2. Fix Command Execution Timeout
- In `engine/actions.rs:171`, change `timeout_seconds.unwrap_or(30)` to use settings
- Pass `context.settings.timeout_ms` to the execute_run_command method
- Convert milliseconds to seconds for backward compatibility

### 3. Update Tests
- Remove CLI timeout tests
- Verify command execution timeout tests still pass
- Ensure settings.timeout_ms is properly used

## Technical Decisions

- The CLI timeout was architectural cruft that serves no purpose
- Claude Code already enforces hook timeouts externally
- Only command execution timeouts matter for resource control
- Maintain backward compatibility with existing timeout_seconds in actions

## Why This Approach

After thorough analysis of Claude Code hooks documentation:
- Claude Code enforces its own timeout by killing the process
- There's no need for Cupcake to self-limit with a CLI timeout
- The only timeouts that matter are for commands Cupcake executes internally
- This simplifies the codebase and removes confusion