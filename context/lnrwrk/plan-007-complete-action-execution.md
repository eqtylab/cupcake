# plan 007: Complete Action Execution Integration

Created: 2025-07-13T16:00:00Z
Depends: plan-002
Enables: none

## Goal

Connect the fully-implemented ActionExecutor to the runtime flow, enabling UpdateState persistence and actual command execution in RunCommand actions. This completes the ~15% gap in Plan 002's implementation.

## Success Criteria

- UpdateState actions persist their state changes to `.cupcake/state/` files
- RunCommand actions execute actual commands instead of showing placeholder text
- Command execution works in both evaluation and action contexts
- Integration tests verify command execution with success/failure scenarios
- State persistence is atomic and handles concurrent access correctly
- All existing tests continue to pass

## Context

Comprehensive code review revealed that while all components are individually implemented:
- ActionExecutor can execute commands and generate state updates
- StateManager can persist state to disk
- PolicyEvaluator determines which policies match

However, these components aren't connected - the run command only uses PolicyEvaluator and never instantiates ActionExecutor, meaning actions are identified but not executed.

## Technical Scope

### 1. Run Command Integration
- Instantiate ActionExecutor in the run command flow
- After policy evaluation, execute actions for matching policies
- Handle both soft and hard actions appropriately

### 2. State Persistence 
- Connect ActionExecutor's state updates to StateManager
- Ensure UpdateState actions persist to session files
- Handle state update failures gracefully

### 3. Command Execution in Evaluation
- Remove placeholder logic in evaluation.rs:189-205
- Either execute commands during evaluation OR defer to post-evaluation execution
- Ensure consistent behavior between conditions and actions

### 4. Comprehensive Testing
- Integration tests for command execution scenarios
- State persistence verification tests  
- Concurrent access tests
- Failure scenario tests

## Risk Mitigation

- State corruption: Use atomic writes with temp files
- Long-running commands: Respect existing timeout configurations
- Concurrent access: StateManager already has session-based isolation
- Command injection: Template substitution already implemented safely