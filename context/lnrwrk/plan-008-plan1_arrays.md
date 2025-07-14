# Plan 008, Part 1: Implement Secure `array:` Executor

Created: 2025-07-15T10:00:00Z
Depends: plan-007
Enables: plan-008-plan2
Priority: CRITICAL

## Goal

Replace the current, insecure shell-based command execution with a new, secure executor that handles the Kubernetes-style `array:` syntax. This plan builds the foundational `CommandGraph` engine, eliminating the shell injection vulnerability and fixing the zombie process leak.

## Success Criteria

1.  ✅ The `Action::RunCommand` and `Condition::Check` variants are updated to accept a new `CommandSpec` enum, starting with the `array:` format.
2.  ✅ A new `CommandExecutor` can parse the `array:` spec (including composition keys like `pipe` and `redirectStdout`) into an internal `CommandGraph`.
3.  ✅ The executor runs the `CommandGraph` using direct process spawning (`tokio::process::Command`), never `sh -c`.
4.  ✅ The executor correctly handles `stdout`/`stdin` for pipes and file redirects.
5.  ✅ The executor waits for child processes to complete, capturing exit codes and preventing zombie processes.
6.  ✅ `onSuccess` (`&&`) and `onFailure` (`||`) logic is correctly implemented based on exit codes.
7.  ✅ All template substitutions (`{{...}}`) are safely applied only to arguments and environment variables, not the command itself.
8.  ✅ Integration tests verify that shell metacharacters in arguments are passed literally and do not result in command injection.

## Context

The current implementation in `src/engine/actions.rs` and `src/engine/conditions.rs` uses `sh -c` to execute commands from a single string. This is vulnerable to shell injection and leaks zombie processes. This plan implements the foundational layer of the new, secure design by building the `array:` executor first.

## Technical Scope

1.  **Update Configuration Structs (`src/config/`)**

    - In `src/config/actions.rs`:
      - Create a new `pub enum CommandSpec` with an `Array(ArrayCommandSpec)` variant.
      - Create `pub struct ArrayCommandSpec` containing `command: Vec<String>`, `args: Option<Vec<String>>`, `workingDir: Option<String>`, `env: Option<Vec<EnvVar>>`, and all seven composition keys (`pipe`, `redirectStdout`, etc.).
      - Create `pub struct EnvVar { name: String, value: String }`.
      - Modify `Action::RunCommand` to contain `spec: CommandSpec` instead of `command: String`.
    - In `src/config/conditions.rs`:
      - Modify `Condition::Check` to also contain `spec: CommandSpec`.

2.  **Create New Command Executor (`src/engine/command_executor/`)**

    - Create a new module: `src/engine/command_executor/mod.rs`.
    - Define an internal `CommandGraph` enum/struct to represent a sequence of processes and operators.
    - Implement a `build_graph_from_array_spec` function that consumes an `ArrayCommandSpec` and produces a `CommandGraph`.
    - Implement the main `execute_graph` function. This function will:
      - Iterate through the `CommandGraph`.
      - Use `tokio::process::Command` to configure each process.
      - Set `.program()` and `.args()` directly (no shell).
      - Use `.stdin(Stdio::piped())`, `.stdout(Stdio::piped())` to manage I/O for pipes.
      - Use `Stdio::from(File::create(...))` for redirects.
      - Spawn child processes, `await` their completion, and check exit codes to handle `onSuccess`/`onFailure`.

3.  **Refactor Engine Logic (`src/engine/`)**

    - In `src/engine/actions.rs`:
      - Remove the old `execute_command_sync` and `execute_command_background` functions.
      - Rewrite `execute_run_command` to call the new `CommandExecutor` when it finds a `CommandSpec::Array`.
      - Ensure `ActionContext::substitute_template` is now only called on `args` and `env` values, _never_ on the command path.
    - In `src/engine/conditions.rs`:
      - Rewrite `evaluate_check` to use the new `CommandExecutor`.

4.  **Update Tests (`/tests`)**
    - Create `tests/command_executor_array_test.rs`.
    - Add tests for simple command execution, pipes, redirects, and conditionals using the `array:` syntax.
    - Add a specific test to confirm that an argument like `"; ls -la"` is treated as a literal string and does not execute `ls`.
    - Update `tests/action_execution_integration_test.rs` and other relevant tests to use the new `array:` format.

## Risk Mitigation

- **Breaking Change**: This is a required and intentional breaking change to the policy format. All documentation and example policies (`src/cli/commands/init.rs`) must be updated.
- **Complexity of I/O Handling**: Managing `stdin`/`stdout` for pipelines can be complex. The implementation must carefully handle process handles and I/O streams to avoid deadlocks. `tokio::process` is well-suited for this.
