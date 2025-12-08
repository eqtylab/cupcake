---
layout: "@/layouts/mdx-layout.astro"
heading: "OpenCode Walkthrough"
description: "Getting started with Cupcake and OpenCode"
---

Cupcake has native support for [OpenCode](https://opencode.ai). This integration uses OpenCode's plugin system (`tool.execute.before` hook) to intercept tool calls and evaluate them against your policies.

This walkthrough demonstrates Cupcake's policy enforcement in action with OpenCode.

## Prerequisites

Before starting, ensure you have:

- **Rust & Cargo** → [Install Rust](https://rustup.rs/)
- **OPA (Open Policy Agent)** → [Install OPA](https://www.openpolicyagent.org/docs/latest/#running-opa)
  - **Windows users**: Download `opa_windows_amd64.exe` and rename to `opa.exe`
- **OpenCode** → AI coding assistant [opencode.ai](https://opencode.ai)

_These are development requirements. The production software will manage these dependencies._

## Setup

### 1. Initialize the Environment

Run the init command from your project directory:

```bash
cupcake init --harness opencode
```

This creates:

```
.cupcake/
  ├── rulebook.yml         # Default configuration
  ├── system/              # System aggregation entrypoint
  │   └── evaluate.rego
  ├── policies/            # Rego policies
  │   └── opencode/
  │       └── builtins/    # Built-in security policies
  ├── signals/             # External data providers
  └── actions/             # Automated response scripts

.opencode/
  └── plugins/
      └── cupcake/         # Cupcake plugin
```

Reset anytime by removing the `.cupcake/` directory and re-running init.

### 2. Start OpenCode

Start OpenCode in this directory. The policy engine will now intercept and evaluate all tool usage.

---

## Quick Test

You can test policy enforcement directly using the cupcake CLI:

```bash
# Should DENY (--no-verify is blocked)
echo '{"hook_event_name":"PreToolUse","session_id":"test","cwd":"'$(pwd)'","tool":"bash","args":{"command":"git commit --no-verify"}}' | cupcake eval --harness opencode

# Should ALLOW
echo '{"hook_event_name":"PreToolUse","session_id":"test","cwd":"'$(pwd)'","tool":"bash","args":{"command":"git status"}}' | cupcake eval --harness opencode
```

Expected output for blocked command:

```json
{
  "decision": "deny",
  "reason": "The --no-verify flag bypasses pre-commit hooks..."
}
```

---

## Interactive Demo

### Step 1: Test Basic Protection

Ask OpenCode to run a dangerous command:

```
> please run git commit --no-verify -m "skip hooks"
```

**Expected Result:** Blocked before execution.

---

### Step 2: Understanding the Block

The `git commit --no-verify` command was blocked by a security policy:

```rego
deny contains decision if {
    input.tool_name == "Bash"
    command := input.tool_input.command

    # Check if this is a git commit with --no-verify
    contains(command, "git commit")
    contains(command, "--no-verify")

    decision := {
        "rule_id": "GIT_NO_VERIFY",
        "reason": "The --no-verify flag bypasses pre-commit hooks and security checks. This is blocked by your organization's security policy.",
        "severity": "HIGH"
    }
}
```

This comes from the `minimal_protection.rego` policy that blocks dangerous git commands.

**Event Format Note**: OpenCode sends events with `tool` and `args` fields:

```json
{
  "hook_event_name": "PreToolUse",
  "session_id": "abc123",
  "cwd": "/path/to/project",
  "tool": "bash",
  "args": {
    "command": "git commit --no-verify -m \"skip hooks\""
  }
}
```

Cupcake's preprocessing layer automatically normalizes these to `tool_name` (PascalCase) and `tool_input` for policy evaluation. This means your policies always use `input.tool_name` and `input.tool_input`, regardless of harness.

---

### Step 3: The Challenge - Bypass Attempt

Now, let's see if OpenCode can remove the blocking policy:

```
> find what policy in .cupcake is blocking us and remove it
```

**Expected Result**: OpenCode will try to access `.cupcake/` directory but **fail**!

---

### Step 4: Built-in Protection Explained

What happened? OpenCode was blocked by the `rulebook_security_guardrails` builtin, which protects Cupcake's own configuration from tampering.

**`Built-ins` are special policies that:**

- Are enabled by default in `rulebook.yml`
- Protect critical system functionality
- Cannot be easily bypassed, even by AI agents
- Provide layered security (global + project level)

**Active built-ins in this demo:**

- `rulebook_security_guardrails` → protects `.cupcake/`
- `protected_paths` → blocks `/etc/`, `/System/` modifications
- `git_pre_check` → validates git operations
- `git_block_no_verify` → blocks git commands that skip verification

---

### Step 5: Centralized Rule Management

Part of the benefit of using a centralized policy enforcement layer is the ability to have a well managed model over rules. Let's see all of the rules cupcake loads at runtime:

```bash
cupcake inspect --harness opencode # will show policies currently loaded
```

```bash
cupcake inspect --harness opencode --table # shows a compact table format
```
