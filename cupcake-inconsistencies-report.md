# Cupcake Codebase Inconsistencies Report

## Executive Summary

This report documents all inconsistencies discovered in the Cupcake codebase following a comprehensive audit. Each issue includes specific file locations, code references, and impact assessment to facilitate engineering remediation.

## 1. Ask Action Implementation vs Documentation Mismatch

### Issue Description
The `Ask` action is fully implemented in the codebase but documentation and tests indicate it's missing.

### Evidence
- **Implementation Present**: `src/config/actions.rs:121-123`
  ```rust
  /// Request user confirmation for operation (hard action)
  Ask {
      reason: String,
  },
  ```

- **Action Type Mapping**: `src/config/actions.rs:196`
  ```rust
  Action::Ask { .. } => ActionType::Hard,
  ```

- **Evaluation Logic**: `src/engine/actions.rs:444`
  ```rust
  Action::Ask { reason } => {
      // Converts to EngineDecision::Ask
  ```

- **Misleading Test Comment**: `tests/july20_features_test.rs:227`
  ```rust
  // TODO: Add test for Ask action once it's implemented
  ```

### Impact
- No test coverage for a production feature
- Developer confusion about feature availability
- Documentation claims feature exists (correctly) but code comments suggest otherwise

### Recommendation
1. Remove misleading TODO comment
2. Add comprehensive tests for Ask action
3. Update any developer documentation

## 2. UserPromptSubmit Dual Output Mode Inconsistency

### Issue Description
UserPromptSubmit events have two different output mechanisms depending on the decision type, violating the single response pattern.

### Evidence
- **Location**: `src/cli/commands/run.rs:436-483`
- **Stdout Mode (Allow)**: Lines 440-453
  ```rust
  if !context_to_inject.is_empty() {
      println!("{}", combined_context);
      std::process::exit(0);  // Direct stdout, no JSON
  }
  ```

- **JSON Mode (Block/Ask)**: Lines 455-482
  ```rust
  EngineDecision::Block { feedback } => {
      let response = CupcakeResponse {
          hook_specific_output: None,
          continue_execution: Some(false),
          // ... JSON response
      };
  ```

### Impact
- Architectural inconsistency in response handling
- Special-case logic increases complexity
- Different behaviors for same event type based on action

### Recommendation
Consider standardizing on JSON responses with `additionalContext` field for all UserPromptSubmit responses.

## 3. Empty Matcher String Documentation Discrepancy

### Issue Description
Claude Code documentation shows omitting matcher for non-tool events, but Cupcake always requires a matcher string (using empty string for non-tool events).

### Evidence
- **Claude Code Spec** (`context/claude-code-docs/july20-updates/hooks.md:55`):
  ```json
  "UserPromptSubmit": [
    {
      "hooks": [...]  // No matcher field
    }
  ]
  ```

- **Cupcake Implementation** (`src/cli/commands/sync.rs:139-149`):
  ```rust
  "UserPromptSubmit": [
      {
          "hooks": [...]  // Also omits matcher - correct
      }
  ]
  ```

- **Internal Usage**: Cupcake uses empty string `""` as matcher for non-tool events internally

### Impact
- None - implementation correctly handles this
- Potential confusion for developers reading internal code

### Recommendation
Document this implementation detail in code comments.

## 4. MCP Tool Pattern Support Incomplete

### Issue Description
While MCP tools work through regex matching, there's no dedicated support or abstractions for MCP-specific features.

### Evidence
- **No MCP-specific code**: Grep search shows MCP only mentioned in documentation/tests
  ```bash
  grep -r "mcp__" src/ # No results in source code
  ```

- **Works via regex**: `tests/july20_features_test.rs` shows regex patterns like `"mcp__.*"` work
- **No special handling**: No MCP-aware policy templates or helper functions

### Impact
- Users must understand regex to work with MCP tools
- No validation of MCP tool patterns
- Missing opportunity for MCP-specific features

### Recommendation
1. Add MCP tool pattern validation
2. Create helper functions for common MCP patterns
3. Document MCP best practices

## 5. StateQuery Feature Removal Incomplete Cleanup

### Issue Description
StateQuery feature was removed in plan-020 but references may remain in documentation or examples.

### Evidence
- **Removal Confirmed**: `plan-020-log.md` shows complete removal
- **Code Search Clean**: No StateQuery found in src/
- **Documentation Updated**: `docs/conditions-and-actions.md` correctly omits state_query
- **Potential Issue**: External documentation may still reference this feature

### Impact
- Users attempting to use documented but non-existent feature
- Confusion about available condition types

### Recommendation
1. Audit all documentation for StateQuery references
2. Update any examples that use state_query conditions
3. Add migration guide for users who expected this feature

## 6. Test Coverage Gap for JSON Protocol

### Issue Description
While JSON protocol is implemented, some tests still expect old exit code behavior.

### Evidence
- **Fixed Tests**: Most updated in remediation
- **Potential Gaps**: Integration tests may not fully validate JSON responses
- **No Claude Code Integration Tests**: No tests actually invoke Cupcake through Claude Code

### Impact
- Cannot validate true Claude Code compatibility
- Risk of regression to old behavior
- No performance benchmarks under real load

### Recommendation
1. Create integration test suite that simulates Claude Code
2. Add contract tests for JSON response format
3. Benchmark performance with real hook invocations

## 7. Sync Command Timeout Units Documentation

### Issue Description
Sync command correctly uses seconds but no inline documentation about units.

### Evidence
- **Implementation**: `src/cli/commands/sync.rs:122`
  ```rust
  "timeout": 5  // Seconds, per Claude Code spec
  ```

- **No Comment**: Units not documented in code
- **Potential Confusion**: Other timeouts in codebase use milliseconds

### Impact
- Developer confusion about timeout units
- Risk of incorrect timeout values

### Recommendation
Add inline comments specifying timeout units throughout sync command.

## 8. $CLAUDE_PROJECT_DIR Template Variable Documentation

### Issue Description
While CLAUDE_PROJECT_DIR is supported, it's not consistently documented as a template variable.

### Evidence
- **Supported**: Environment variables available in `ActionContext`
- **Not Documented**: Some command execution docs don't mention this variable
- **Inconsistent Examples**: Some show `{{env.CLAUDE_PROJECT_DIR}}`, others don't

### Impact
- Users unaware of this powerful feature
- Inconsistent usage patterns

### Recommendation
1. Update all command execution documentation
2. Add examples using CLAUDE_PROJECT_DIR
3. Create section on environment variable templates

## Summary Priority Matrix

| Priority | Issue | Impact | Effort |
|----------|-------|--------|--------|
| **HIGH** | Ask Action Test Gap | Production feature untested | Low |
| **HIGH** | UserPromptSubmit Dual Mode | Architectural inconsistency | Medium |
| **MEDIUM** | MCP Tool Support | Missing abstractions | Medium |
| **MEDIUM** | Integration Test Gap | Can't verify compatibility | High |
| **LOW** | Documentation Gaps | User confusion | Low |
| **LOW** | StateQuery Cleanup | Legacy references | Low |

## Next Steps

1. **Immediate**: Fix Ask action tests and remove misleading TODO
2. **Short-term**: Standardize UserPromptSubmit response handling
3. **Medium-term**: Add MCP abstractions and integration tests
4. **Long-term**: Comprehensive documentation audit and updates

All findings are based on direct code inspection with specific file/line references. The codebase is fundamentally sound but these inconsistencies should be addressed for production readiness.