# Cursor Harness Configuration

## Critical Difference from Claude Code

**Cursor hooks MUST be installed globally** at `~/.cursor/hooks.json`. Unlike Claude Code which supports both project-level (`.claude/settings.json`) and global (`~/.claude/settings.json`) configurations, **Cursor does not support project-level hooks**.

Reference: [Cursor Hooks Documentation](https://cursor.com/docs/agent/hooks.md)

## The Configuration Behavior

### `cupcake init --harness cursor` (Project Init)

Creates hooks at `~/.cursor/hooks.json` (global) with **relative policy paths**:

```json
{
  "version": 1,
  "hooks": {
    "beforeShellExecution": [{
      "command": "cupcake eval --harness cursor --policy-dir .cupcake"
    }],
    ...
  }
}
```

**Why relative paths work**: Cursor spawns hook processes with `cwd` set to the workspace root. When the hook runs, `.cupcake` resolves to the project's policy directory.

**Example flow**:

1. User opens `/Users/alice/myproject/` in Cursor
2. Agent attempts to run a shell command
3. Cursor spawns: `cupcake eval --harness cursor --policy-dir .cupcake`
4. Process cwd: `/Users/alice/myproject/`
5. `.cupcake` → `/Users/alice/myproject/.cupcake/`

### `cupcake init --global --harness cursor` (Global Init)

Creates hooks at `~/.cursor/hooks.json` (global) with **absolute policy paths**:

```json
{
  "version": 1,
  "hooks": {
    "beforeShellExecution": [{
      "command": "cupcake eval --harness cursor --policy-dir /Users/alice/.config/cupcake"
    }],
    ...
  }
}
```

**Use case**: Organization-wide policies that apply to all Cursor workspaces.

## Implementation Details

### File: `harness_config.rs`

#### `CursorHarness::settings_path()`

```rust
fn settings_path(&self, _global: bool) -> PathBuf {
    // Cursor hooks MUST always be in ~/.cursor/hooks.json (global)
    // Cursor does not support project-level hooks like Claude Code does.
    // The hooks are always read from the user's home directory.
    // Reference: https://cursor.com/docs/agent/hooks.md
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("~"))
        .join(".cursor")
        .join("hooks.json")
}
```

**Key point**: The `global` parameter is ignored (prefixed with `_`). Cursor hooks are **always** written to the global location.

#### `CursorHarness::generate_hooks()`

```rust
fn generate_hooks(&self, policy_dir: &Path, global: bool) -> Result<Value> {
    let policy_path = if global {
        // Global config - use absolute path
        let abs_path = fs::canonicalize(policy_dir)
            .unwrap_or_else(|_| policy_dir.to_path_buf());
        abs_path.display().to_string()
    } else {
        // Project config - use relative path from workspace root
        ".cupcake".to_string()
    };

    Ok(json!({
        "version": 1,
        "hooks": {
            "beforeShellExecution": [{
                "command": format!("cupcake eval --harness cursor --policy-dir {}", policy_path)
            }],
            ...
        }
    }))
}
```

**Key point**: The `global` parameter determines whether to use absolute or relative paths in the hook commands, NOT the location of the hooks file.

## Bug History

### Original Bug (Fixed 2025-09-03)

**Problem**: `CursorHarness::settings_path()` returned `.cursor/hooks.json` (project-level) when `global=false`, but Cursor ignores project-level hook files.

**Result**: Running `cupcake init --harness cursor` created `.cursor/hooks.json` in the project directory, which Cursor never read.

**Fix**: Changed `settings_path()` to always return `~/.cursor/hooks.json` regardless of the `global` parameter.

## Testing

### Verify Project Init

```bash
cd /tmp/test-project
cupcake init --harness cursor

# Verify hooks created at global location
cat ~/.cursor/hooks.json

# Should show relative path: --policy-dir .cupcake
```

### Verify Global Init

```bash
cupcake init --global --harness cursor

# Verify hooks created at global location
cat ~/.cursor/hooks.json

# Should show absolute path: --policy-dir /Users/alice/.config/cupcake
```

## Comparison with Claude Code

| Aspect                    | Claude Code                    | Cursor                    |
| ------------------------- | ------------------------------ | ------------------------- |
| **Project hooks**         | `.claude/settings.json` ✅     | Not supported ❌          |
| **Global hooks**          | `~/.claude/settings.json` ✅   | `~/.cursor/hooks.json` ✅ |
| **Hook location choice**  | Respects `--global` flag       | Always global             |
| **Policy path (project)** | `$CLAUDE_PROJECT_DIR/.cupcake` | `.cupcake` (relative)     |
| **Policy path (global)**  | Absolute path                  | Absolute path             |
| **Process cwd**           | Project root (via env var)     | Workspace root (direct)   |

## Key Takeaways

1. **Cursor hooks are ALWAYS global** - stored at `~/.cursor/hooks.json`
2. **Project init uses relative paths** - `.cupcake` resolves via workspace cwd
3. **Global init uses absolute paths** - points to `~/.config/cupcake/`
4. **The `global` parameter affects policy paths, not hook file location** (for Cursor)
5. **Cursor spawns hooks with cwd=workspace root** - enabling relative path resolution
