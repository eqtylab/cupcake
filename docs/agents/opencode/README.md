# OpenCode Integration

Cupcake integrates with [OpenCode](https://opencode.ai) to enforce policies on AI coding agent actions.

## Getting Started

**[Quickstart Guide](./quickstart.md)** - Get up and running in 5 minutes

## How It Works

Unlike Claude Code/Cursor which use external hooks (stdin/stdout), OpenCode uses an **in-process plugin architecture**. Cupcake provides:

1. **TypeScript Plugin** (`cupcake-plugins/opencode/`) - Intercepts tool execution in OpenCode
2. **Rust Harness** - Evaluates policies via `cupcake eval --harness opencode`
3. **Example Policies** (`examples/opencode/`) - Ready-to-use policy templates

```
OpenCode → Plugin → cupcake eval → Policy Decision → Allow/Block
```

## Documentation

| Document                                         | Description                        |
| ------------------------------------------------ | ---------------------------------- |
| [quickstart.md](./quickstart.md)                 | 5-minute setup guide               |
| [installation.md](./installation.md)             | Detailed installation instructions |
| [plugin-reference.md](./plugin-reference.md)     | Plugin API and configuration       |
| [integration-design.md](./integration-design.md) | Technical architecture details     |

## Key Differences from Other Harnesses

| Aspect            | Claude Code / Cursor            | OpenCode                       |
| ----------------- | ------------------------------- | ------------------------------ |
| Integration       | External hooks (stdin/stdout)   | In-process TypeScript plugin   |
| Blocking          | Return JSON `{continue: false}` | Throw Error                    |
| Ask Support       | Native                          | Converted to deny with message |
| Context Injection | `additionalContext` field       | Limited (Phase 2)              |

## Event Support

| Event        | Status    | Description                  |
| ------------ | --------- | ---------------------------- |
| PreToolUse   | Supported | Block tools before execution |
| PostToolUse  | Supported | Validate after execution     |
| SessionStart | Future    | Initialize session context   |
| SessionEnd   | Future    | Cleanup on session end       |

## Tool Name Mapping

OpenCode uses lowercase tool names. Cupcake normalizes them:

| OpenCode | Cupcake Policy |
| -------- | -------------- |
| `bash`   | `Bash`         |
| `edit`   | `Edit`         |
| `write`  | `Write`        |
| `read`   | `Read`         |
| `grep`   | `Grep`         |
| `glob`   | `Glob`         |

## Example Policy

```rego
# METADATA
# scope: package
# custom:
#   routing:
#     required_events: ["PreToolUse"]
#     required_tools: ["Bash"]
package cupcake.policies.opencode.example

import rego.v1

deny contains decision if {
    input.tool_name == "Bash"
    contains(input.tool_input.command, "rm -rf /")

    decision := {
        "rule_id": "DANGEROUS_RM",
        "reason": "Cannot delete root directory",
        "severity": "CRITICAL"
    }
}
```

## Quick Test

```bash
# Initialize project
cupcake init --harness opencode

# Test a deny scenario
echo '{"hook_event_name":"PreToolUse","session_id":"test","cwd":"/tmp","tool":"bash","args":{"command":"git commit --no-verify"}}' | cupcake eval --harness opencode
```

## Support

- **Examples**: `examples/opencode/`
- **Issues**: GitHub Issues
- **Full Docs**: [Cupcake Documentation](../../README.md)
