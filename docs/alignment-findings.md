# Claude Code Alignment Findings

## 1. Default Timeout ✅ FIXED
- **Status**: Fixed in commit
- **Change**: Updated from 30 seconds to 60 seconds to match Claude Code
- **Files Modified**: 
  - `src/config/types.rs` - Changed `default_timeout_ms()` from 30000 to 60000
  - Updated documentation and tests

## 2. Per-Command Timeout ✅ ALREADY SUPPORTED
- **Status**: Cupcake DOES support per-command timeout
- **Implementation**: 
  - `RunCommand` action has `timeout_seconds: Option<u32>` field
  - When specified, overrides global timeout for that command
  - Converts to milliseconds: `timeout_seconds * 1000`
- **Example YAML**:
  ```yaml
  action:
    type: run_command
    spec:
      mode: array
      command: ["./long-running-script.sh"]
    timeout_seconds: 120  # Override to 2 minutes
  ```

## 3. Parallel Hook Execution ⚠️ SEQUENTIAL
- **Status**: Cupcake executes policies SEQUENTIALLY, not in parallel
- **Evidence**: 
  - `execute_matched_actions()` uses a for loop (line 142 in engine.rs)
  - Each policy action is executed one after another
  - No async/concurrent execution found
- **Impact**: 
  - Performance difference when multiple policies match
  - Claude Code runs all matching hooks in parallel for better performance
  - This is a significant architectural difference

## Summary

1. **Timeout Alignment**: ✅ Complete - Now matches Claude Code's 60-second default
2. **Per-Command Timeout**: ✅ Already supported via `timeout_seconds` field
3. **Parallel Execution**: ⚠️ Major difference - Cupcake is sequential, Claude Code is parallel

The parallel execution difference is the most significant finding and represents a real architectural divergence from Claude Code's behavior.