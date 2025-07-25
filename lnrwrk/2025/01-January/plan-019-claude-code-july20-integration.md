# Plan 019: Claude Code July 20 Integration

## Overview
Transform Cupcake from a reactive policy enforcer to a proactive behavioral guidance system by integrating Claude Code's July 20, 2024 hooks update. This enables context injection and two-way communication between Cupcake and Claude Code.

## Phases

### Phase 1: Core Response System (✅ COMPLETED)
- ✅ Update PolicyDecision enum to EngineDecision with Allow/Block/Approve/Ask
- ✅ Create CupcakeResponse struct matching JSON hook contract
- ✅ Refactor response handler to support both exit codes and JSON
- ✅ Rename Action::Approve to Action::Allow for clarity
- ✅ Write comprehensive tests for new response format
- ✅ Committed: feat: implement claude code july 20 response system

### Phase 2: Context Injection (✅ COMPLETED)
- ✅ Add InjectContext action to Action enum
- ✅ Implement UserPromptSubmit stdout handling for context injection
- ✅ Support both stdout (exit 0) and JSON methods
- ✅ Test context injection with various scenarios
- ✅ Committed: feat: implement context injection for behavioral guidance

### Phase 3: Robust Sync Command (✅ COMPLETED)
- ✅ Implement intelligent sync command that manages .claude/settings.local.json
- ✅ Auto-discover settings file location with fallback to ~/.config/claude/settings.json
- ✅ Support dry-run mode to preview changes
- ✅ Preserve existing user settings while merging Cupcake hooks
- ✅ Update TUI to generate modern hook configurations
- ✅ Test with various settings.json states
- ✅ Committed: feat: implement robust sync command for claude code hooks

### Phase 4: Stateful Context Engine (✅ COMPLETED)
- ✅ Add StateQuery condition variant to conditions system
- ✅ Implement StateQueryFilter for querying historical tool usage:
  - Filter by tool name
  - Filter by command patterns  
  - Filter by success/failure results
  - Filter by time windows (within_minutes)
- ✅ Integrate StateManager with ConditionEvaluator
- ✅ Fix policy matcher logic to support "*" wildcard for all events
- ✅ Create comprehensive tests for stateful context injection:
  - Test InjectContext action
  - Test StateQuery conditions with various filters
  - Test time-based constraints
  - Test expect_exists true/false logic
  - Test complex multi-condition policies
  - Test full policy evaluation with state
- ✅ Create example policies demonstrating stateful workflows:
  - Test-driven development enforcement
  - Safe deployment checklists
  - Code review best practices

### Phase 5: Final Integration (🔄 PENDING)
- [ ] Add $CLAUDE_PROJECT_DIR support for project-specific policies
- [ ] Document MCP tool matching patterns (mcp_*)
- [ ] Update README with behavioral guidance examples
- [ ] Update all documentation files
- [ ] Create final integration tests
- [ ] Complete plan and merge to main

## Key Achievements

### StateQuery Condition
The StateQuery condition enables policies to make decisions based on historical tool usage:

```yaml
conditions:
  - type: state_query
    filter:
      tool: Bash
      command_contains: "npm test"
      result: success
      within_minutes: 30
    expect_exists: true
```

This allows sophisticated workflows like:
- Ensuring tests pass before commits
- Preventing dangerous operations after specific actions
- Providing contextual reminders based on recent activity
- Enforcing time-based workflow requirements

### Context Injection
The InjectContext action provides non-blocking behavioral guidance:

```yaml
action:
  type: inject_context
  context: |
    ⚠️ Recent test failures detected!
    Please fix failing tests before committing.
  use_stdout: true
```

### Policy Matcher Enhancement
Fixed the policy matcher logic to properly handle the "*" wildcard, which now matches all events (both tool and non-tool events like UserPromptSubmit).

## Implementation Notes

1. **Two-Pass Evaluation**: The policy evaluator caches condition results to ensure consistent evaluation across both passes.

2. **State Loading Optimization**: Session state is only loaded when policies actually use StateQuery conditions, improving performance.

3. **Stdout vs JSON**: UserPromptSubmit events support both stdout (simple) and JSON (structured) response methods.

4. **Time-Based Queries**: StateQuery supports `within_minutes` to enable time-based workflow policies.

## Testing

Created comprehensive test suites:
- `tests/stateful_context.rs`: Unit tests for StateQuery and InjectContext
- `tests/stateful_policies.rs`: Integration tests demonstrating real-world workflows
- `examples/policies/stateful/`: Example YAML policies for common scenarios

All tests passing ✅

## Next Steps

Phase 5 will complete the integration with:
- Project-specific policy support via $CLAUDE_PROJECT_DIR
- MCP tool pattern documentation
- Comprehensive documentation updates
- Final integration testing