# Plan 008, Part 3: Implement `shell:` Hatch and `encode` CLI

Created: 2025-07-15T10:10:00Z
Depends: plan-008-plan2
Enables: (none)
Priority: Medium

## Goal

Implement the `shell:` escape hatch for executing complex or legacy scripts, gated by a new `allow_shell` setting. Additionally, create the `cupcake encode` CLI command to improve developer experience by converting shell one-liners into the secure `array:` format.

## Success Criteria

1.  ✅ The `CommandSpec` enum is extended with a `Shell(ShellCommandSpec)` variant.
2.  ✅ A new global setting, `allow_shell: bool`, is added to `Settings` in `src/config/types.rs`.
3.  ✅ The `CommandExecutor` will only execute a `shell:` command if `allow_shell` is `true`, otherwise it returns an error.
4.  ✅ `shell:` commands are executed via `/bin/sh -c`, are properly sandboxed with timeouts, and their use is logged for auditing.
5.  ✅ A new `cupcake encode <command>` subcommand is added to the CLI.
6.  ✅ The `encode` command successfully parses a shell string and prints the equivalent, secure `array:` YAML to `stdout`.

## Context

While the `array:` and `string:` formats cover most use cases securely, some legacy scripts or complex shell grammar may require a true shell. This plan adds that capability as an explicit, auditable, and governable escape hatch. The `encode` tool encourages migration to the safer `array:` format.

## Technical Scope

1.  **Update Configuration (`src/config/`)**

    - In `src/config/actions.rs`:
      - Create `pub struct ShellCommandSpec { pub script: String }`.
      - Add the `Shell(ShellCommandSpec)` variant to the `CommandSpec` enum.
    - In `src/config/types.rs`:
      - Add `pub allow_shell: bool` to the `Settings` struct, defaulting to `false`.
    - In `src/config/loader.rs`:
      - Ensure the loader correctly reads the `allow_shell` setting.

2.  **Update Command Executor (`src/engine/command_executor/mod.rs`)**

    - Update the main `execute` function to handle the `CommandSpec::Shell` variant.
    - Inside this handler, first check the `allow_shell` setting from the loaded configuration. If `false`, return an `ActionResult::Error`.
    - If `true`, execute the `script` string using `tokio::process::Command::new("sh").arg("-c").arg(script)`.
    - This execution path should still respect timeouts and other sandboxing controls.

3.  **Implement `encode` CLI Command**

    - In `src/cli/app.rs`:
      - Add an `Encode { command: String }` variant to the `Commands` enum.
    - Create `src/cli/commands/encode.rs`:
      - Implement the `CommandHandler` for `EncodeCommand`.
      - The `execute` method will:
        - Take the input command string.
        - Use the `parser::parse_string_spec` function (from Part 2) to generate the `CommandGraph`.
        - Implement a "renderer" function that takes the `CommandGraph` and converts it into an `ArrayCommandSpec` struct.
        - Use `serde_yaml_ng::to_string` to serialize the `ArrayCommandSpec` struct to a YAML string.
        - Print the resulting YAML to standard output.
    - In `src/main.rs`:
      - Wire up the new `EncodeCommand`.

4.  **Update Tests (`/tests`)**
    - Add tests for the `shell:` command:
      - One test that shows it works when `allow_shell: true`.
      - One test that shows it is blocked and returns an error when `allow_shell: false` (the default).
    - Add a new CLI integration test for `cupcake encode`:
      - Run `cupcake encode 'npm test | grep PASS'`.
      - Capture `stdout` and parse the YAML to verify it has the correct `command`, `pipe`, etc. keys.

## Risk Mitigation

- **Security of `shell:`**: The risk is mitigated by making it opt-in (`allow_shell: false` by default), which forces a conscious security decision. Its usage should be logged conspicuously for auditing.
- **`encode` Tool Complexity**: The `encode` command's logic depends on the parser from Part 2. By building a renderer that converts the `CommandGraph` intermediate representation to an `ArrayCommandSpec`, we avoid duplicating logic.
