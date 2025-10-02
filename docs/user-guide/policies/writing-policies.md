# Writing Policies for Cupcake

## Quick Start

Cupcake policies are simple, declarative rules that focus on your business logic. The engine handles all the complexity of routing, aggregation, and response formatting.

```rego
# METADATA
# scope: package
# title: My Security Policy
# custom:
#   routing:
#     required_events: ["PreToolUse"]
#     required_tools: ["Bash"]
package cupcake.policies.my_security

import rego.v1

# Your business logic - that's it!
deny contains decision if {
    contains(input.tool_input.command, "rm -rf")

    decision := {
        "reason": "Destructive command blocked",
        "severity": "HIGH",
        "rule_id": "SEC-001"
    }
}
```

## Core Concepts

### 1. Metadata-Driven Routing

Instead of writing routing logic in your policies, you declare what events and tools you care about in OPA metadata:

```yaml
# METADATA
# scope: package
# custom:
#   routing:
#     required_events: ["PreToolUse"]     # Which Claude Code events to handle
#     required_tools: ["Bash", "Shell"]   # Which tools to monitor (optional)
#     required_signals: ["git_status"]    # External context needed (optional)
```

**The engine guarantees**: If your policy is evaluating, the event and tool already match your requirements. You don't need to check them again.

### 2. Decision Verbs

Cupcake uses modern Rego v1.0 syntax with decision "verbs" that express your intent clearly:

| Verb | Purpose | Claude Code Behavior |
|------|---------|---------------------|
| `halt` | Emergency stop | Terminates entire session |
| `deny` | Block action | Prevents tool execution with feedback |
| `block` | Block (post-action) | Provides corrective feedback |
| `ask` | Request confirmation | Prompts user before proceeding |
| `allow_override` | Explicit permission | Allows with logged reason |
| `add_context` | Inject information | Adds context to Claude's awareness |

### 3. Trust-Based Evaluation

Your policies focus purely on business logic. The engine handles:
- **Routing**: Only relevant policies execute (O(1) lookup)
- **Aggregation**: All decisions collected automatically
- **Prioritization**: Halt > Deny > Ask > Allow (enforced by engine)
- **API Mapping**: Correct Claude Code JSON responses

## Writing Policies

### Basic Security Policy

```rego
# METADATA
# scope: package
# title: Bash Command Security
# authors: ["Security Team"]
# custom:
#   severity: HIGH
#   id: BASH-SEC
#   routing:
#     required_events: ["PreToolUse"]
#     required_tools: ["Bash"]
package cupcake.policies.bash_security

import rego.v1

# Block dangerous commands
deny contains decision if {
    contains(input.tool_input.command, "sudo")

    decision := {
        "reason": "Sudo commands require explicit approval",
        "severity": "HIGH",
        "rule_id": "BASH-SEC-001"
    }
}

# Warn about risky patterns
ask contains decision if {
    regex.match(`curl.*\|.*sh`, input.tool_input.command)
    
    decision := {
        "reason": "Piping to shell can execute untrusted code. Proceed?",
        "severity": "MEDIUM",
        "rule_id": "BASH-SEC-002"
    }
}

# Add helpful context
add_context contains "⚠️ Production environment detected" if {
    contains(input.cwd, "/prod")
}
```

### Multi-Tool Policy

```rego
# METADATA
# scope: package
# custom:
#   routing:
#     required_events: ["PreToolUse"]
#     required_tools: ["Write", "Edit", "MultiEdit"]
package cupcake.policies.file_protection

import rego.v1

# Protect sensitive directories
deny contains decision if {
    # No need to check event or tool - engine guarantees it's a file operation
    sensitive_paths := [".ssh", ".aws", ".env", "secrets"]
    some path in sensitive_paths
    contains(input.tool_input.file_path, path)
    
    # Note: sprintf doesn't work in WASM, use concat instead
    decision := {
        "reason": concat("", ["Cannot modify files in sensitive directory: ", path]),
        "severity": "HIGH",
        "rule_id": "FILE-001"
    }
}
```

### Using Signals (External Context)

```rego
# METADATA
# scope: package
# custom:
#   routing:
#     required_events: ["PreToolUse"]
#     required_tools: ["Bash"]
#     required_signals: ["git_branch", "git_status"]
package cupcake.policies.git_safety

import rego.v1

# Prevent commits on main branch
deny contains decision if {
    contains(input.tool_input.command, "git commit")
    input.signals.git_branch == "main"
    
    decision := {
        "reason": "Direct commits to main branch are not allowed",
        "severity": "HIGH",
        "rule_id": "GIT-001"
    }
}

# Warn about uncommitted changes
add_context contains warning if {
    contains(input.tool_input.command, "git checkout")
    input.signals.git_status.has_changes
    
    warning := "Warning: You have uncommitted changes that will be lost"
}
```

