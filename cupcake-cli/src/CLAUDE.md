# Cursor Harness Configuration

## Project-Level and User-Level Hooks

Cursor now supports both project-level and user-level hooks, similar to Claude Code.

**Hook Priority Order (highest to lowest):** Enterprise → Project → User

Reference: [Cursor Hooks Documentation](https://docs.cursor.com/context/hooks)

## Configuration Behavior

### `cupcake init --harness cursor` (Project Init)

Creates hooks at `.cursor/hooks.json` (project-level) with **relative policy paths**:

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

Creates hooks at `~/.cursor/hooks.json` (user-level) with **absolute policy paths**:

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
fn settings_path(&self, global: bool) -> PathBuf {
    // Cursor now supports both project-level and user-level hooks
    // Priority order: Enterprise → Project → User
    // Reference: https://docs.cursor.com/context/hooks
    if global {
        // User-level hooks: ~/.cursor/hooks.json
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("~"))
            .join(".cursor")
            .join("hooks.json")
    } else {
        // Project-level hooks: .cursor/hooks.json
        Path::new(".cursor").join("hooks.json")
    }
}
```

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

## Testing

### Verify Project Init

```bash
cd /tmp/test-project
cupcake init --harness cursor

# Verify hooks created at project location
cat .cursor/hooks.json

# Should show relative path: --policy-dir .cupcake
```

### Verify Global Init

```bash
cupcake init --global --harness cursor

# Verify hooks created at user location
cat ~/.cursor/hooks.json

# Should show absolute path: --policy-dir /Users/alice/.config/cupcake
```

## Comparison with Claude Code

| Aspect                    | Claude Code                    | Cursor                       |
| ------------------------- | ------------------------------ | ---------------------------- |
| **Project hooks**         | `.claude/settings.json` ✅     | `.cursor/hooks.json` ✅      |
| **User hooks**            | `~/.claude/settings.json` ✅   | `~/.cursor/hooks.json` ✅    |
| **Hook location choice**  | Respects `--global` flag       | Respects `--global` flag     |
| **Policy path (project)** | `$CLAUDE_PROJECT_DIR/.cupcake` | `.cupcake` (relative)        |
| **Policy path (global)**  | Absolute path                  | Absolute path                |
| **Process cwd**           | Project root (via env var)     | Workspace root (direct)      |

## Key Takeaways

1. **Cursor supports both project and user-level hooks** - stored at `.cursor/hooks.json` and `~/.cursor/hooks.json`
2. **Project init creates project-level hooks** - `.cursor/hooks.json` with relative policy paths
3. **Global init creates user-level hooks** - `~/.cursor/hooks.json` with absolute policy paths
4. **Cursor spawns hooks with cwd=workspace root** - enabling relative path resolution
