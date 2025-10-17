# Quick Start Guide

Get Cupcake running with your AI coding agent by choosing your platform below.

## Choose Your Platform

Cupcake integrates with two AI coding agents:

### 🤖 Claude Code (Anthropic)
- **Best for**: Command-line workflows, integrated development
- **Integration**: Claude CLI with hooks system
- **Events**: PreToolUse, PostToolUse, UserPromptSubmit, and more
- **Context injection**: Full support for adding guidance to agent context

👉 [**Get Started with Claude Code**](./harnesses/claude-code.md)

### 🎯 Cursor (Cursor.com)
- **Best for**: VS Code-style editor experience
- **Integration**: Global hooks at `~/.cursor/hooks.json`
- **Events**: beforeShellExecution, afterFileEdit, beforeReadFile, and more
- **Agent feedback**: Separate messages for users and AI agent

👉 [**Get Started with Cursor**](./harnesses/cursor.md)

---

## Interactive Walkthrough Examples

Want a guided experience? Check out our interactive examples:

### Claude Code Walkthrough
```bash
cd examples/claude-code/0_Welcome
./setup.sh
```

**What you'll learn:**
- Policy evaluation flow
- Command blocking in action
- File protection policies
- Git workflow enforcement
- Debug file analysis

📖 [Claude Code Example README](../../examples/claude-code/0_Welcome/README.md)

### Cursor Walkthrough
```bash
cd examples/cursor/0_Welcome
./setup.sh
```

**What you'll learn:**
- Cursor hook configuration
- Shell command protection
- MCP tool control
- File read protection
- Prompt filtering
- Debug file analysis

📖 [Cursor Example README](../../examples/cursor/0_Welcome/README.md)

---

## Installation

Before running examples, install Cupcake:

### From Source (Recommended for Development)
```bash
git clone https://github.com/eqtylab/cupcake-rego.git
cd cupcake-rego/cupcake-rewrite
cargo build --release

# Add to PATH
export PATH="$(pwd)/target/release:$PATH"
```

### From Releases
```bash
# Download latest binary
curl -LO https://github.com/eqtylab/cupcake-rego/releases/latest/download/cupcake-$(uname -s)-$(uname -m)

# Make executable
chmod +x cupcake-*
mv cupcake-* /usr/local/bin/cupcake
```

### Verify Installation
```bash
cupcake --version
```

---

## Quick Commands Reference

Once you've chosen your platform and completed setup:

### Verify Configuration
```bash
cupcake verify --policy-dir .cupcake
```

### Inspect Active Policies
```bash
cupcake inspect --policy-dir .cupcake --table
```

### Test a Policy
```bash
# Create test event
echo '{"hook_event_name":"PreToolUse","tool_name":"Bash","tool_input":{"command":"rm -rf /"}}' > test.json

# Claude Code evaluation
cupcake eval --harness claude < test.json

# Cursor evaluation
cupcake eval --harness cursor < test.json
```

### Enable Debug Logging
```bash
cupcake eval --harness claude --debug-files --log-level info
```

---

## Next Steps

**Learn the Fundamentals:**
- [Writing Policies](./policies/writing-policies.md) - Create custom rules in Rego
- [Built-in Policies Reference](./policies/builtin-policies-reference.md) - 11 pre-built policies
- [Harness Comparison](./harnesses/harness-comparison.md) - Claude Code vs Cursor

**Advanced Configuration:**
- [Signals](./configuration/signals.md) - Gather dynamic data for policies
- [Actions](./configuration/actions.md) - Execute commands on policy decisions
- [Trust System](./cli/trust.md) - Script integrity verification

**Architecture:**
- [Harness Model](./architecture/harness-model.md) - How harness integration works
- [Hybrid Model](./architecture/hybrid-model.md) - Rego + Rust architecture

---

## Common Issues

### "Command not found: cupcake"
Add to PATH and restart terminal:
```bash
echo 'export PATH="$HOME/.cupcake/bin:$PATH"' >> ~/.zshrc
source ~/.zshrc
```

### Policy Not Firing
Check if policy is enabled:
```bash
cupcake inspect --policy-dir .cupcake
```

Enable debug mode to see evaluation:
```bash
cupcake eval --harness claude --log-level debug
```

### Claude Code / Cursor Not Responding
**Claude Code:**
```bash
# Verify hooks configured
cat .claude/settings.json

# Restart Claude Code after config changes
```

**Cursor:**
```bash
# Check global hooks
cat ~/.cursor/hooks.json

# Restart Cursor completely (Cmd+Q, reopen)
```

For detailed troubleshooting, see your platform's integration guide:
- [Claude Code Troubleshooting](./harnesses/claude-code.md#troubleshooting)
- [Cursor Troubleshooting](./harnesses/cursor.md#troubleshooting)

---

## Getting Help

- 📖 [Full Documentation](../README.md)
- 💬 [GitHub Issues](https://github.com/eqtylab/cupcake-rego/issues)
- 🔍 [Examples Directory](../../examples/)

**You're ready to start building intelligent policies for your AI agent!** 🎉
