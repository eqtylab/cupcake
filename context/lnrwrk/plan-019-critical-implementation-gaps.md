# plan-019-critical-implementation-gaps.md

Created: 2025-01-26T14:30:00Z
Type: Critical Issue Analysis
Status: URGENT - Implementation Incomplete

## Executive Summary

The plan-019 Claude Code July 20 integration is **critically incomplete**. Core functionality is broken, and the implementation diverges significantly from the documented requirements. This analysis documents the exact gaps between what was planned and what was delivered.

## Critical Implementation Gaps

### 1. Run Command: Hybrid Communication Model ❌

**Required (plan-019-plan.md Phase 1):**
> "Abandon Exit-Code-Based Communication: We will fully commit to the JSON output format. The `run` command will no longer use different exit codes to signal outcomes; it will always exit 0 and communicate decisions (allow, deny, ask) via a structured JSON payload on `stdout`."

**Actual Implementation (`src/cli/commands/run.rs:467-486`):**
```rust
match &decision {
    EngineDecision::Allow => {
        std::process::exit(0);  // ❌ Still using exit codes!
    }
    EngineDecision::Block { feedback } => {
        eprintln!("{}", feedback);
        std::process::exit(2);  // ❌ Still using exit code 2!
    }
    _ => {
        // Only Approve and Ask use JSON
        let response = CupcakeResponse::from_pre_tool_use_decision(&decision);
        handler.send_json_response(response);
    }
}
```

**Impact:** Basic Allow/Block operations don't follow the new contract. This breaks the fundamental promise of JSON-based communication.

### 2. Sync Command: Completely Wrong Hook Format ❌

**Required (Claude Code July 20 format):**
```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "*",
        "hooks": [
          {
            "type": "command",
            "command": "cupcake run --event PreToolUse",
            "timeout": 60
          }
        ]
      }
    ]
  }
}
```

**Actual Implementation (`src/cli/commands/sync.rs:108-143`):**
```json
{
  "hooks": {
    "PreToolUse": {
      "command": "cupcake run PreToolUse",  // ❌ Wrong structure!
      "timeout": 5000,
      "description": "..."
    }
  }
}
```

**Critical Errors:**
- Missing array wrapper for hook event
- Missing `matcher` field
- Missing nested `hooks` array
- Wrong command format (missing `--event` flag)
- Using milliseconds instead of seconds for timeout

**Impact:** The sync command will generate invalid configurations that Claude Code will reject or ignore. Users cannot register Cupcake with Claude Code.

### 3. Ask Action: Not Implemented ❌

**Required (plan-019-plan.md Phase 1):**
> "Add the `Ask { reason: String }` variant to the `EngineDecision` enum."

**Actual State:**
- ✅ `EngineDecision::Ask` exists in `src/engine/response.rs`
- ✅ Response handling for Ask exists
- ❌ **No `Action::Ask` in the Action enum** (`src/config/actions.rs`)
- ❌ Cannot create policies that use Ask action
- ❌ Test comment: `"TODO: Add test for Ask action once it's implemented"`

**Impact:** Users cannot create policies that prompt for user confirmation, a key feature of the July 20 updates.

### 4. Documentation: Severely Outdated ❌

**Required Updates (plan-019-plan.md Documentation section):**
- Rename `approve` to `allow`
- Add `inject_context` action
- Add `ask` action
- Update from exit-code model to JSON model
- Add `UserPromptSubmit` as primary hook

**Actual Documentation (`docs/conditions-and-actions.md`):**
```markdown
### Action Types
1. **provide_feedback** - Show message (never blocks)
2. **block_with_feedback** - Block operation with message
3. **approve** - Auto-approve (bypass permission prompt)  # ❌ Should be "allow"
4. **run_command** - Execute a command
5. **update_state** - Record custom event to state
6. **conditional** - If/then/else based on a condition
# ❌ Missing: inject_context, ask
```

**Response Model Still Shows Exit Codes:**
```
Cupcake->>Hook: Exit 2 + Feedback  # ❌ Should be JSON response
```

**Impact:** Users following documentation will create invalid policies and misunderstand how Cupcake works.

### 5. Incomplete Feature Implementation 🟡

**Partially Implemented:**
- ✅ InjectContext action exists and works
- ✅ StateQuery condition implemented
- ✅ $CLAUDE_PROJECT_DIR support added
- ✅ MCP tool pattern matching works
- 🟡 BUT: These features work in a broken system due to issues 1-4

## Root Cause Analysis

### Why This Happened

1. **Partial Implementation Strategy:** The implementation attempted to maintain backward compatibility when plan-019 explicitly stated:
   > "backwards compatibility is NOT required - no migrations necessary - full update granted, remove old/unused code"

2. **Incomplete Phase 1:** The fundamental communication protocol change (Phase 1) was only partially implemented, creating a hybrid system that satisfies neither the old nor new contract.

3. **Missing Integration Testing:** No end-to-end tests verify that Cupcake actually works with Claude Code's new hook format.

## Severity Assessment

**CRITICAL**: The system is fundamentally broken for its intended purpose:

1. **Cannot integrate with Claude Code**: The sync command produces invalid configurations
2. **Inconsistent behavior**: Mix of exit codes and JSON creates unpredictable results
3. **Missing core features**: Ask action is a documented feature that doesn't exist
4. **Misleading documentation**: Users will be confused and create invalid configurations

## Required Fixes

### Priority 1: Fix Sync Command
- Implement correct hook format with array structure
- Add matcher field support
- Fix command format to use `--event` flag
- Convert timeout to seconds

### Priority 2: Complete JSON Communication
- Remove ALL `process::exit()` calls from `RunCommand::execute`
- Always return JSON response for all decisions
- Special handling for UserPromptSubmit context injection

### Priority 3: Implement Ask Action
- Add `Action::Ask { reason: String }` to the Action enum
- Add evaluation logic to convert Ask actions to EngineDecision::Ask
- Add tests for Ask action

### Priority 4: Update Documentation
- Comprehensive update of all documentation files
- Remove all references to exit codes
- Document new actions and JSON format
- Add examples of new features

## Validation Criteria

The implementation will be complete when:

1. `cupcake sync` generates valid Claude Code hook configurations that work
2. ALL decisions use JSON output (no exit codes except final exit 0)
3. Policies can use `ask` action
4. Documentation accurately reflects the implementation
5. Integration tests pass with actual Claude Code

## Conclusion

Plan-019 is not complete. The current state represents approximately 40% completion with critical architectural issues that prevent the system from functioning as designed. The hybrid approach violates the core architectural decision to "abandon exit-code-based communication" and creates a system that works with neither the old nor new Claude Code hook format.

**Recommendation:** Complete plan-019 implementation before any other work. The current state is not shippable.