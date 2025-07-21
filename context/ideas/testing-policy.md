2.  **Comprehensive Policy Testing:** How can a developer trust their policies? By testing them. The current `validate` command checks syntax, but a `test` command could check _behavior_.

    - **Vision:** A `cupcake test` command could look for a `guardrails/tests/` directory. Developers could create test files like `test_dangerous_commands.yaml`:

      ```yaml
      # guardrails/tests/test_dangerous_commands.yaml
      - name: "Test rm -rf is blocked"
        # Mock hook event input
        input:
          hook_event_name: PreToolUse
          tool_name: Bash
          tool_input:
            command: "rm -rf /"
        # Assertions on the outcome
        expect:
          decision: block
          feedback_contains: "Dangerous command blocked"

      - name: "Test safe command is allowed"
        input:
          hook_event_name: PreToolUse
          tool_name: Bash
          tool_input:
            command: "ls -la"
        expect:
          decision: allow
      ```

    - This would allow policy development to follow a standard Test-Driven Development (TDD) workflow, dramatically increasing confidence and reliability.
