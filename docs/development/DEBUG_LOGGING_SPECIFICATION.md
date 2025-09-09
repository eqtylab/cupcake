# Debug Logging System - Developer Specification

**Document Version**: 1.0  
**Status**: Approved for Implementation  
**Created**: 2024-09-05  

## Overview

Add comprehensive debug logging to capture the complete lifecycle of every Claude Code event through the Cupcake policy engine, regardless of whether policies match or actions are taken.

## Key Design Principles

1. **Zero production impact** - Only enabled via `CUPCAKE_DEBUG_FILES` environment variable
2. **Single file per event** - Each Claude Code event generates one complete debug file
3. **Human-readable format** - Clear sections with intuitive separation
4. **Complete visibility** - Captures events even when no policies match
5. **Minimal code changes** - Centralized implementation in `eval_command()`

## Implementation Details

### 1. Debug Capture Structure (`cupcake-core/src/debug.rs`)

Create a new module with:

```rust
pub struct DebugCapture {
    pub event_received: Value,           // Raw Claude Code event
    pub trace_id: String,                // Unique identifier
    pub timestamp: SystemTime,           // When received
    pub routed: bool,                    // Did we find matching policies?
    pub matched_policies: Vec<String>,   // Which policies matched
    pub signals_configured: Vec<String>, // What signals were needed
    pub signals_executed: Vec<SignalExecution>, // Signal results
    pub wasm_decision_set: Option<DecisionSet>, // Raw WASM output
    pub final_decision: Option<FinalDecision>,  // Synthesized decision
    pub response_to_claude: Option<Value>,      // What we sent back
    pub actions_configured: Vec<String>,        // What actions were configured
    pub actions_executed: Vec<ActionExecution>, // Action results
    pub errors: Vec<String>,             // Any errors encountered
}

pub struct SignalExecution {
    pub name: String,
    pub command: String,
    pub result: Value,
    // Additional timing/exit code info can be captured in the text output
}

pub struct ActionExecution {
    pub name: String, 
    pub command: String,
    // Additional timing/exit code info can be captured in the text output
}
```

### 2. File Structure

**Location**: `.cupcake/debug/`  
**Format**: `YYYY-MM-DD_HH-MM-SS_<trace_id>.txt`

Example output:

```
===== Claude Code Event [2024-09-05 20:30:15] [abc123-def456] =====
Event Type: PreToolUse
Tool: Bash
Session ID: session-789

Raw Event:
{
  "hook_event_name": "PreToolUse",
  "tool_name": "Bash",
  "tool_input": { "command": "rm -rf /tmp/test" },
  ...
}

----- Routing -----
Matched: Yes (3 policies)
- cupcake.policies.security_policy
- cupcake.policies.builtins.rulebook_security_guardrails  
- cupcake.global.policies.system_protection

----- Signals -----
Configured: 2 signals
- __builtin_rulebook_protected_paths
- __builtin_system_protection_paths

Executed:
[__builtin_rulebook_protected_paths]
  Command: echo '["/etc", "/System"]'
  Duration: 5ms
  Result: ["/etc", "/System"]

[__builtin_system_protection_paths]
  Command: echo '[]'
  Duration: 3ms
  Result: []

----- WASM Evaluation -----
Decision Set:
  Halts: 0
  Denials: 1
    - [SECURITY-001] Dangerous command blocked: rm -rf (HIGH)
  Blocks: 0
  Asks: 0
  Allow Overrides: 0
  Context: 0

----- Synthesis -----
Final Decision: Deny
Reason: Dangerous command blocked: rm -rf

----- Response to Claude -----
{
  "continue": false,
  "stopReason": "Dangerous command blocked: rm -rf"
}

----- Actions -----
Configured: 1 action (on_any_denial)
Executed:
[log_denial]
  Command: echo "Denial logged" >> /tmp/denials.log
  Duration: 10ms
  Exit Code: 0

===== End Event [20:30:15.234] Duration: 45ms =====
```

### 3. Integration Points

#### `cupcake-cli/src/main.rs` - Primary capture point
- Initialize `DebugCapture` at start of `eval_command()`
- Pass capture object through the evaluation pipeline
- Write debug file at end if `CUPCAKE_DEBUG_FILES` is set

#### `cupcake-core/src/engine/mod.rs` - Engine integration
- Add optional `debug_capture: Option<&mut DebugCapture>` parameter to `evaluate()`
- Record routing decisions, signal execution, WASM results
- Pass through to action execution

#### `cupcake-core/src/harness/mod.rs` - Response capture
- Capture the formatted response before returning to Claude

### 4. Minimal Performance Impact
- All debug operations are gated by `if let Some(debug) = &mut debug_capture`
- No allocations or processing when debug is disabled
- File I/O happens once at the end of evaluation
- Uses existing tracing infrastructure where possible

### 5. Error Handling
- Debug logging failures should never break evaluation
- Use `warn!()` if debug file can't be written
- Continue normal processing even if debug fails

## Benefits

1. **Complete visibility** into policy evaluation for development
2. **Easy troubleshooting** - see exactly why policies fired or didn't
3. **Performance analysis** - timing data for each stage
4. **Signal debugging** - see what data signals are providing
5. **Action verification** - confirm actions executed correctly

## Files to Modify

1. Create `cupcake-core/src/debug.rs` - New debug module
2. Update `cupcake-core/src/lib.rs` - Export debug module
3. Modify `cupcake-cli/src/main.rs` - Add debug capture to eval_command
4. Update `cupcake-core/src/engine/mod.rs` - Thread debug through evaluation
5. Update `cupcake-core/src/harness/mod.rs` - Capture response formatting

## Testing Plan

1. Set `CUPCAKE_DEBUG_FILES=1` environment variable
2. Run various Claude Code events through the system
3. Verify debug files are created in `.cupcake/debug/`
4. Confirm no impact when environment variable is not set
5. Test with events that match no policies to ensure capture still works

## Future Scope (Not Planned)

### Additional Fields Not in Original Plan
- Session ID tracking
- Detailed timing breakdowns per phase (routing_duration_ms, signal_duration_ms, etc.)
- Separate error types (SignalError, ActionError)
- Complex PolicyMatch structures with metadata
- Response generation timing
- Working directory tracking for actions

### Advanced Features
- Web UI for browsing debug files
- Structured JSON output option
- Integration with external monitoring systems
- Debug file retention and rotation policies
- Real-time debug streaming
- Success metrics and acceptance criteria
- Risk assessment and mitigation strategies

These features could be valuable additions but are not part of the current implementation plan to maintain simplicity and focus on core debugging needs.

---

*This specification serves as the authoritative guide for debug logging system implementation. All implementation decisions should align with the architecture and requirements outlined in this document.*