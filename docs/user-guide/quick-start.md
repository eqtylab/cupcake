# Quick Start Guide - 5 Minutes to Your First Policy

Get Cupcake running with your AI agent in just 5 minutes.

## 1. Install Cupcake (1 minute)

### macOS/Linux

```bash
curl -sSL https://raw.githubusercontent.com/eqtylab/cupcake/v0.1.6/scripts/install.sh | bash
```

### Verify Installation

```bash
cupcake --version
```

## 2. Initialize Your Project (1 minute)

### Basic Setup

```bash
# In your project directory
cupcake init --harness claude --builtins git_pre_check,protected_paths
```

This command:

- Creates `.cupcake/` directory structure
- Configures Claude Code integration automatically
- Enables two useful builtin policies

### What You Get

```
.cupcake/
├── rulebook.yml       # Configuration with enabled builtins
├── policies/           # Your custom policies go here
├── signals/            # External data scripts
└── actions/            # Response scripts
```

## 3. Test Your Setup (1 minute)

```bash
# Verify your configuration
cupcake verify --policy-dir .cupcake

# See what policies are active
cupcake inspect --policy-dir .cupcake
```

You should see:

- git_pre_check policy enabled
- protected_paths policy enabled
- Claude Code hooks configured

## 4. Try It Out (2 minutes)

With Claude Code:

1. Open your project in Claude
2. Ask Claude to edit a protected file:
   ```
   "Edit the file /etc/hosts"
   ```
3. Cupcake will block this with: "System path modification blocked by policy"

Test git protection:

1. Ask Claude to commit without tests:
   ```
   "Commit these changes with message 'quick fix'"
   ```
2. Cupcake will run validation before allowing the commit

## 5. Customize Your Policies (Optional)

### Enable More Builtins

Edit `.cupcake/rulebook.yml`:

```yaml
builtins:
  global_file_lock:
    enabled: true # Prevent ALL file modifications

  post_edit_check:
    by_extension:
      "py":
        command: "python -m py_compile"
        message: "Python syntax error"
```

### Write a Custom Policy

Create `.cupcake/policies/no-sudo.rego`:

```rego
# METADATA
# scope: package
# title: No Sudo Commands Policy
# custom:
#   routing:
#     required_events: ["PreToolUse"]
#     required_tools: ["Bash"]
package cupcake.policies.no_sudo

import rego.v1

deny contains decision if {
    contains(input.tool_input.command, "sudo")
    decision := {
        "reason": "Sudo commands require explicit approval",
        "severity": "HIGH",
        "rule_id": "NO-SUDO"
    }
}
```

## Next Steps

**Learn More:**

- [Writing Policies](./policies/writing-policies.md) - Create custom rules
- [Builtin Policies](./policies/builtin-policies-reference.md) - 11 pre-built policies
- [Signals & Actions](./configuration/signals.md) - Dynamic responses

**Common Tasks:**

- `cupcake init --builtins global_file_lock` - Read-only session
- `cupcake init --global` - Machine-wide policies
- `cupcake trust init && cupcake trust update` - Enable script trust

## Troubleshooting

**"Command not found: cupcake"**

- Add to PATH: `echo 'export PATH="$HOME/.cupcake/bin:$PATH"' >> ~/.zshrc`
- Restart terminal or run: `source ~/.zshrc`

**"Policy not firing"**

- Check enabled: `cupcake inspect --policy-dir .cupcake`
- Enable debug: `cupcake eval --log-level debug`

**"Claude Code not responding to policies"**

- Verify hooks: `cat .claude/settings.json`
- Restart Claude Code after configuration changes

---

**You're ready!** Cupcake is now protecting your codebase with intelligent policies.
