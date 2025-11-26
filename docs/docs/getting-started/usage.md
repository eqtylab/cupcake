---
layout: "@/layouts/mdx-layout.astro"
heading: "Usage"
description: "Get up and running with Cupcake"
---

import HarnessSelector from '@/components/harness-selector.astro';

## Getting Started

After [installation](/getting-started/installation), you're ready to set up Cupcake for your project. The first step is choosing which AI coding agent (harness) you're using.

### Select Your Harness

<HarnessSelector>
  <div class="tab-content active" data-harness="claude">

## Initialize for Claude Code

Claude Code supports both **project-level** and **global** configurations.

### Project Setup (Recommended)

Navigate to your project directory and initialize Cupcake:

```bash
cupcake init --harness claude
```

This creates a `.cupcake/` directory in your project with:
- `policies/` - Your policy files
- `rulebook.yml` - Configuration file
- `signals/` - External data providers
- `actions/` - Automated responses

And configures Claude Code hooks at `.claude/settings.json`.

### Global Setup

For organization-wide policies that apply to all projects:

```bash
cupcake init --global --harness claude
```

This creates configuration at `~/.config/cupcake/` (or equivalent on your platform) and sets up hooks at `~/.claude/settings.json`.

  </div>

  <div class="tab-content" data-harness="cursor">

## Initialize for Cursor

**Important**: Cursor only supports **global** hooks (not project-level).

### Project Setup with Global Hooks

Navigate to your project directory and initialize Cupcake:

```bash
cupcake init --harness cursor
```

This creates a `.cupcake/` directory in your project with policies and configuration, and sets up **global hooks** at `~/.cursor/hooks.json` that use relative paths (`.cupcake`).

When you open this project in Cursor, the hooks will automatically find the project's `.cupcake/` directory.

### Global Setup

For organization-wide policies:

```bash
cupcake init --global --harness cursor
```

This creates configuration at `~/.config/cupcake/` and sets up global hooks at `~/.cursor/hooks.json` with absolute paths.

  </div>
</HarnessSelector>

## Enable Built-in Policies (Optional)

Cupcake includes pre-built security policies you can enable during initialization:

<div class="harness-tabs-builtins">
  <div class="tab-content active" data-harness="claude">

```bash
# Enable specific builtins
cupcake init --harness claude --builtins git_pre_check,git_block_no_verify

# Global with builtins
cupcake init --global --harness claude --builtins system_protection,sensitive_data_protection
```

  </div>

  <div class="tab-content" data-harness="cursor">

```bash
# Enable specific builtins
cupcake init --harness cursor --builtins git_pre_check,git_block_no_verify

# Global with builtins
cupcake init --global --harness cursor --builtins system_protection,sensitive_data_protection
```

  </div>
</div>

Available builtins include:
- `git_pre_check` - Run checks before git operations
- `git_block_no_verify` - Prevent `--no-verify` flag usage
- `system_protection` - Protect system directories
- `sensitive_data_protection` - Block access to sensitive files
- `protected_paths` - Make specific paths read-only

See the [Built-in Configuration Reference](/reference/builtin-config) for complete details.

## Verify Installation

Test that Cupcake is working correctly:

<div class="harness-tabs-verify">
  <div class="tab-content active" data-harness="claude">

### 1. Create a test event

Save this to `test-event.json`:

```json
{
  "hook_event_name": "PreToolUse",
  "tool_name": "Bash",
  "tool_input": {
    "command": "echo 'Hello from Cupcake!'"
  },
  "session_id": "test-session",
  "cwd": "/tmp",
  "transcript_path": "/tmp/transcript.md"
}
```

### 2. Evaluate the event

```bash
cupcake eval --harness claude --policy-dir .cupcake/policies < test-event.json
```

### 3. Expected output

You should see a JSON response with an `Allow` decision:

```json
{
  "Allow": {
    "context": []
  }
}
```

  </div>

  <div class="tab-content" data-harness="cursor">

### 1. Create a test event

Save this to `test-event.json`:

```json
{
  "eventType": "beforeShellExecution",
  "command": "echo 'Hello from Cupcake!'",
  "workingDirectory": "/tmp"
}
```

### 2. Evaluate the event

```bash
cupcake eval --harness cursor --policy-dir .cupcake/policies < test-event.json
```

### 3. Expected output

You should see a JSON response indicating the command is allowed:

```json
{
  "continue": true
}
```

  </div>
</div>

## Next Steps

From here, you're ready to either configure your own Rego policies or use the built-ins:

### Option 1: Use Built-in Policies

Cupcake includes battle-tested security policies ready to use. Edit your `.cupcake/rulebook.yml` to enable and configure them:

```yaml
builtins:
  git_pre_check:
    enabled: true
    checks:
      - command: "npm test"
        message: "Tests must pass before commit"

  protected_paths:
    enabled: true
    paths:
      - "/etc/"
      - "~/.ssh/"
```

See the **[Built-in Configuration Reference](/reference/builtin-config)** for all available builtins and their options.

### Option 2: Write Custom Policies

Create your own policies in `.cupcake/policies/<harness>/` using OPA Rego:

```rego
package cupcake.policies.example

import rego.v1

# METADATA
# scope: package
# custom:
#   routing:
#     required_events: ["PreToolUse"]
#     required_tools: ["Bash"]

deny contains decision if {
    input.tool_input.command contains "rm -rf"

    decision := {
        "rule_id": "SAFETY-001",
        "reason": "Dangerous command blocked",
        "severity": "HIGH"
    }
}
```
