# Plan 008, Part 2: Implement Safe `string:` Parser

Created: 2025-07-15T10:05:00Z
Depends: plan-008-plan1
Enables: plan-008-plan3
Priority: High

## Goal

Implement the `string:` command syntax to provide a convenient, shell-like experience for policy authors. This involves creating a limited, safe parser that transforms a command string into the same secure `CommandGraph` used by the `array:` executor, ensuring no shell is ever invoked.

## Success Criteria

1.  ✅ The `CommandSpec` enum in `src/config/actions.rs` is extended with a `String(StringCommandSpec)` variant.
2.  ✅ A new parser correctly tokenizes a command string using the `shell-words` crate.
3.  ✅ The parser correctly identifies the V1.0 operators: `|`, `>`, `>>`, `&&`, `||`.
4.  ✅ The parser successfully transforms a valid `string:` into the internal `CommandGraph` representation.
5.  ✅ The parser correctly rejects any unsupported shell syntax (e.g., `$(...)`, backticks, globs for V1) with a clear `UnsupportedSyntax` error.
6.  ✅ The `CommandExecutor` can now execute a command specified via the `string:` syntax.
7.  ✅ End-to-end tests verify that `string:` commands are executed correctly and are immune to shell injection.

## Context

With the secure `array:` executor built in Part 1, this plan adds a convenience layer. Instead of forcing users to write verbose `array:` specs for simple commands, we can provide a familiar one-liner syntax that is parsed into the same secure intermediate representation, offering the best of both worlds: ergonomics and security.

## Technical Scope

1.  **Update Configuration Structs (`src/config/`)**

    - In `src/config/actions.rs`:
      - Create `pub struct StringCommandSpec { pub command: String }`.
      - Add the `String(StringCommandSpec)` variant to the `CommandSpec` enum.

2.  **Implement the String Parser (`src/engine/command_executor/parser.rs`)**

    - Create a new module file for the parser.
    - Add `shell-words` as a dependency in `Cargo.toml`. The documentation is in `context/lib_docs/shell-quotes-1.1.0.md`.
    - Implement a `parse_string_spec` function that:
      - Takes a `&str` as input.
      - Uses `shell_words::split()` to get an initial `Vec<String>`.
      - Iterates through the tokens, building up a `Command` struct (program + args).
      - When an operator token (`|`, `&&`, etc.) is found, it finalizes the current `Command` and adds it and the operator to a list of `Token`s.
      - If any token is not a recognized operator, it's treated as a command argument.
      - If any unsupported shell syntax is detected (or if `shell_words::split` fails), return an error.
    - Implement a function to convert the `Vec<Token>` into the `CommandGraph` used by the executor.

3.  **Integrate Parser into Executor (`src/engine/command_executor/mod.rs`)**

    - The main `execute` function of the `CommandExecutor` should be updated to handle a `CommandSpec::String` variant.
    - It will call the new `parser::parse_string_spec` to get the `CommandGraph` and then pass it to the existing `execute_graph` function from Part 1.

4.  **Update Tests (`/tests`)**
    - In `tests/command_executor_array_test.rs` (or a new `..._string_test.rs`):
      - Add tests for valid `string:` commands with pipes and redirects.
      - Add tests that verify unsupported syntax (e.g., `echo $(whoami)`) is rejected by the parser and returns an error, not executed.
      - Add a test to confirm that a `string:` command is just as safe from injection as an `array:` command.

## Risk Mitigation

- **Parser Complexity**: Writing parsers can be tricky. By leveraging the `shell-words` crate for the initial tokenization, we significantly reduce complexity. The main task is a simple linear scan to identify the small, fixed set of V1 operators.
- **Security Regressions**: The primary risk is accidentally introducing a security hole. This is mitigated by ensuring the parser has a strict allow-list of operators and rejects everything else, and by reusing the same secure `CommandGraph` executor from Part 1.
