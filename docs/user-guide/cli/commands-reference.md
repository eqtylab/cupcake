# CLI Commands Reference

Complete reference for all Cupcake CLI commands.

## cupcake init

Initialize a new Cupcake project or global configuration.

```bash
cupcake init [OPTIONS]
```

### Options

- `--global` - Initialize machine-wide configuration
- `--harness <TYPE>` - Configure agent integration (claude)
- `--builtins <LIST>` - Enable specific builtins (comma-separated)

### Examples

```bash
# Basic project initialization
cupcake init

# With Claude Code integration
cupcake init --harness claude

# Enable specific builtins
cupcake init --builtins git_pre_check,protected_paths

# Global configuration with custom builtins
cupcake init --global --builtins system_protection
```

### Available Builtins

**Project-level:** `always_inject_on_prompt`, `global_file_lock`, `git_pre_check`, `post_edit_check`, `rulebook_security_guardrails`, `protected_paths`, `git_block_no_verify`, `enforce_full_file_read`

**Global-level:** `system_protection`, `sensitive_data_protection`, `cupcake_exec_protection`

---

## cupcake eval

Evaluate an event against policies (used by agent hooks).

```bash
cupcake eval [OPTIONS]
```

### Options

- `--policy-dir <PATH>` - Directory containing policies (default: ./policies)
- `--debug` - Enable debug output
- `--strict` - Exit non-zero on deny decisions

### Usage

```bash
# Evaluate event from stdin
echo '{"hook_event_name": "PreToolUse", ...}' | cupcake eval --policy-dir .cupcake

# With debug output
cupcake eval --debug < event.json

# Strict mode for CI/CD
cupcake eval --strict --policy-dir .cupcake
```

---

## cupcake verify

Verify engine configuration and policies compile correctly.

```bash
cupcake verify [OPTIONS]
```

### Options

- `--policy-dir <PATH>` - Directory containing policies (default: ./policies)

### Examples

```bash
# Verify project configuration
cupcake verify --policy-dir .cupcake

# Verify after adding new policy
cupcake verify --policy-dir .cupcake
```

### Output

```
Found 5 .rego files
All policies compile successfully
System aggregation policy present
Metadata parsing successful
Verification complete!
```

---

## cupcake validate

Validate policies for syntax and best practices.

```bash
cupcake validate [OPTIONS]
```

### Options

- `--policy-dir <PATH>` - Directory to validate (default: .cupcake/policies)
- `--json` - Output results as JSON

### Examples

```bash
# Validate all policies
cupcake validate --policy-dir .cupcake/policies

# JSON output for CI/CD
cupcake validate --json > validation-report.json
```

### Checks Performed

- Rego syntax validation
- Metadata completeness
- Import statements (rego.v1)
- Decision verb usage
- Common anti-patterns

---

## cupcake inspect

Inspect policies to show metadata and routing information.

```bash
cupcake inspect [OPTIONS]
```

### Options

- `--policy-dir <PATH>` - Directory containing policies
- `--json` - Output as JSON
- `--table` - Display in table format

### Examples

```bash
# Show all policy metadata
cupcake inspect --policy-dir .cupcake/policies

# Table view
cupcake inspect --table

# JSON for programmatic access
cupcake inspect --json | jq '.policies[].routing'
```

### Output Example

```
Policy: cupcake.policies.git_safety
  Events: PreToolUse
  Tools: Bash
  Description: Prevent dangerous git operations

Policy: cupcake.policies.builtins.protected_paths
  Events: PreToolUse
  Tools: Write, Edit, MultiEdit
  Description: Block writes to protected paths
```

---

## cupcake trust

Manage script trust and integrity verification.

```bash
cupcake trust <SUBCOMMAND>
```

### Subcommands

#### trust init
Initialize trust for this project:
```bash
cupcake trust init
```

#### trust update
Update trust manifest with current scripts:
```bash
cupcake trust update
```

#### trust verify
Verify current scripts against trust manifest:
```bash
cupcake trust verify
```

#### trust list
List trusted scripts and their status:
```bash
cupcake trust list
```

#### trust disable
Temporarily disable trust verification:
```bash
cupcake trust disable
```

#### trust enable
Re-enable trust verification:
```bash
cupcake trust enable
```

#### trust reset
Remove trust manifest and disable trust mode:
```bash
cupcake trust reset
```

### Examples

```bash
# Initialize trust for a project
cupcake trust init

# Update trust after adding/changing scripts
cupcake trust update

# Verify all scripts against manifest
cupcake trust verify

# List all trusted scripts and status
cupcake trust list

# Temporarily disable trust for testing
cupcake trust disable
```

---

## Environment Variables

Control Cupcake behavior with environment variables:

### Logging & Debug

- `RUST_LOG` - Set log level (error, warn, info, debug, trace)
- `CUPCAKE_TRACE` - Enable evaluation tracing (eval, signals, wasm, all)
- `CUPCAKE_DEBUG_FILES` - Write debug files to `.cupcake/debug/`

### Configuration

- `CUPCAKE_GLOBAL_CONFIG` - Override global config location
- `CUPCAKE_NO_TRUST` - Disable trust verification
- `CLAUDE_PROJECT_DIR` - Project directory for Claude Code hooks

### Examples

```bash
# Debug logging
RUST_LOG=debug cupcake eval < event.json

# Trace policy evaluation
CUPCAKE_TRACE=eval cupcake eval --policy-dir .cupcake

# Debug to files
CUPCAKE_DEBUG_FILES=1 cupcake eval < event.json

# Disable global config for testing
CUPCAKE_GLOBAL_CONFIG=/nonexistent cupcake verify
```

---

## Exit Codes

Cupcake uses standard exit codes:

- `0` - Success / Allow decision
- `1` - General error
- `2` - Deny decision (in strict mode)
- `3` - Configuration error
- `4` - Policy compilation error

---

## Common Workflows

### Initial Setup
```bash
cupcake init --harness claude --builtins git_pre_check
cupcake verify --policy-dir .cupcake
```

### Add Custom Policy
```bash
# Write policy
echo "package cupcake.policies.custom" > .cupcake/policies/custom.rego
# Validate
cupcake validate --policy-dir .cupcake/policies
# Verify
cupcake verify --policy-dir .cupcake
```

### Debug Policy Issues
```bash
# Enable all debugging
RUST_LOG=debug CUPCAKE_TRACE=all cupcake eval < test-event.json

# Write debug files
CUPCAKE_DEBUG_FILES=1 cupcake eval < test-event.json
ls .cupcake/debug/
```

### Trust Script Workflow
```bash
# Initialize trust for project
cupcake trust init

# Add new signal script
echo '#!/bin/bash' > .cupcake/signals/check.sh
chmod +x .cupcake/signals/check.sh

# Update trust manifest to include new script
cupcake trust update

# Use in guidebook.yml
echo "signals:
  my_check:
    command: .cupcake/signals/check.sh" >> .cupcake/guidebook.yml

# Verify all scripts are trusted
cupcake trust verify
```