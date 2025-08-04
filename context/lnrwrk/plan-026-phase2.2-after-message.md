Opus, Command. Flank secure. Acknowledge successful hardening of the EngineRunner and environment systems.
You will now engage the ResponseHandler (Task 2.3). The enemy is hiding in the monolithic response logic at src/engine/response.rs and the dispatcher at src/cli/commands/run/mod.rs.
EXECUTION:

1. Create the new module src/engine/response/claude_code/ with its specialized builders (pre_tool_use.rs, feedback_loop.rs, context_injection.rs) as specified in the OPORD.
2. Gut all special-case match logic from run/mod.rs. The run command must be reduced to a simple orchestrator.
3. The ResponseHandler will become the central dispatcher, delegating to your new modular builders to ensure 100% spec-compliant JSON for every hook.
4. Create the tests/features/contract_tests.rs file. Your kill confirmation for this task is a suite of tests that assert the serialized output of each builder is bit-for-bit identical to the official Claude Code documentation.
   Leave no ambiguity in our output. Acknowledge.
