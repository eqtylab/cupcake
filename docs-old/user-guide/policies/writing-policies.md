# Writing Policies for Cupcake

## Policy Template

Every policy follows this pattern:

```rego
# METADATA
# scope: package
# custom:
#   routing:
#     required_events: ["PreToolUse"]     # Event types this policy handles
#     required_tools: ["Bash"]            # Tools to monitor (optional)
package cupcake.policies.my_security

import rego.v1

deny contains decision if {
    # Always check the event type first
    input.hook_event_name == "PreToolUse"
    input.tool_name == "Bash"

    # Your security logic
    contains(input.tool_input.command, "rm -rf")

    decision := {
        "reason": "Destructive command blocked",
        "severity": "HIGH",
        "rule_id": "SEC-001"
    }
}
```

## Decision Verbs

| Verb             | Purpose              | Effect                                |
| ---------------- | -------------------- | ------------------------------------- |
| `halt`           | Emergency stop       | Terminates entire session             |
| `deny`           | Block action         | Prevents execution with feedback      |
| `block`          | Block (post-action)  | Provides corrective feedback          |
| `ask`            | Request confirmation | Prompts user before proceeding        |
| `allow_override` | Explicit permission  | Allows with logged reason             |
| `add_context`    | Inject guidance      | Adds context to agent (Claude only)   |

## Automatic Input Preprocessing

Cupcake automatically preprocesses all input before your policies evaluate it. This provides security defenses against common bypass techniques without requiring any action from policy authors.

### What You Get Automatically

#### 1. Resolved File Paths

For all file operations (Edit, Write, Read, etc.), Cupcake adds:
- `resolved_file_path` - The canonical, absolute path to the file
- `original_file_path` - The original path for reference
- `is_symlink` - Boolean flag indicating if the path is a symbolic link

**Example:** Instead of parsing paths yourself, use the preprocessed field:
```rego
# ✅ GOOD - Use the preprocessed canonical path
deny contains decision if {
    contains(input.resolved_file_path, "/sensitive")
    # ...
}

# ❌ AVOID - Don't parse paths manually
deny contains decision if {
    contains(input.tool_input.file_path, "/sensitive")  # May miss symlinks!
    # ...
}
```

#### 2. Normalized Whitespace

All Bash commands have their whitespace automatically normalized:
- Multiple spaces collapsed to single space
- Unicode spaces converted to ASCII
- Tabs/newlines converted to spaces
- Content within quotes preserved exactly

**Example:** Both of these commands will be normalized to the same string:
```bash
"rm   -rf   /tmp"     # Multiple spaces
"rm　-rf　/tmp"       # Unicode spaces
# Both become: "rm -rf /tmp"
```

#### 3. Symlink Detection

When a file path is actually a symbolic link, Cupcake:
- Resolves it to the actual target path in `resolved_file_path`
- Sets `is_symlink: true` flag
- Preserves the original link name in `original_file_path`

**Example:** Detecting symlink bypass attempts:
```rego
# Automatically protected from symlink bypasses
deny contains decision if {
    input.is_symlink
    contains(input.resolved_file_path, ".cupcake")
    decision := {
        "reason": "Symlink to Cupcake directory detected",
        "severity": "HIGH",
        "rule_id": "SYMLINK-001"
    }
}
```

### Special Cases

#### MultiEdit Tool

For `MultiEdit`, each edit in the array gets its own preprocessing:
```rego
deny contains decision if {
    input.tool_name == "MultiEdit"
    some edit in input.tool_input.edits

    # Each edit has its own resolved_file_path
    contains(edit.resolved_file_path, "/protected")

    decision := {
        "reason": "Cannot edit protected file",
        "severity": "HIGH",
        "rule_id": "MULTI-001"
    }
}
```

#### Pattern-Based Tools

Tools like `Glob` and `Grep` use patterns, not file paths, so they don't get path resolution:
```rego
# Glob patterns are NOT resolved (they're patterns, not paths)
input.tool_name == "Glob"
input.tool_input.pattern == "**/*.js"  # This stays as-is
```

## Examples

### Shell Command Protection

```rego
# METADATA
# scope: package
# custom:
#   routing:
#     required_events: ["PreToolUse"]
#     required_tools: ["Bash"]
package cupcake.policies.bash_security

import rego.v1

deny contains decision if {
    input.hook_event_name == "PreToolUse"
    input.tool_name == "Bash"
    contains(input.tool_input.command, "sudo")

    decision := {
        "reason": "Sudo commands require explicit approval",
        "severity": "HIGH",
        "rule_id": "BASH-SEC-001"
    }
}

ask contains decision if {
    input.hook_event_name == "PreToolUse"
    input.tool_name == "Bash"
    regex.match(`curl.*\|.*sh`, input.tool_input.command)

    decision := {
        "reason": "Piping to shell can execute untrusted code. Proceed?",
        "severity": "MEDIUM",
        "rule_id": "BASH-SEC-002"
    }
}
```

