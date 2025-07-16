# Plan 007: Complete Action Execution Integration

Created: 2025-07-13T00:00:00Z
Depends: plan-001, plan-002
Enables: plan-008

## Goal

Complete the integration of the ActionExecutor with the runtime evaluation system, ensuring all actions (RunCommand, UpdateState, feedback) are fully executed and state changes are properly persisted.

## Success Criteria

- ActionExecutor is properly invoked from the run command flow
- RunCommand actions execute actual commands instead of showing placeholders
- UpdateState actions persist state changes to the append-only state store
- State persistence integrates with existing SessionState management
- All action types have comprehensive integration tests
- Commands execute with proper error handling and timeout management
- State updates are atomic and handle concurrent access correctly

## Context

The ActionExecutor implementation is complete but disconnected from the main evaluation flow. Currently, the run command evaluates policies and determines which actions to take, but doesn't actually execute them through the ActionExecutor. This is the final piece needed to make Cupcake fully functional as a policy enforcement engine.

Key integration points:
- `src/cli/commands/run.rs` - needs to invoke ActionExecutor after evaluation
- `src/engine/action_executor.rs` - ready but not called
- `src/state/mod.rs` - needs state persistence methods for UpdateState
- Command execution shows "Would execute:" instead of actual execution

## Technical Scope

1. **Run Command Integration**
   - Modify run command to create ActionExecutor instance
   - Pass evaluation results to ActionExecutor::execute_actions()
   - Handle execution results and errors appropriately

2. **State Persistence**
   - Implement append_state method in SessionState
   - Ensure atomic writes to state files
   - Handle state key conflicts and merging

3. **Command Execution**
   - Remove placeholder behavior in command execution
   - Implement proper process spawning with tokio::process
   - Add timeout handling for long-running commands
   - Capture and return command output/errors

4. **Testing**
   - Integration tests for full evaluation → execution flow
   - Unit tests for state persistence operations
   - Tests for command execution with various scenarios
   - Error handling and edge case coverage

## Risk Mitigation

- State file corruption: Use atomic writes with temp files
- Command injection: Already validated in policy loading
- Concurrent state access: Use file locking or atomic operations
- Long-running commands: Implement configurable timeouts