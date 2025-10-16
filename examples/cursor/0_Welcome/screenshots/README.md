# Screenshots Directory

This directory contains screenshots for the Cursor evaluation walkthrough.

## Required Screenshots

Please capture and add the following screenshots:

### cursor-block-rm.png
- **What**: Cursor being blocked from running `rm -rf` command
- **Shows**: The deny response with both user and agent messages
- **Key Point**: Demonstrates differentiated feedback

### cursor-block-cupcake.png
- **What**: Cursor blocked from accessing `.cupcake/` directory
- **Shows**: The rulebook_security_guardrails builtin protecting configuration
- **Key Point**: Shows Cupcake protecting itself from tampering

### cursor-block-git-no-verify.png
- **What**: Git commit with `--no-verify` flag being blocked
- **Shows**: The builtin policy preventing hook bypass
- **Key Point**: Demonstrates git workflow protection

### cursor-file-protection.png
- **What**: Cursor blocked from reading sensitive files like SSH keys
- **Shows**: File protection policy in action
- **Key Point**: Shows granular file access control

### cursor-mcp-protection.png
- **What**: MCP database operations being blocked
- **Shows**: DELETE or dangerous UPDATE being prevented
- **Key Point**: Demonstrates MCP tool protection

### cursor-inspect.png
- **What**: Output of `cupcake inspect --harness cursor`
- **Shows**: List of active policies for Cursor
- **Key Point**: Shows policy management interface

## Capturing Screenshots

1. Run the demo scenarios in Cursor
2. Capture the UI showing:
   - The user's request
   - Cursor's attempt to execute
   - The block message/response
   - Any agent feedback shown

3. Save with the exact filenames listed above
4. Ensure text is readable and UI elements are clear

## Placeholder Text

Until actual screenshots are captured, the README references these with placeholder text:
```
_[Screenshot placeholder: Shows ...]_
```

Replace these with actual screenshots when available.