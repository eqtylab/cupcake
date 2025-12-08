---
title: "Quickstart"
description: "Get started with Cupcake Catalog in 5 minutes"
---

# Quickstart

This guide will help you install your first rulebook from the Cupcake Catalog.

## Prerequisites

- [Cupcake](https://cupcake.eqtylab.io/getting-started/installation/) installed (v0.3.0+)
- An initialized Cupcake project (`cupcake init`)

## Step 1: Search the Catalog

Browse available rulebooks:

```bash
cupcake catalog search
```

Filter by harness type:

```bash
cupcake catalog search --harness claude
```

## Step 2: View Rulebook Details

Get detailed information about a rulebook:

```bash
cupcake catalog show security-hardened
```

This shows:

- Available versions
- Supported harnesses
- Description and keywords
- Installation instructions

## Step 3: Install a Rulebook

Install the latest version:

```bash
cupcake catalog install security-hardened
```

Or install a specific version:

```bash
cupcake catalog install security-hardened@1.0.0
```

Or use semver ranges:

```bash
# Compatible version (^1.0 = >=1.0.0 <2.0.0)
cupcake catalog install security-hardened@^1.0

# Patch-level updates (~1.0 = >=1.0.0 <1.1.0)
cupcake catalog install security-hardened@~1.0
```

## Step 4: Verify Installation

List installed rulebooks:

```bash
cupcake catalog list
```

The rulebook is now installed in `.cupcake/catalog/security-hardened/` and will be automatically evaluated when you run Cupcake.

## Step 5: Test It Out

Try a command that the security-hardened rulebook should block:

```bash
# Create a test event with a dangerous command
cat > test-event.json << 'EOF'
{
  "hook_event_name": "PreToolUse",
  "tool_name": "Bash",
  "tool_input": { "command": "rm -rf /" },
  "session_id": "test",
  "cwd": "/tmp",
  "transcript_path": "/tmp/transcript.md"
}
EOF

# This should be blocked by the dangerous_commands policy
cupcake eval --harness claude < test-event.json
```

Expected output (denied):

```json
{
  "hookSpecificOutput": {
    "hookEventName": "PreToolUse",
    "permissionDecision": "deny",
    "userFacingMessage": "Blocked: dangerous command detected"
  }
}
```

## Next Steps

- [Learn how catalog policies integrate](using-rulebooks.md) with your project
- [View CLI reference](../reference/cli.md) for all catalog commands
- [Create your own rulebook](../authoring/index.md) to share with the community
