# Cupcake Init Command

## Overview

The `cupcake init` command creates a new Cupcake project structure in the current directory. It sets up the minimal required files and directories following the `.cupcake/` convention, and can optionally configure integration with agent harnesses like Claude Code.

## Usage

```bash
# Basic initialization
cupcake init

# Initialize with Claude Code integration
cupcake init --harness claude

# Initialize with specific builtins enabled
cupcake init --builtins git_pre_check,protected_paths

# Initialize with both harness and builtins
cupcake init --harness claude --builtins git_pre_check,protected_paths

# Initialize global configuration
cupcake init --global

# Initialize global with Claude Code integration
cupcake init --global --harness claude

# Initialize global with specific builtins
cupcake init --global --builtins system_protection
```

## What It Creates

Running `cupcake init` creates the following structure:

```
.cupcake/
├── policies/
│   ├── system/
│   │   └── evaluate.rego    # Required aggregation policy
│   └── example.rego         # Placeholder policy (never fires)
├── signals/                 # Empty directory for signal scripts
└── actions/                 # Empty directory for action scripts
```

### Files Created

#### `system/evaluate.rego`

The mandatory system aggregation policy that collects decision verbs from all policies. This file is required for the Cupcake engine to function and should not be modified unless you understand the aggregation system.

#### `example.rego`

A minimal placeholder policy that:

- Prevents OPA compilation issues
- Provides a template structure for writing real policies
- Contains a rule that never fires (checks for an impossible command)
- Can be safely deleted once you add your own policies

## Behavior

### First Run

When run in a directory without `.cupcake/`:

```bash
$ cupcake init
Initialized Cupcake project in .cupcake/
   Add your policies to .cupcake/policies/
   Add signal scripts to .cupcake/signals/
   Add action scripts to .cupcake/actions/
```

### Subsequent Runs

If `.cupcake/` already exists, the command exits cleanly without modifying anything:

```bash
$ cupcake init
Cupcake project already initialized (.cupcake/ exists)
```

## Next Steps

After initialization:

1. **Add Your Policies**: Create new `.rego` files in `.cupcake/policies/` with your security rules
2. **Add Signals** (optional): Place executable scripts in `.cupcake/signals/` to gather runtime information
3. **Add Actions** (optional): Place executable scripts in `.cupcake/actions/` to respond to policy decisions
4. **Test Your Setup**: Use `cupcake verify --policy-dir .` to verify your configuration

## Example Policy

Replace the example policy with something like:

```rego
# METADATA
# scope: package
# title: File Safety Policy
# custom:
#   routing:
#     required_events: ["PreToolUse"]
#     required_tools: ["Write", "Edit"]
package cupcake.policies.file_safety

import rego.v1

deny contains decision if {
    contains(input.tool_input.file_path, "/etc/")
    decision := {
        "reason": "System files cannot be modified",
        "severity": "HIGH",
        "rule_id": "FILE-001"
    }
}
```

## Builtin Policies

The `--builtins` flag allows you to enable specific builtin policies during initialization. Instead of manually uncommenting sections in `rulebook.yml`, you can specify which builtins to enable from the command line.

### Available Builtins

**Project-level builtins** (for use with `cupcake init`):

- `claude_code_always_inject_on_prompt` - Adds context to every user prompt
- `git_pre_check` - Validates before git operations
- `post_edit_check` - Runs checks after file edits
- `rulebook_security_guardrails` - Protects `.cupcake/` from modification
- `protected_paths` - User-defined read-only paths
- `git_block_no_verify` - Prevents bypassing commit hooks
- `claude_code_enforce_full_file_read` - Requires complete file reads under size limit

**Global builtins** (for use with `cupcake init --global`):

- `system_protection` - Blocks critical system path modifications
- `sensitive_data_protection` - Blocks credential file reads
- `cupcake_exec_protection` - Prevents direct cupcake binary execution

### Usage Examples

```bash
# Enable a single builtin
cupcake init --builtins git_pre_check

# Enable multiple builtins
cupcake init --builtins git_pre_check,protected_paths

# Combine with harness integration
cupcake init --harness claude --builtins git_pre_check,protected_paths

# Global init with custom builtins (instead of all three defaults)
cupcake init --global --builtins system_protection
```

### Default Configurations

When you enable certain builtins, Cupcake provides sensible defaults:

- **git_pre_check**: Adds a basic validation command
- **protected_paths**: Includes `/etc/`, `/System/`, and `~/.ssh/` by default
- **post_edit_check**: Adds Python and Rust syntax checks for `.py` and `.rs` files

You can customize these defaults by editing `rulebook.yml` after initialization.

### Error Handling

If you specify an invalid builtin name, Cupcake will show an error with all valid options:

```bash
$ cupcake init --builtins invalid_name
Error: Unknown builtin 'invalid_name'. Valid project builtins are:
claude_code_always_inject_on_prompt, git_pre_check, post_edit_check, ...
```

## Harness Integration

The `--harness` flag automatically configures integration with agent harnesses:

### Claude Code Integration

When you use `--harness claude`, Cupcake automatically:

1. Creates or updates `.claude/settings.json` with appropriate hooks
2. Configures four key hook events:

   - **PreToolUse**: Evaluates all tool uses before execution
   - **PostToolUse**: Validates file modifications (Edit/Write operations)
   - **UserPromptSubmit**: Enables prompt validation and context injection
   - **SessionStart**: Loads project context at session start

3. Uses smart path handling:
   - Project configs use `$CLAUDE_PROJECT_DIR/.cupcake` for portability
   - Global configs use absolute paths

### Example Claude Code Configuration

Running `cupcake init --harness claude` generates:

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "*",
        "hooks": [
          {
            "type": "command",
            "command": "cupcake eval --policy-dir $CLAUDE_PROJECT_DIR/.cupcake"
          }
        ]
      }
    ],
    "PostToolUse": [
      {
        "matcher": "Edit|MultiEdit|Write",
        "hooks": [
          {
            "type": "command",
            "command": "cupcake eval --policy-dir $CLAUDE_PROJECT_DIR/.cupcake"
          }
        ]
      }
    ],
    "UserPromptSubmit": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "cupcake eval --policy-dir $CLAUDE_PROJECT_DIR/.cupcake"
          }
        ]
      }
    ],
    "SessionStart": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "cupcake eval --policy-dir $CLAUDE_PROJECT_DIR/.cupcake"
          }
        ]
      }
    ]
  }
}
```

### Merging with Existing Settings

If `.claude/settings.json` already exists, Cupcake will:

- Preserve all existing settings (env vars, model preferences, etc.)
- Merge new hooks without creating duplicates
- Show a warning message about the merge operation

### Manual Configuration

If automatic configuration fails, Cupcake provides manual instructions. You can also manually configure Claude Code by creating `.claude/settings.json` with the structure shown above

## Design Principles

- **Zero Configuration**: Works immediately after init with sensible defaults
- **No Real Impact**: The example policy never fires, ensuring no unexpected blocks
- **Convention Over Configuration**: Uses `.cupcake/` directory structure
- **Simple and Focused**: Does one thing - creates the minimal required structure
