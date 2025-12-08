# OpenCode Quickstart Guide

Get Cupcake policy enforcement running with OpenCode in 5 minutes.

## Prerequisites

- [OpenCode](https://opencode.ai) installed
- Cupcake CLI built or installed

## Step 1: Build Cupcake (if not already done)

```bash
cd /path/to/cupcake
cargo build --release

# Add to PATH or use directly
export PATH="$PATH:$(pwd)/target/release"
```

## Step 2: Initialize Your Project

```bash
cd /path/to/your/project

# Initialize Cupcake for OpenCode
cupcake init --harness opencode
```

This creates the following structure:

```
.cupcake/
├── rulebook.yml              # Configuration
├── system/
│   └── evaluate.rego         # System entrypoint (auto-generated)
├── helpers/
│   └── commands.rego         # Shared helper functions
└── policies/
    └── opencode/
        └── builtins/         # Built-in policies
            └── *.rego
```

## Step 3: Add a Policy

Create a simple policy to block dangerous git commands:

```bash
cat > .cupcake/policies/opencode/git_safety.rego << 'EOF'
# METADATA
# scope: package
# title: Git Safety Policy
# custom:
#   routing:
#     required_events: ["PreToolUse"]
#     required_tools: ["Bash"]
package cupcake.policies.opencode.git_safety

import rego.v1

# Block git commit with --no-verify
deny contains decision if {
    input.tool_name == "Bash"
    command := input.tool_input.command
    contains(command, "git commit")
    contains(command, "--no-verify")

    decision := {
        "rule_id": "GIT_NO_VERIFY",
        "reason": "The --no-verify flag bypasses pre-commit hooks. Remove it to proceed.",
        "severity": "HIGH"
    }
}

# Block force push
deny contains decision if {
    input.tool_name == "Bash"
    command := input.tool_input.command
    contains(command, "git push")
    contains(command, "--force")
    not contains(command, "--force-with-lease")

    decision := {
        "rule_id": "GIT_FORCE_PUSH",
        "reason": "Force pushing can overwrite remote history. Use --force-with-lease instead.",
        "severity": "HIGH"
    }
}
EOF
```

## Step 4: Test the CLI

Verify policies work by sending test events:

```bash
# Test: Should be DENIED (--no-verify)
echo '{
  "hook_event_name": "PreToolUse",
  "session_id": "test",
  "cwd": "'$(pwd)'",
  "tool": "bash",
  "args": {"command": "git commit --no-verify -m test"}
}' | cupcake eval --harness opencode

# Expected: {"decision":"deny","reason":"The --no-verify flag bypasses..."}

# Test: Should be ALLOWED
echo '{
  "hook_event_name": "PreToolUse",
  "session_id": "test",
  "cwd": "'$(pwd)'",
  "tool": "bash",
  "args": {"command": "git status"}
}' | cupcake eval --harness opencode

# Expected: {"decision":"allow"}
```

## Step 5: Install the OpenCode Plugin

Install the pre-built plugin that connects OpenCode to Cupcake:

```bash
# Build the plugin (if not already built)
cd /path/to/cupcake/cupcake-plugins/opencode
bun install && bun run build  # or: npm install && npm run build

# Install to your project - just copy a single file!
cd /path/to/your/project
mkdir -p .opencode/plugin
cp /path/to/cupcake/cupcake-plugins/opencode/dist/cupcake.js .opencode/plugin/
```

## Step 6: Run OpenCode

Start OpenCode and test policy enforcement:

```bash
opencode
```

Try these prompts to test:

- `"run git commit --no-verify -m test"` - Should be **blocked**
- `"run git push --force origin main"` - Should be **blocked**
- `"run git status"` - Should be **allowed**

## What's Happening

```
OpenCode                    Plugin                      Cupcake
   │                          │                            │
   │ tool.execute.before      │                            │
   ├─────────────────────────>│                            │
   │                          │ cupcake eval --harness     │
   │                          │     opencode < event.json  │
   │                          ├───────────────────────────>│
   │                          │                            │ Evaluate policies
   │                          │                            │ in WASM sandbox
   │                          │    {"decision":"deny",...} │
   │                          │<───────────────────────────┤
   │        throw Error       │                            │
   │<─────────────────────────┤                            │
   │ Tool blocked!            │                            │
```

## Next Steps

- **Add more policies**: See `examples/opencode/` for examples
- **Configure builtins**: Edit `.cupcake/rulebook.yml` to enable built-in policies
- **Write custom policies**: See [Policy Authoring Guide](../../user-guide/policies/authoring.md)

## Troubleshooting

### "cupcake: command not found"

Add cupcake to your PATH:

```bash
export PATH="$PATH:/path/to/cupcake/target/release"
```

Or specify the full path in plugin config (`.cupcake/opencode.json`):

```json
{
  "cupcakePath": "/full/path/to/cupcake"
}
```

### Policies not being evaluated

Check that:

1. `.cupcake/system/evaluate.rego` file exists (at root level)
2. `.cupcake/policies/opencode/` directory exists
3. Your policy has correct routing metadata

Debug with:

```bash
cupcake eval --harness opencode --log-level debug < test_event.json
```

### Plugin not loading

Verify plugin is installed:

```bash
ls -la .opencode/plugin/cupcake.js
# Should see the cupcake.js file
```

## Example Policies

Copy example policies from the Cupcake repo:

```bash
cp /path/to/cupcake/examples/opencode/0_Welcome/*.rego .cupcake/policies/opencode/
```

Available examples:

- `minimal_protection.rego` - Block dangerous git commands
- `git_workflow.rego` - Enforce git best practices
- `file_protection.rego` - Protect sensitive files
