# Cupcake Inspect --verbose Enhancement Plan

## Current Output (Normal Mode)

```
NAME                    EVENT            TOOL ACTION              CONDITIONS
----------------------- ---------------- ---- ------------------- ----------
dangerous-command-check PreToolUse       Bash block_with_feedback tool_input.command ~ "rm -rf"
security-check          UserPromptSubmit *    block_with_feedback prompt ~ "password|secret"
inject-context          UserPromptSubmit *    inject_context      always

Total: 3 policies
```

## Proposed --verbose Output

### 1. Policy Details Section
```
=== Policy: dangerous-command-check ===
Event:       PreToolUse
Tool:        Bash
Description: Block dangerous commands
Source:      /path/to/cupcake.yaml:18-27

Conditions:
  - Pattern Match:
    Field: tool_input.command
    Regex: "rm -rf"
    
Action: block_with_feedback
  - Feedback: "Dangerous command blocked"
  - Suppress Output: false

---
```

### 2. Configuration Overview
```
=== Configuration Overview ===
Config File:    /path/to/cupcake.yaml
Imported Files: 
  - /path/to/policies/security.yaml
  - /path/to/policies/development.yaml

Settings:
  - Debug Mode: false
  - Allow Shell: false
  - Timeout: 60000ms (60s)
  - Sandbox UID: none

Total Policies: 15
By Event:
  - PreToolUse: 8
  - PostToolUse: 3
  - UserPromptSubmit: 3
  - SessionStart: 1
```

### 3. Action Details
For complex actions like conditional or run_command:

```
Action: conditional
  If: pattern match (prompt ~ "security")
  Then: inject_context "Security guidelines..."
  Else: provide_feedback "General guidelines..."
```

```
Action: run_command
  Command: ["./scripts/validate.sh", "{{prompt}}"]
  Mode: array (secure)
  Timeout: 30s
  On Failure: block
  On Failure Feedback: "Validation failed"
```

### 4. Validation Warnings
```
=== Validation Warnings ===
⚠️  Policy 'test-policy' has no description
⚠️  Policy 'broad-matcher' uses wildcard matcher - will match all tools
⚠️  Multiple policies match PreToolUse:Bash - execution order matters:
    1. dangerous-command-check
    2. audit-commands
    3. rate-limiter
```

### 5. Template Variable Usage
```
=== Template Variables Used ===
{{prompt}}       - Used in 3 policies
{{session_id}}   - Used in 2 policies
{{env.USER}}     - Used in 1 policy
{{tool_name}}    - Used in 5 policies
```

## Implementation Summary

The --verbose flag would add:
1. **Full policy details** with descriptions and source locations
2. **Configuration overview** showing all loaded files and settings
3. **Detailed action breakdown** especially for complex actions
4. **Validation warnings** for potential issues
5. **Template variable analysis** showing which variables are used where
6. **Import hierarchy** showing how policies were loaded
7. **Execution order** when multiple policies match the same event/tool

This would greatly help developers:
- Debug policy evaluation issues
- Understand execution order
- Find configuration problems
- See the full context of their policies
- Verify template variable usage