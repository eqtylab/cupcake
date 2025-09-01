# Cupcake Builtins Demo

This example demonstrates 4 builtin abstractions working together to provide comprehensive security without writing custom Rego policies.

## Enabled Builtins

1. **never_edit_files** - Blocks all file write operations (active)
2. **always_inject_on_prompt** - Injects demo context on every prompt
3. **git_pre_check** - Validates before git operations
4. **post_edit_check** - Would validate after edits (blocked by never_edit_files)

## Testing the Demo

```bash
# From the cupcake-rewrite directory
cargo run -- examples/builtins_demo/.cupcake
```

Then test with various Claude Code events:

### Test 1: File Edit Block
```json
{
  "hook_event_name": "PreToolUse",
  "tool_name": "Edit",
  "params": {
    "file_path": "test.rs",
    "old_string": "fn main()",
    "new_string": "fn main() {"
  }
}
```
Expected: HALT with message about editing being disabled

### Test 2: User Prompt Context
```json
{
  "hook_event_name": "UserPromptSubmit",
  "prompt": "Help me understand this code"
}
```
Expected: Context added about demo session

### Test 3: Git Operation Check
```json
{
  "hook_event_name": "PreToolUse",
  "tool_name": "Bash",
  "params": {
    "command": "git commit -m 'test'"
  }
}
```
Expected: Pre-checks run, may halt if checks fail


## Configuration

All configuration is in `.cupcake/guidebook.yml`. You can:
- Enable/disable individual builtins
- Customize messages and checks
- Add file patterns and contexts
- Configure validation commands

## How It Works

1. The guidebook.yml enables builtins
2. Engine auto-generates required signals
3. Builtin policies in `policies/builtins/` are loaded
4. Policies use generated signals for dynamic behavior
5. No custom Rego needed for common patterns!