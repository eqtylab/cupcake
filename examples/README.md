# Cupcake Examples

This directory contains example policies, configurations, and test events to demonstrate Cupcake's capabilities.

## Quick Test

```bash
# Test MCP memory tool (should be denied due to sensitive content)
cat examples/events/mcp_memory_create.json | cupcake eval --policy-dir examples/policies

# Test MCP GitHub tool (should ask for confirmation)  
cat examples/events/mcp_github_delete.json | cupcake eval --policy-dir examples/policies

# Test MCP filesystem tool (should be denied - system path)
cat examples/events/mcp_filesystem_read.json | cupcake eval --policy-dir examples/policies
```

## MCP Tool Support

Cupcake automatically recognizes MCP tools by their naming pattern: `mcp__<server>__<tool>`

### Policy Examples:
- `mcp_memory_guard.rego` - Protects against storing sensitive data in memory
- `mcp_filesystem_security.rego` - Blocks access to sensitive system paths  
- `mcp_github_operations.rego` - Controls destructive GitHub operations

### Test Events:
- `events/mcp_memory_create.json` - MCP memory tool usage
- `events/mcp_github_delete.json` - MCP GitHub repository deletion
- `events/mcp_filesystem_read.json` - MCP filesystem access

MCP tools work identically to built-in Claude Code tools - no special configuration needed.