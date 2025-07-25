# Claude Code July 20 Updates - Exact Technical Changes

Created: 2025-01-25T10:15:00Z
Type: Technical Reference Document

## Overview

This document captures the EXACT technical changes introduced in the Claude Code July 20 updates, focusing on what specifically changed in how Claude Code hooks work.

## New Features

### 1. $CLAUDE_PROJECT_DIR Environment Variable

**What's New:**
- A new environment variable `$CLAUDE_PROJECT_DIR` is available when Claude Code spawns hook commands
- Points to the root directory of the current project
- Enables project-relative hook scripts

**Example:**
```json
{
  "command": "$CLAUDE_PROJECT_DIR/.claude/hooks/check-style.sh"
}
```

**Important:** This variable is ONLY available when Claude Code spawns the command, not in your general shell environment.

### 2. Wildcard Matcher Support

**Original Behavior:**
- `"matcher": "*"` was INVALID
- Had to use `"matcher": ""` (empty string) or omit matcher entirely to match all tools

**New Behavior:**
- `"matcher": "*"` is now VALID and matches all tools
- `"matcher": ""` still works (backward compatible)
- Omitting matcher still works (backward compatible)

### 3. Enhanced JSON Output Control

#### A. PreToolUse Hook Changes

**New Fields:**
```json
{
  "permissionDecision": "allow" | "deny" | "ask",
  "permissionDecisionReason": "string explaining the decision",
  "hookSpecificOutput": {
    // Hook-specific fields
  }
}
```

**Deprecated but Still Supported:**
```json
{
  "decision": "approve" | "block",
  "message": "string"
}
```

**New Behavior:**
- `"ask"` option prompts user for confirmation with the reason
- More granular control over tool execution

#### B. UserPromptSubmit Hook Special Behavior

**Critical New Behavior:**
- When exit code is 0, stdout is AUTOMATICALLY added to Claude's context
- This happens BEFORE the JSON output parsing
- Enables context injection without blocking

**New JSON Fields:**
```json
{
  "hookSpecificOutput": {
    "additionalContext": "Context to add to Claude"
  }
}
```

#### C. Universal JSON Fields (All Hooks)

**New Common Fields:**
```json
{
  "continue": true | false,  // Should Claude continue processing
  "stopReason": "string",    // Shown to user if continue is false
  "suppressOutput": true | false  // Hide output from user
}
```

### 4. Task Tool Documentation

**What Changed:**
- Task tool is now explicitly documented as "Sub agent tasks"
- Links to sub agents documentation for more details
- Clarifies this is for launching sub-agents

## Behavioral Changes

### 1. JSON Output Precedence

**New Clear Rules:**
- `"continue": false` ALWAYS takes precedence
- If `"continue": false`, any `"decision": "block"` is ignored
- Makes control flow more predictable

### 2. UserPromptSubmit Context Injection

**Major Behavioral Change:**
- Exit code 0 + stdout = context injection
- Exit code 2 + stderr = block with feedback (unchanged)
- This is a SIGNIFICANT change in how UserPromptSubmit works

### 3. Timeout Behavior Clarification

**Clarified (not changed):**
- Individual command timeouts don't affect other commands in the array
- Each command runs independently

## Examples from Documentation

### Example 1: Exit Code Based Control
```bash
#!/bin/bash
# Validates bash commands for safety
input=$(cat)
command=$(echo "$input" | jq -r '.tool_input.command // empty')

if [[ "$command" == *"rm -rf"* ]] && [[ "$command" != *"--no-preserve-root"* ]]; then
    echo "Dangerous command detected: rm -rf without safeguards" >&2
    exit 2
fi

exit 0
```

### Example 2: JSON Output with Context Injection
```javascript
// For UserPromptSubmit - adds context about security
const input = JSON.parse(fs.readFileSync(0, 'utf-8'));

if (input.prompt.toLowerCase().includes('security')) {
    console.log(JSON.stringify({
        continue: true,
        hookSpecificOutput: {
            additionalContext: "Remember: Our security policies require..."
        }
    }));
}
```

### Example 3: PreToolUse with New Permission Model
```python
#!/usr/bin/env python3
import json
import sys

data = json.load(sys.stdin)
tool = data.get('tool_name')
file_path = data.get('tool_input', {}).get('file_path', '')

if tool in ['Write', 'Edit'] and file_path.endswith('.md'):
    # Auto-approve documentation changes
    print(json.dumps({
        "permissionDecision": "allow",
        "permissionDecisionReason": "Documentation files auto-approved"
    }))
else:
    # Default behavior
    sys.exit(0)
```

## What Stayed the Same

1. **Exit Code Behavior:**
   - Exit 0 = Allow (with special stdout handling for UserPromptSubmit)
   - Exit 2 = Block with stderr as feedback
   - Other exit codes = Treat as errors

2. **Hook Event Types:**
   - All existing hooks still fire at the same times
   - No new hook types added

3. **Basic Structure:**
   - Settings.json format unchanged
   - Array of commands per hook still supported
   - Timeout configurations unchanged

## Summary of Critical Changes

1. **$CLAUDE_PROJECT_DIR** - New env var for portable scripts
2. **Wildcard matcher** - `*` now valid for matching all tools
3. **Permission model** - New allow/deny/ask system for PreToolUse
4. **Context injection** - UserPromptSubmit stdout with exit 0 adds to context
5. **Universal control** - All hooks can use continue/stopReason/suppressOutput
6. **Deprecated syntax** - approve/block still works but permissionDecision preferred

These changes significantly expand what hooks can do while maintaining backward compatibility.