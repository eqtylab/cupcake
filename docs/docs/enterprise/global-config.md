# Global Config

Global configuration enables organization-wide policy enforcement across all projects on a machine.

## Location

| Platform | Path                                     |
| -------- | ---------------------------------------- |
| Linux    | `~/.config/cupcake/`                     |
| macOS    | `~/Library/Application Support/cupcake/` |
| Windows  | `%APPDATA%\cupcake\`                     |

## Directory Structure

```
cupcake/
├── rulebook.yml
├── system/                    # Shared system entrypoint
│   └── evaluate.rego
├── policies/
│   ├── claude/
│   │   └── builtins/
│   └── cursor/
│       └── builtins/
├── signals/
└── actions/
```

## Evaluation Order

Cupcake uses two-phase evaluation:

1. **Global policies run first**
2. **If global returns Halt/Deny/Block → project policies never run**
3. **If global allows → project policies run**

Global policies cannot be overridden by project configuration.

## CLI Override

```bash
cupcake eval --global-config /path/to/config
```

Path must be absolute.

## Writing Global Policies

Same as project policies. See [Policies](../reference/policies/index.md).

Global policies use namespace `cupcake.global.policies.*` instead of `cupcake.policies.*`.
