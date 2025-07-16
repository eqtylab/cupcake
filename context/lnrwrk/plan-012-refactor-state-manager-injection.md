# Plan 012: Refactor StateManager Dependency Injection

Created: 2025-01-14T10:20:00Z
Depends: plan-008, plan-009
Enables: plan-016
Priority: MODERATE

## Goal

Refactor StateManager from being passed as `Option<&mut StateManager>` through multiple function layers to using proper dependency injection at construction time.

## Success Criteria

- StateManager injected at ActionExecutor construction
- Cleaner function signatures without StateManager parameters
- Thread-safe implementation using Arc<Mutex<>>
- Easier testing with mock StateManager
- No change in functionality

## Context

Plan 007 passes StateManager through multiple function calls as `Option<&mut StateManager>`, creating complex lifetime requirements and forcing single-threaded execution. This pattern pollutes APIs, makes testing harder, and prevents parallel action execution.

## Technical Debt

- **Current**: Complex lifetime management, API pollution
- **Target**: Clean dependency injection pattern
- **Benefits**: Simpler code, better testability, potential parallelism
- **Risk**: Moderate refactoring of action execution system