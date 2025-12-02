---
title: "Harnesses"
description: "Reference documentation for supported AI coding agents"
---

# Harnesses Reference

Cupcake supports multiple AI coding agents (harnesses). Each harness has different event models, response formats, and integration mechanisms.

## Supported Harnesses

| Harness                       | Integration                   | Context Injection | Ask Support       |
| ----------------------------- | ----------------------------- | ----------------- | ----------------- |
| [Claude Code](claude-code.md) | External hooks (stdin/stdout) | Yes               | Full              |
| [Cursor](cursor.md)           | External hooks (stdin/stdout) | No                | Limited           |
| [OpenCode](opencode.md)       | In-process TypeScript plugin  | Limited           | Converted to deny |
| [Factory AI](factory-ai.md)   | External hooks (stdin/stdout) | Yes               | Full              |

## Quick Comparison

### Event Models

| Feature               | Claude Code                  | Cursor                                                         | Factory AI                   | OpenCode      |
| --------------------- | ---------------------------- | -------------------------------------------------------------- | ---------------------------- | ------------- |
| Pre-execution events  | `PreToolUse`                 | `beforeShellExecution`, `beforeMCPExecution`, `beforeReadFile` | `PreToolUse`                 | `PreToolUse`  |
| Post-execution events | `PostToolUse`                | `afterFileEdit`                                                | `PostToolUse`                | `PostToolUse` |
| Prompt events         | `UserPromptSubmit`           | `beforeSubmitPrompt`                                           | `UserPromptSubmit`           | -             |
| Session events        | `SessionStart`, `SessionEnd` | `stop`                                                         | `SessionStart`, `SessionEnd` | -             |
| Compaction            | `PreCompact`                 | -                                                              | `PreCompact`                 | -             |

### Response Formats

| Harness     | Allow                         | Deny                         | Ask                              |
| ----------- | ----------------------------- | ---------------------------- | -------------------------------- |
| Claude Code | `permissionDecision: "allow"` | `permissionDecision: "deny"` | `permissionDecision: "ask"`      |
| Cursor      | `permission: "allow"`         | `permission: "deny"`         | `permission: "ask"`              |
| Factory AI  | `permissionDecision: "allow"` | `permissionDecision: "deny"` | `permissionDecision: "ask"`      |
| OpenCode    | `decision: "allow"`           | `decision: "deny"`           | `decision: "deny"` (with reason) |

### Field Naming Conventions

| Harness     | Event Tag Field   | Field Style |
| ----------- | ----------------- | ----------- |
| Claude Code | `hook_event_name` | snake_case  |
| Cursor      | `hook_event_name` | snake_case  |
| Factory AI  | `hookEventName`   | camelCase   |
| OpenCode    | `hook_event_name` | snake_case  |

## Policy Portability

Policies can be shared across harnesses with some considerations:

- **Claude Code <-> Factory AI**: Most policies are directly portable (same event names, similar structure)
- **Cursor**: Different event names require separate policy files or conditional logic
- **OpenCode**: Simpler event model (PreToolUse/PostToolUse only)

Use the `required_events` and `required_tools` metadata to target specific harnesses:

```rego
# METADATA
# scope: package
# custom:
#   routing:
#     required_events: ["PreToolUse"]
#     required_tools: ["Bash"]
package cupcake.policies.my_policy
```