## What Cupcake Provides Automatically

### 1. Automatic Policy Discovery

Drop a `.rego` file in the policies directory - Cupcake automatically:
- Discovers and loads it
- Parses metadata for routing
- Includes it in the aggregation
- No registration or configuration needed

### 2. System Aggregation Policy

Cupcake provides `cupcake.system.evaluate` that automatically:
```rego
# You never write this - Cupcake provides it
evaluate := {
    "halts": [h | h := data.cupcake.policies..halt[_]],
    "denials": [d | d := data.cupcake.policies..deny[_]],
    "asks": [a | a := data.cupcake.policies..ask[_]],
    # ... all decision verbs
}
```

This uses OPA's `walk()` function to recursively find ALL decisions across ALL policies.

### 3. Input Structure

Cupcake provides a consistent input structure:

```json
{
  "hook_event_name": "PreToolUse",
  "tool_name": "Bash",
  "tool_input": {
    "command": "rm -rf /tmp/cache"
  },
  "session_id": "abc-123",
  "cwd": "/home/user/project",
  "signals": {
    // Your requested signals appear here
  }
}
```

### 4. Decision Prioritization

Cupcake automatically enforces priority (you don't implement this):
1. **Halt** - Stops everything immediately
2. **Deny/Block** - Prevents or corrects action
3. **Ask** - Requests user confirmation
4. **Allow** - Proceeds (with optional context)

## Best Practices

### DO

- **Trust the routing** - Don't re-check event types or tool names
- **Use decision verbs** - `deny contains`, `halt contains`, etc.
- **Provide clear reasons** - Users and Claude need to understand why
- **Use severity levels** - HIGH, MEDIUM, LOW for proper aggregation
- **Include rule IDs** - For debugging and audit trails

### DON'T

- **Don't check routing conditions** - The engine already did this
- **Don't write aggregation logic** - Cupcake handles this automatically
- **Don't worry about priority** - The synthesis layer handles conflicts
- **Don't format responses** - Cupcake generates Claude Code JSON

## Testing Policies

Test policies directly with OPA:

```bash
# Test a specific decision verb
opa eval -d examples/policies -i input.json "data.cupcake.policies.bash_security.deny"

# Test the full aggregation
opa eval -d examples/policies -i input.json "data.cupcake.system.evaluate"
```

Test with Cupcake:

```bash
# Test specific scenario
echo '{"hook_event_name": "PreToolUse", "tool_name": "Bash", "tool_input": {"command": "sudo rm -rf /"}}' | cupcake eval --policy-dir examples/policies
```

## Migration from Old Format

If you have policies using the old `selector := {}` format:

**Old (deprecated):**
```rego
selector := {
    "event": "PreToolUse",
    "tools": ["Bash"]
}

deny if {
    input.event.tool_name == "Bash"
    contains(input.event.tool_input.command, "sudo")
}
```

**New (current):**
```rego
# METADATA
# custom:
#   routing:
#     required_events: ["PreToolUse"]
#     required_tools: ["Bash"]

deny contains decision if {
    # No need to check tool_name - routing guaranteed it
    contains(input.tool_input.command, "sudo")
    
    decision := {
        "reason": "Sudo requires approval",
        "severity": "HIGH",
        "rule_id": "SUDO-001"
    }
}
```

## Advanced Topics

### Dynamic Policy Loading

Policies can be loaded from external sources at runtime. The engine will automatically recompile when policies change.

### Performance Optimization

- Policies are compiled to WASM for near-native performance
- Routing provides O(1) lookup - only relevant policies execute
- Signals are fetched in parallel and cached

### Custom Signals

Implement custom signals by adding them to your guidebook.yml:

```yaml
signals:
  git_branch:
    command: "git branch --show-current"
  database_status:
    script: "./scripts/check_db.sh"
```

## Summary

Cupcake policies are simple by design. You write the business logic, Cupcake handles everything else:

1. Declare what you care about (metadata)
2. Write your rules (decision verbs)
3. Drop in the policies directory
4. Cupcake does the rest

The engine is intelligent so your policies don't have to be.