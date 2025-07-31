# Plan for plan-022: Complete Removal of `string:` Command Mode

Created: 2025-07-31T14:00:00Z

## Approach

The primary goal is the complete and total eradication of the `string:` command execution mode from the Cupcake codebase. This removal will be absolute, with no consideration for backward compatibility. The codebase, documentation, and tests will be modified to reflect a reality where only the secure `array:` mode and the governed `shell:` escape hatch exist.

The process will be guided by the compiler. We will start by removing the core type definitions for string mode, which will intentionally break the build. We will then systematically resolve every compilation error, ensuring that every trace of the feature is removed from the logic, tests, and dependent components. Finally, we will perform a thorough sweep of all documentation and historical artifacts to erase any remaining mention of the feature.

## Steps

1.  **Remove Core Type Definitions**

    - In `src/config/actions.rs`:
      - Delete the `String(StringCommandSpec)` variant from the `CommandSpec` enum.
      - Delete the entire `pub struct StringCommandSpec`.

2.  **Delete the String Parser and its Dependency**

    - Delete the file `src/engine/command_executor/parser.rs`.
    - In `src/engine/command_executor/mod.rs`, remove the `mod parser;` declaration and any associated `use` statements.
    - In `Cargo.toml`, remove the `shell-words = "1.1.0"` dependency.

3.  **Update the Command Executor**

    - In `src/engine/command_executor/mod.rs`, remove the `CommandSpec::String` match arm from the `build_graph` method.
    - Delete the `build_graph_from_string` method entirely.

4.  **Purge Dedicated Test Files**

    - Delete `tests/string_command_spec_test.rs`.
    - Delete `tests/string_parser_integration_test.rs`.
    - Delete `tests/string_security_test.rs`.

5.  **Clean Up Cross-Functional Code and Tests**

    - In `src/cli/commands/inspect.rs`, remove the `CommandSpec::String` match arm from the `format_single_condition` helper function.
    - In `tests/cross_mode_security_test.rs`, remove all tests that involve or reference `StringCommandSpec` or `mode: string`. This includes `test_string_mode_no_shell_escalation`, `test_consistent_security_across_modes`, and `test_template_consistency_across_modes`.
    - Perform a project-wide search in the `/tests` directory for any remaining YAML or JSON fixtures using `mode: string` and convert them to `array` mode or remove them.

6.  **Scrub All Documentation**

    - In `README.md`:
      - Remove the "String Commands and Shell Execution" section.
      - Update any examples that use `mode: string` to use `mode: array` instead.
    - In `docs/command-execution.md`:
      - Delete the entire "String Mode" section.
      - Ensure all examples exclusively use `array` or `shell` mode.
    - In `docs/conditions-and-actions.md`:
      - Remove the mention of `string` mode for `check` conditions.
    - In `docs/secure-command-execution.md`:
      - Remove any text that contrasts `array` mode with the convenience of `string` mode.
    - Review all other `.md` files in `/docs` and the root directory for any lingering references to "string mode" and remove them.

7.  **Final Verification**
    - Run `cargo check --all-targets --all-features` to confirm there are no remaining compilation errors.
    - Run `just test-tui` to execute the entire test suite and ensure all tests pass.
    - Perform a final, case-insensitive project-wide search for `stringcommandspec` and `string mode` to guarantee complete removal.

## Technical Decisions (APPROVED)

- **Removal of `shell-words` Crate**: With the deletion of the string parser (`parser.rs`), the `shell-words` crate is no longer required and will be removed from `Cargo.toml` to reduce dependencies.
- **Removal of `cupcake encode` Command**: The `encode` CLI command was designed to migrate shell strings into the secure `array:` format, a task that relied on the string parser. As this parser is being removed, the `encode` command and its related files (e.g., `src/cli/commands/encode.rs`) will also be deleted.

### Other verifications:

Critical Security Problems

1. Fundamental Shell Parsing Complexity

The code attempts to reimplement shell parsing without actually using a shell, but this is inherently flawed:

- Incomplete Quote Handling: Lines 693-713 show that quoted operators like grep "|" file.txt are mishandled - the | inside quotes is still treated
  as a pipe operator
- Template Injection: Line 452-462 shows basic string replacement that could be exploited with crafted template values
- Path Traversal: No validation of file paths in redirects (lines 284, 312)

2. Shell-words Library Dependency Risk

Line 88 relies on the shell-words crate for tokenization, but then the code attempts to classify tokens manually. This creates a mismatch:

- Shell-words handles escaping and quoting correctly
- But the manual classification in classify_tokens() (lines 121-137) doesn't preserve quote context
- This means echo "rm -rf /" could potentially be mishandled

3. False Security Through Blacklisting

Lines 99-118 show a blacklist approach that's fundamentally flawed:

- Only blocks obvious patterns like $(...) and backticks
- Doesn't block other dangerous constructs like ${...}, process substitution <(...), etc.
- Misses shell expansions, glob patterns, and other dangerous features

4. Template Substitution Vulnerabilities

The template system (lines 452-462) is naive:
let placeholder = format!("{{{{{}}}}}", key);
result = result.replace(&placeholder, value);
This could lead to injection if template values contain shell metacharacters.

These would obviously be Eradicated by the completion of the plan, and obviously wouldn't linger. One of your final tasks in the implementation will be to validate that these problems are removed as well.Again, the only modes we'll support by the end of this is array mode and normal shell mode.If any of these security features apply to shell mode, we should document that. If there are flaws, we need to deal with that. But hopefully this is all confined to the string mode implementation.
