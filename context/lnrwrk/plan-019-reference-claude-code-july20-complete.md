# Claude Code July 20 Updates - Complete Technical Reference with Nuances

Created: 2025-01-25T10:30:00Z
Type: Comprehensive Technical Reference

## Overview

This document provides the COMPLETE technical details of Claude Code July 20 updates, including all nuances and clarifications discovered through careful analysis.

## Major New Features

### 1. $CLAUDE_PROJECT_DIR Environment Variable

**Technical Details:**
- New environment variable available ONLY when Claude Code spawns the hook command
- Provides absolute path to project root directory
- Enables portable, project-relative hook scripts
- NOT available in general shell environment

**Example:**
```json
{
  "command": "$CLAUDE_PROJECT_DIR/.claude/hooks/check-style.sh"
}
```

**Nuance:** This variable is injected by Claude Code at runtime, making hooks portable across different machines and project locations.

### 2. Enhanced Matcher Syntax

**Original Limitations:**
- `"matcher": "*"` was INVALID - would cause errors
- Only `"matcher": ""` or omitting matcher worked for "match all"

**New Flexibility:**
- `"matcher": "*"` - Now VALID for matching all tools
- `"matcher": ""` - Still works (backward compatible)
- Omitting `matcher` field entirely - Still works
- All three methods are now equivalent for "match all"

**Configuration Flexibility for Non-Tool Events:**
```json
{
  "hooks": {
    "UserPromptSubmit": [
      {
        // Can completely omit "matcher" field for these events
        "hooks": [
          {
            "type": "command",
            "command": "/path/to/script.py"
          }
        ]
      }
    ]
  }
}
```

### 3. MCP (Model Context Protocol) Tool Support

**Completely New Feature:**
- Support for MCP tools with special naming pattern
- Pattern: `mcp__<server>__<tool>`
- Examples:
  - `mcp__memory__create_entities`
  - `mcp__filesystem__read_file`
  - `mcp__github__search_repositories`

**Matching Strategies:**
```json
{
  "matcher": "mcp__memory__.*",        // All memory server tools
  "matcher": "mcp__.*__write.*",       // All write operations across MCP servers
  "matcher": "mcp__github__search.*"   // Specific GitHub search tools
}
```

### 4. PreCompact Hook Event (NEW)

**Previously Undocumented Event:**
- Fires before Claude Code compacts the context
- Two trigger types with different matchers:
  - `"manual"` - User invoked via `/compact` command
  - `"auto"` - Automatic due to full context window

**Input Schema:**
```json
{
  "session_id": "abc123",
  "transcript_path": "~/.claude/projects/.../00893aaf-19fa-41d2-8238-13269b9b3ca0.jsonl",
  "hook_event_name": "PreCompact",
  "trigger": "manual" | "auto",
  "custom_instructions": ""  // Only populated for manual trigger
}
```

### 5. Enhanced JSON Output Control

#### A. Universal Fields (All Hooks)

**New Common Control Fields:**
```json
{
  "continue": true | false,      // Master control - overrides everything
  "stopReason": "string",        // Shown to user when continue=false
  "suppressOutput": true | false // Hide from transcript mode
}
```

**Critical Precedence Rule:** `"continue": false` ALWAYS takes precedence over any other decision fields.

#### B. PreToolUse Permission Model

**New Granular Control:**
```json
{
  "hookSpecificOutput": {
    "hookEventName": "PreToolUse",
    "permissionDecision": "allow" | "deny" | "ask",
    "permissionDecisionReason": "Explanation"
  }
}
```

**Behavior Details:**
- `"allow"` - Bypasses permission system, reason shown to user only
- `"deny"` - Blocks tool call, reason shown to Claude for correction
- `"ask"` - NEW: Prompts user for confirmation with reason

**Deprecated but Functional:**
```json
{
  "decision": "approve" | "block",
  "reason": "string"
}
```

#### C. UserPromptSubmit Special Behavior

**Critical Nuance - Two Methods for Context Injection:**

**Method 1 - Simple (Special Case):**
```python
# Exit code 0 + stdout = context injection
print("Current project status: 5 tests failing")
sys.exit(0)
```

**Method 2 - JSON Control:**
```python
output = {
    "hookSpecificOutput": {
        "hookEventName": "UserPromptSubmit",
        "additionalContext": "Current project status: 5 tests failing"
    }
}
print(json.dumps(output))
sys.exit(0)
```

**Both methods achieve the same result** - the text is added to Claude's context.

**Important:** This stdout → context behavior is UNIQUE to UserPromptSubmit. For all other hooks, stdout with exit code 0 only appears in transcript mode.

