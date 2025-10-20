# Cursor Fixture Policies

This directory contains **Cursor-specific example policies** for the Welcome walkthrough. These policies are adapted from the general Cupcake fixtures to work with Cursor's event model.

## Policies

### security_policy.rego
**Event**: `beforeShellExecution`

Blocks dangerous shell commands with **differentiated feedback**:
- **User sees**: "Dangerous command blocked: rm -rf"
- **Agent sees**: Technical guidance via `agent_context` field with specific alternatives

Example:
```rego
decision := {
    "reason": "Dangerous command blocked",
    "agent_context": "rm -rf detected. Use 'trash' command for safe deletion...",
    "severity": "CRITICAL"
}
```

### file_protection.rego
**Events**: `beforeReadFile`, `afterFileEdit`

Two-part protection:
1. **beforeReadFile** - Blocks reading sensitive files (.ssh/, .aws/, .env)
2. **afterFileEdit** - Validates system file modifications (/etc/)

### mcp_protection.rego
**Event**: `beforeMCPExecution`

Protects MCP database operations:
- **Denies**: DELETE, DROP, TRUNCATE operations
- **Asks**: Confirmation for UPDATE operations

### prompt_filter.rego
**Event**: `beforeSubmitPrompt`

Blocks prompts containing potential secrets:
- Detects secret-related keywords (password, api_key, secret, token)
- Uses regex to identify long random strings (20+ chars)

## Key Differences from Claude Code Fixtures

| Aspect | Claude Code | Cursor |
|--------|------------|--------|
| **Events** | `PreToolUse`, `PostToolUse` | `beforeShellExecution`, `afterFileEdit`, etc. |
| **Tool Access** | `input.tool_name`, `input.tool_input.command` | `input.command` (direct field) |
| **Feedback** | Single `reason` | `reason` + `agent_context` (differentiated) |
| **Context Injection** | `add_context` verb supported | Not supported by Cursor |

## Event-Specific Adaptations

### beforeShellExecution
- Direct access to `input.command` (not nested in `tool_input`)
- No `tool_name` field (event itself indicates shell execution)
- Supports `agent_context` for AI-specific feedback

### beforeReadFile / afterFileEdit
- File path in `input.file_path` (direct)
- `content` available in event for read operations (beforeReadFile only)
- Cursor provides file content before the operation

### beforeMCPExecution
- MCP tool name in `input.tool_name`
- Tool parameters in `input.tool_input`
- Similar structure to Claude Code but different event name

## Usage

These fixtures are copied by `setup.sh` in the Cursor Welcome walkthrough:

```bash
cp ../../fixtures/cursor/security_policy.rego .cupcake/policies/cursor/
cp ../../fixtures/cursor/file_protection.rego .cupcake/policies/cursor/
cp ../../fixtures/cursor/mcp_protection.rego .cupcake/policies/cursor/
cp ../../fixtures/cursor/prompt_filter.rego .cupcake/policies/cursor/
```

## Testing

Test these policies with sample events:

```bash
# Test shell command blocking
echo '{"hook_event_name":"beforeShellExecution","command":"rm -rf /tmp/test"}' | \
  cupcake eval --harness cursor

# Test file read protection
echo '{"hook_event_name":"beforeReadFile","file_path":"~/.ssh/id_rsa"}' | \
  cupcake eval --harness cursor

# Test MCP protection
echo '{"hook_event_name":"beforeMCPExecution","tool_name":"postgres","tool_input":"DELETE FROM users"}' | \
  cupcake eval --harness cursor
```

## Contributing

When updating these policies:
1. Test with actual Cursor events
2. Ensure `agent_context` provides helpful guidance
3. Match Cursor's event structure (not Claude Code's)
4. Update this README with any new patterns
