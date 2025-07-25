# MCP Tool Matching Patterns

## Overview

Model Context Protocol (MCP) tools in Claude Code follow a naming convention of `mcp__<server>__<tool>`. Cupcake policies can match these tools using regular expressions to create targeted guardrails for specific MCP servers or tools.

## MCP Tool Naming Convention

MCP tools are exposed to Claude with names following this pattern:
- `mcp__filesystem__read_file`
- `mcp__github__create_issue`
- `mcp__slack__send_message`

The format is: `mcp__<server_name>__<tool_name>`

## Policy Matching Patterns

### Match All MCP Tools

To create policies that apply to all MCP tools:

```yaml
PreToolUse:
  "mcp__.*":
    - name: log-all-mcp-usage
      description: Log all MCP tool usage
      conditions: []
      action:
        type: provide_feedback
        message: "MCP tool {{tool_name}} is being used"
```

### Match Specific MCP Server

To target all tools from a specific MCP server:

```yaml
PreToolUse:
  "mcp__filesystem__.*":
    - name: restrict-filesystem-access
      description: Restrict filesystem MCP server operations
      conditions:
        - type: pattern
          field: tool_input.path
          regex: "^/etc/"
      action:
        type: block_with_feedback
        feedback: "Access to /etc/ via MCP filesystem server is restricted"
```

### Match Specific MCP Tool

To target a specific tool from a specific server:

```yaml
PreToolUse:
  "mcp__github__create_issue":
    - name: validate-github-issues
      description: Ensure GitHub issues have proper labels
      conditions:
        - type: pattern
          field: tool_input.title
          regex: "^\\[.+\\]"  # Require [TAG] prefix
      action:
        type: allow
```

### Complex MCP Patterns

You can use more sophisticated regex patterns:

```yaml
PreToolUse:
  # Match any MCP tool that modifies data
  "mcp__.*(create|update|delete|write|send).*":
    - name: audit-mcp-modifications
      description: Audit all MCP tools that modify data
      conditions: []
      action:
        type: run_command
        spec:
          mode: array
          command: ["logger"]
          args: ["MCP modification: {{tool_name}}"]

  # Match filesystem or github MCP servers
  "mcp__(filesystem|github)__.*":
    - name: require-confirmation
      description: Require confirmation for filesystem/github operations
      conditions:
        - type: not
          condition:
            type: state_query
            filter:
              tool: Bash
              command_contains: "confirm"
              within_minutes: 5
      action:
        type: block_with_feedback
        feedback: "Please run 'confirm' command before using {{tool_name}}"
```

## State-Aware MCP Policies

Combine MCP patterns with StateQuery conditions for sophisticated workflows:

```yaml
UserPromptSubmit:
  "*":
    - name: mcp-usage-summary
      description: Provide summary of recent MCP tool usage
      conditions:
        - type: pattern
          field: prompt
          regex: "(?i)what.*mcp.*tools"
        - type: state_query
          filter:
            tool: "mcp__.*"  # Regex patterns work in StateQuery too
            within_minutes: 60
          expect_exists: true
      action:
        type: inject_context
        context: |
          Recent MCP tool usage in the last hour:
          - Check session transcript for detailed MCP operations
          - Use 'cupcake inspect' to see active MCP policies
```

## Best Practices

1. **Be Specific**: Use the most specific pattern that meets your needs
   - `mcp__github__.*` is better than `mcp__.*` if you only care about GitHub

2. **Layer Policies**: Create general MCP policies and specific overrides
   ```yaml
   # General MCP policy
   "mcp__.*":
     - name: log-all-mcp
       ...
   
   # Specific override for dangerous operations
   "mcp__filesystem__(delete|remove).*":
     - name: block-dangerous-filesystem
       ...
   ```

3. **Use Conditions**: Don't just match on tool name, also check inputs
   ```yaml
   "mcp__slack__send_message":
     - name: prevent-spam
       conditions:
         - type: state_query
           filter:
             tool: "mcp__slack__send_message"
             within_minutes: 1
           expect_exists: false  # No messages in last minute
   ```

4. **Document Intent**: Use clear names and descriptions
   ```yaml
   "mcp__.*__delete.*":
     - name: mcp-deletion-safeguard
       description: |
         Prevent accidental deletions via MCP tools.
         Requires explicit confirmation before any delete operation.
   ```

## Integration with Claude Code

When Claude Code has MCP servers configured, these patterns automatically apply. The MCP tools appear alongside built-in tools like Read, Write, and Bash, and Cupcake policies treat them uniformly.

Example `.claude/settings.json` with MCP:
```json
{
  "mcpServers": {
    "filesystem": {
      "command": "npx",
      "args": ["@modelcontextprotocol/server-filesystem", "/home/user"]
    }
  }
}
```

This would expose tools like:
- `mcp__filesystem__read_file`
- `mcp__filesystem__write_file`
- `mcp__filesystem__list_directory`

All of which can be matched by Cupcake policies using the patterns documented above.