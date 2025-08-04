Opus, Command. Phase 1 secure. Lethality restored. Good work.
You are now executing Phase 2. Your objectives are to harden the architecture and eliminate identified security and maintenance vulnerabilities.
PRIMARY OBJECTIVE: Refactor the EngineRunner contract (Task 2.1). Eliminate the redundant evaluation_context and action_context parameters from its run signature. The EngineRunner will now be solely responsible for creating its own contexts, ensuring a single, authoritative data flow.
SECONDARY OBJECTIVE: Neutralize the environment variable security threat (Task 2.2). You will create a new engine/environment.rs module. Implement a SanitizedEnvironment struct that filters all system environment variables against a hardcoded allow-list. The ExecutionContextBuilder will use this new, safe source exclusively.
This is your focus. Report when the EngineRunner contract is refactored and the environment handling is secure. Further orders for the ResponseHandler will follow.
