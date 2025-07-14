# Plan 011: Add Configurable Command Timeouts

Created: 2025-01-14T10:15:00Z
Depends: plan-007
Enables: plan-015
Priority: IMPORTANT

## Goal

Replace the hardcoded 30-second command timeout with configurable timeouts at both policy and global levels.

## Success Criteria

- Policy-level timeout configuration in YAML
- Global default timeout configuration
- Backward compatibility (30s default if not specified)
- Clear documentation for timeout behavior
- Tests for various timeout scenarios

## Context

Plan 007 hardcodes a 30-second default timeout in `ActionExecutor::execute_run_command()` using `timeout_seconds.unwrap_or(30)`. This one-size-fits-all approach doesn't work for real-world commands that range from quick checks (need <5s) to long builds (need >5min).

## User Impact

- **Current Problem**: Commands fail prematurely or waste time waiting
- **Solution**: Developers can set appropriate timeouts per command
- **Configuration Levels**: Per-action, per-policy file, global default
- **Affected Component**: `src/engine/actions.rs:171` and policy schema