### File Protection

```rego
# METADATA
# scope: package
# custom:
#   routing:
#     required_events: ["PreToolUse"]
#     required_tools: ["Write", "Edit"]
package cupcake.policies.file_protection

import rego.v1

deny contains decision if {
    input.hook_event_name == "PreToolUse"
    input.tool_name in ["Write", "Edit"]

    sensitive_paths := [".ssh", ".aws", ".env", "secrets"]
    some path in sensitive_paths
    contains(input.tool_input.file_path, path)

    decision := {
        "reason": concat("", ["Cannot modify files in sensitive directory: ", path]),
        "severity": "HIGH",
        "rule_id": "FILE-001"
    }
}
```

### Git Safety with Signals

```rego
# METADATA
# scope: package
# custom:
#   routing:
#     required_events: ["PreToolUse"]
#     required_tools: ["Bash"]
#     required_signals: ["git_branch"]
package cupcake.policies.git_safety

import rego.v1

deny contains decision if {
    input.hook_event_name == "PreToolUse"
    input.tool_name == "Bash"
    contains(input.tool_input.command, "git commit")
    input.signals.git_branch == "main"

    decision := {
        "reason": "Direct commits to main branch are not allowed",
        "severity": "HIGH",
        "rule_id": "GIT-001"
    }
}
```

### Cursor-Specific Events

```rego
# METADATA
# scope: package
# custom:
#   routing:
#     required_events: ["beforeShellExecution"]
package cupcake.policies.cursor.shell

import rego.v1

deny contains decision if {
    input.hook_event_name == "beforeShellExecution"
    contains(input.command, "rm -rf")

    decision := {
        "reason": "Destructive command blocked",
        "agent_context": "Use 'trash' command or be more specific about what to delete",
        "severity": "HIGH",
        "rule_id": "CURSOR-SHELL-001"
    }
}
```

### Multi-Event Policy

```rego
# METADATA
# scope: package
# custom:
#   routing:
#     required_events: ["PreToolUse", "PostToolUse"]
#     required_tools: ["Bash"]
package cupcake.policies.command_audit

import rego.v1

# Log all commands before execution
add_context contains msg if {
    input.hook_event_name == "PreToolUse"
    input.tool_name == "Bash"
    msg := concat("", ["Executing: ", input.tool_input.command])
}

# Check for failures after execution
deny contains decision if {
    input.hook_event_name == "PostToolUse"
    input.tool_name == "Bash"
    input.tool_response.exit_code != 0
    contains(input.tool_input.command, "deploy")

    decision := {
        "reason": "Deployment command failed - manual intervention required",
        "severity": "HIGH",
        "rule_id": "DEPLOY-001"
    }
}
```

## Input Structure

Your policies receive this input:

```json
{
  "hook_event_name": "PreToolUse",
  "tool_name": "Bash",
  "tool_input": {
    "command": "git status"
  },
  "session_id": "abc-123",
  "cwd": "/home/user/project",
  "signals": {
    "git_branch": "main",
    "git_status": "clean"
  }
}
```

## Decision Object Fields

| Field           | Required | Description                                     |
| --------------- | -------- | ----------------------------------------------- |
| `reason`        | Yes      | User-facing explanation                        |
| `rule_id`       | Yes      | Unique identifier for this rule                |
| `severity`      | Yes      | HIGH, MEDIUM, or LOW                           |
| `agent_context` | No       | Technical guidance for agent (Cursor only)     |
| `question`      | No       | Question to ask user (required for `ask` verb) |

## Testing Your Policies

```bash
# Test a specific policy
echo '{"hook_event_name": "PreToolUse", "tool_name": "Bash", "tool_input": {"command": "sudo rm -rf /"}}' | \
  cupcake eval --harness claude --policy-dir .cupcake/policies

# Test with OPA directly
opa eval -d .cupcake/policies -i event.json "data.cupcake.policies.bash_security.deny"
```

## File Organization

```
.cupcake/
├── policies/
│   ├── claude/           # Claude Code policies
│   │   ├── system/       # System entrypoint (provided)
│   │   ├── git.rego
│   │   └── files.rego
│   └── cursor/           # Cursor policies
│       ├── system/       # System entrypoint (provided)
│       ├── shell.rego
│       └── database.rego
├── rulebook.yml          # Configuration
└── signals/              # Signal scripts
```

## Quick Reference

1. **Declare events** in metadata header
2. **Check event type** as first condition in rule
3. **Return decision object** with reason, severity, and rule_id
4. **Use `concat`** for string formatting (not sprintf)
5. **Access signals** via `input.signals.<name>`

That's it. Drop your `.rego` files in the policies directory and Cupcake handles the rest.