## Behavioral Changes and Nuances

### 1. Exit Code Behavior Matrix

| Hook Event | Exit 0 | Exit 2 | Other |
|------------|--------|--------|-------|
| PreToolUse | Allow (stdout → transcript) | Block (stderr → Claude) | Error (continue) |
| PostToolUse | Continue (stdout → transcript) | Feedback (stderr → Claude) | Error (continue) |
| UserPromptSubmit | **SPECIAL: stdout → context** | Block (stderr → user) | Error (continue) |
| Notification | Continue (stdout → transcript) | N/A (stderr → user) | Error (continue) |
| Stop/SubagentStop | Continue (stdout → transcript) | Block (stderr → Claude) | Error (continue) |
| PreCompact | Continue (stdout → transcript) | N/A (stderr → user) | Error (continue) |

### 2. JSON Precedence Rules (Crystal Clear)

**Order of Precedence:**
1. `"continue": false` - ALWAYS wins, stops everything
2. Hook-specific decisions (`permissionDecision`, `decision`)
3. Default behavior if no decision specified

**Example showing precedence:**
```json
{
  "continue": false,                    // This wins
  "stopReason": "Critical error",       // User sees this
  "decision": "block",                  // IGNORED due to continue=false
  "reason": "This is never processed"   // IGNORED
}
```

### 3. Security Model Nuances

**Configuration Snapshot System:**
- Claude Code captures hook configuration at startup
- External modifications to settings files don't affect running session
- Changes require review via `/hooks` menu to take effect
- Prevents malicious runtime hook injection

**Implications:**
- Safe from external tampering during session
- Must restart Claude Code or use `/hooks` to apply changes
- Provides audit trail of hook modifications

### 4. Timeout Behavior Details

**Per-Command Timeout Control:**
```json
{
  "hooks": [
    {
      "type": "command",
      "command": "quick-check.sh",
      "timeout": 5  // 5 seconds
    },
    {
      "type": "command", 
      "command": "slow-analysis.py",
      "timeout": 300  // 5 minutes
    }
  ]
}
```

**Key Behavior:**
- Default timeout: 60 seconds
- Individual timeouts don't affect other commands
- All commands in array run in parallel
- One timeout doesn't cascade to others

### 5. Hook Execution Environment

**Available Environment:**
- Current working directory from hook input `cwd` field
- User's environment variables
- `CLAUDE_PROJECT_DIR` (only when Claude Code spawns command)
- stdin contains JSON hook data

**Execution Details:**
- Parallel execution for multiple matching hooks
- Output aggregation before returning to Claude
- Progress shown in transcript mode (Ctrl-R) for most hooks
- Notification hooks only log in debug mode

## Complete Hook Event Reference

### Tool-Based Events (use matchers)
1. **PreToolUse** - Before tool execution, can block
2. **PostToolUse** - After tool execution, can provide feedback

### Non-Tool Events (no matchers needed)
3. **UserPromptSubmit** - Before prompt processing, can inject context
4. **Notification** - On Claude notifications
5. **Stop** - When main agent finishes
6. **SubagentStop** - When sub-agent (Task tool) finishes
7. **PreCompact** - Before context compaction (manual/auto)

## Critical Implementation Considerations

### 1. UserPromptSubmit as Game Changer
- Only hook that can inject context with simple stdout
- Enables proactive behavior shaping
- Can block problematic prompts before processing
- Perfect for dynamic policy injection

### 2. Ask Permission Pattern
- New `"ask"` option enables user education
- Better UX than hard blocks for edge cases
- Builds trust while maintaining security

### 3. MCP Integration Possibilities
- Hook into any MCP tool operation
- Enable cross-server policies
- Monitor and control external integrations

### 4. Backward Compatibility Maintained
- Old syntax still works but deprecated
- Smooth migration path available
- No breaking changes for existing hooks

## Summary of All Nuances

1. **UserPromptSubmit stdout is special** - Goes to context, not just transcript
2. **MCP tools are fully supported** - New integration point
3. **PreCompact is a new event** - Not just the original events
4. **continue=false overrides everything** - Master kill switch
5. **Matcher flexibility increased** - Three ways to match all
6. **Security snapshot model** - Prevents runtime tampering
7. **Per-command timeouts** - Fine-grained control
8. **Ask permission is new** - Not just allow/deny
9. **JSON and exit code methods can coexist** - For UserPromptSubmit
10. **Project portability via $CLAUDE_PROJECT_DIR** - New deployment model

This represents a significant expansion of Claude Code's extensibility while maintaining a careful balance of power, security, and backward compatibility.