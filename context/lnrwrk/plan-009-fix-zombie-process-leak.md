# Plan 009: Fix Zombie Process Resource Leak

Created: 2025-01-14T10:05:00Z
Depends: plan-008
Enables: plan-013
Priority: CRITICAL

## Goal

Fix the resource leak in background command execution where spawned processes become zombies because their Child handles are immediately dropped.

## Success Criteria

- Background processes properly tracked and cleaned up
- No zombie processes accumulate over time
- Process exit codes can be retrieved if needed
- Graceful cleanup on cupcake termination
- Tests verify zombie prevention

## Context

Plan 007's implementation of `execute_command_background()` spawns processes but immediately discards the Child handle with `Ok(_) => Ok(())`. This creates zombie processes on Unix systems that consume system resources until cupcake exits. Long-running cupcake instances could exhaust process table entries.

## Technical Details

- **Location**: `src/engine/actions.rs:408-417` in `execute_command_background()`
- **Issue**: `spawn()` returns `Child` handle that is immediately dropped
- **Impact**: Zombie processes accumulate, potential resource exhaustion
- **Solution Approach**: Track Child handles and implement proper cleanup