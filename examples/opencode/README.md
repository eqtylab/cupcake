# OpenCode Example Project

This is a complete working example of Cupcake + OpenCode integration.

## Quick Test

From this directory, test policy enforcement:

```bash
# Should DENY (--no-verify is blocked)
echo '{"hook_event_name":"PreToolUse","session_id":"test","cwd":"'$(pwd)'","tool":"bash","args":{"command":"git commit --no-verify"}}' | cupcake eval --harness opencode

# Should ALLOW
echo '{"hook_event_name":"PreToolUse","session_id":"test","cwd":"'$(pwd)'","tool":"bash","args":{"command":"git status"}}' | cupcake eval --harness opencode
```

## Directory Structure

```
examples/opencode/
├── .cupcake/
│   ├── rulebook.yml              # Configuration
│   └── policies/
│       └── opencode/
│           ├── system/
│           │   └── evaluate.rego # Required aggregator
│           ├── minimal_protection.rego
│           ├── git_workflow.rego
│           └── file_protection.rego
├── .opencode/
│   └── plugins/
│       └── cupcake/              # Pre-built plugin
└── 0_Welcome/                    # Reference copies of policies
```

## Using in Your Own Project

Copy the `.cupcake/` and `.opencode/` directories to your project:

```bash
cp -r .cupcake /path/to/your/project/
cp -r .opencode /path/to/your/project/
```

## Example Policies

### 0_Welcome/minimal_protection.rego

A simple starter policy that demonstrates basic command blocking:

- Blocks `git commit --no-verify` (bypasses hooks)
- Blocks `git push --force` (dangerous)
- Blocks `rm -rf` on system directories (critical)

**Use case**: Basic safety net for dangerous commands

### 0_Welcome/git_workflow.rego

Enforces git best practices and workflows:

- Asks for confirmation on lazy commit messages ("wip", "fix", etc.)
- Warns before pushing directly to main branch

**Use case**: Encouraging good git hygiene

### 0_Welcome/file_protection.rego

Protects sensitive files from modification:

- Blocks editing of `.env` files (secrets protection)
- Blocks writing to protected configuration files
- Warns when modifying `package.json` dependencies

**Use case**: Preventing accidental exposure of secrets or breaking configuration

## Testing Policies

You can test policies directly using the cupcake CLI:

```bash
# Test a PreToolUse event
echo '{
  "hook_event_name": "PreToolUse",
  "session_id": "test",
  "cwd": "'$(pwd)'",
  "tool": "bash",
  "args": {"command": "git commit --no-verify"}
}' | cupcake eval --harness opencode --policy-dir .cupcake/policies
```

Expected output for blocked command:

```json
{
  "decision": "deny",
  "reason": "The --no-verify flag bypasses pre-commit hooks..."
}
```

## Policy Structure

OpenCode policies use the same structure as other Cupcake harnesses:

```rego
# METADATA
# scope: package
# custom:
#   routing:
#     required_events: ["PreToolUse"]
#     required_tools: ["Bash"]
package cupcake.policies.opencode.your_policy

import rego.v1

# Deny decision (blocks execution)
deny contains decision if {
    # Your conditions here
    decision := {
        "rule_id": "RULE_ID",
        "reason": "Why this is blocked",
        "severity": "HIGH"
    }
}

# Ask decision (requires approval - converted to deny in Phase 1)
ask contains decision if {
    # Your conditions here
    decision := {
        "rule_id": "RULE_ID",
        "reason": "Why approval is needed",
        "question": "What to ask the user",
        "severity": "MEDIUM"
    }
}
```

## Available Tools

OpenCode policies can route on these tools:

- **Bash** - Shell command execution
- **Edit** - File editing (replace text)
- **Write** - File creation/overwriting
- **Read** - File reading
- **Grep** - Search file contents
- **Glob** - Find files by pattern
- **List** - List directory contents
- **Custom tools** - User-defined tools

## Event Types

### PreToolUse

Fired before a tool executes. Access tool arguments via:

```rego
input.tool_name      # e.g., "Bash", "Edit"
input.tool_input     # Tool-specific arguments
input.session_id     # Session identifier
input.cwd            # Current working directory
```

### PostToolUse (Phase 2)

Fired after a tool executes. Access result via:

```rego
input.tool_name
input.tool_input
input.tool_response  # Execution result
```

## Decision Types

| Decision | Effect                 | Use Case                                 |
| -------- | ---------------------- | ---------------------------------------- |
| `deny`   | Blocks execution       | Security violations, dangerous commands  |
| `ask`    | Prompts for approval\* | Risky but sometimes necessary operations |
| `allow`  | Permits execution      | Explicitly allow after checks            |

\*Note: In Phase 1, `ask` decisions are converted to `deny` with a message explaining approval is needed.

## Tips for Writing Policies

1. **Be specific**: Match exact tool names and command patterns
2. **Use severity**: Help users understand impact (CRITICAL, HIGH, MEDIUM, LOW)
3. **Provide context**: Explain WHY something is blocked, not just WHAT
4. **Offer alternatives**: Tell users what they should do instead
5. **Test thoroughly**: Use `cupcake eval` to test before deploying

## Next Steps

- Read the [Plugin Reference](../../../docs/agents/opencode/plugin-reference.md)
- Review [Integration Design](../../../docs/agents/opencode/integration-design.md)
- Explore builtin policies in `fixtures/global_builtins/`
