The sync Command's Complexity:
The plan correctly identifies the need to implement the sync command (src/cli/commands/sync.rs). However, the implementation must be very robust. It will be modifying a user's configuration file (.claude/settings.local.json), which could contain other, unrelated hooks.
Risk: A naive implementation could overwrite or corrupt the user's existing settings.
Mitigation (Implied by the plan's quality): The implementation must safely parse the existing JSON, intelligently merge the Cupcake hook configuration without destroying other entries, and write the file back. This is a solvable but non-trivial task that needs careful attention to detail.

Testing Strategy for the New Protocol
Observation: The current integration tests likely rely on checking exit codes and stderr. The plan's shift to a JSON stdout model will require a new testing approach.
Gap: The plan doesn't explicitly mention the need to create new integration tests (e.g., in tests/run_command_integration_test.rs) that capture stdout and parse the resulting JSON to validate decisions like allow, deny, and ask.
Implication: This is a minor point, as any competent developer would do this, but it's worth calling out. A robust suite of tests validating the full CupcakeResponse JSON structure for various scenarios will be essential for ensuring correctness.

Potential Flaw: Documentation and Example Debt:
The plan correctly notes that documentation will need to be updated. The significance of this should not be underestimated. The entire value proposition of Cupcake is changing.
Risk: If the documentation isn't thoroughly overhauled, users will not understand how to leverage the most powerful new features. The existing examples in docs/ and README.md will become misleading.
Mitigation: A dedicated workstream should be created to update docs/conditions-and-actions.md, docs/policy-format.md, and the README.md with new examples focused on UserPromptSubmit and inject_context. The "Proactive Guidance" example from plan-019-plan.md is a perfect candidate.

Gap: The current Condition enum in src/config/conditions.rs does not have a variant for querying the StateManager. The check type is exclusively for running external commands.
Implication: To unlock the full potential of stateful guidance, a new condition type, such as state_query, will need to be implemented. This is not explicitly listed as a step in the plan-019-plan.md but is a critical dependency for the advanced use cases envisioned. This should be added as a formal step, likely in Phase 4.
