# Cursor Policy Resolution

## Hook Configuration Options

Cursor supports two hook configuration locations:
- **Project-level**: `.cursor/hooks.json` (recommended)
- **Global**: `~/.cursor/hooks.json`

Each project has its own `.cupcake/` directory with project-specific policies. This document explains how Cupcake resolves policy paths.

## The Answer: Working Directory

**Cursor spawns hook processes with `cwd` set to the workspace root.**

When Cursor triggers a hook, it:

1. Opens the workspace at `/Users/alice/myproject/`
2. Agent attempts an action (e.g., shell command)
3. Cursor spawns: `cupcake eval --harness cursor --policy-dir .cupcake`
4. **Process working directory: `/Users/alice/myproject/`** ← Key point
5. Relative path `.cupcake` resolves to `/Users/alice/myproject/.cupcake/`

## Complete Flow Diagram

```
User Opens Workspace
    ↓
/Users/alice/myproject/
    ↓
Agent Attempts Action (e.g., shell command)
    ↓
Cursor Reads .cursor/hooks.json (project-level, preferred)
   OR ~/.cursor/hooks.json (global fallback)
    ↓
{
  "beforeShellExecution": [{
    "command": "cupcake eval --harness cursor --policy-dir .cupcake"
  }]
}
    ↓
Cursor Spawns Process:
  - Command: cupcake eval --harness cursor --policy-dir .cupcake
  - CWD: /Users/alice/myproject/  ← This is the magic
  - stdin: {hook_event_name: "beforeShellExecution", command: "rm -rf", ...}
    ↓
Cupcake Resolves Policy Directory:
  .cupcake → /Users/alice/myproject/.cupcake/
    ↓
Cupcake Loads Policies:
  - /Users/alice/myproject/.cupcake/policies/**/*.rego
  - Compiles to WASM
  - Evaluates event against policies
    ↓
Cupcake Returns Response:
  {permission: "deny", user_message: "...", agent_message: "..."}
    ↓
Cursor Blocks Action & Shows Message
```

## Why This Works

### Relative Path Resolution

The shell (or OS process spawner) resolves relative paths against the **current working directory**:

```bash
# If cwd = /Users/alice/myproject/
.cupcake                    → /Users/alice/myproject/.cupcake/
.cupcake/policies/          → /Users/alice/myproject/.cupcake/policies/
.cupcake/bundle.tar.gz      → /Users/alice/myproject/.cupcake/bundle.tar.gz
```

### No Environment Variables Needed

Unlike Claude Code which uses `$CLAUDE_PROJECT_DIR`, Cursor relies on **process working directory**. This is simpler but requires the hook to be spawned in the correct directory (which Cursor does automatically).

## Comparison: Claude Code vs Cursor

| Aspect                    | Claude Code                                           | Cursor                                           |
| ------------------------- | ----------------------------------------------------- | ------------------------------------------------ |
| **Hook Location**         | `.claude/settings.json` OR `~/.claude/settings.json`  | `.cursor/hooks.json` OR `~/.cursor/hooks.json`   |
| **Policy Resolution**     | Environment variable (`$CLAUDE_PROJECT_DIR/.cupcake`) | Process cwd + relative path (`.cupcake`)         |
| **Project Policy Path**   | `$CLAUDE_PROJECT_DIR/.cupcake`                        | `.cupcake`                                       |
| **Global Policy Path**    | `/Users/alice/.config/cupcake/`                       | `/Users/alice/.config/cupcake/`                  |
| **Multi-project Support** | Via environment variable                              | Via working directory                            |
| **Hook Configuration**    | Project-level or global                               | Project-level (`.cursor/`) or global (`~/.cursor/`) |

## Supported Cursor Events

Cursor supports the following hook events:

### Before Events (Can Block)

| Event                  | Description                           | Key Input Fields                    | Response Format |
| ---------------------- | ------------------------------------- | ----------------------------------- | --------------- |
| `beforeShellExecution` | Before running shell command          | `command`, `cwd`                    | `permission`, `user_message`, `agent_message` |
| `beforeMCPExecution`   | Before running MCP tool               | `tool_name`, `tool_input`           | `permission`, `user_message`, `agent_message` |
| `beforeReadFile`       | Before reading a file                 | `file_path`, `content`              | `permission` |
| `beforeSubmitPrompt`   | Before submitting user prompt         | `prompt`, `attachments`             | `continue`, `user_message` |

### After Events (Fire-and-Forget)

| Event                 | Description                            | Key Input Fields                           | Response Format |
| --------------------- | -------------------------------------- | ------------------------------------------ | --------------- |
| `afterShellExecution` | After shell command completes          | `command`, `output`, `duration`            | `{}` (empty) |
| `afterMCPExecution`   | After MCP tool completes               | `tool_name`, `tool_input`, `result_json`, `duration` | `{}` (empty) |
| `afterFileEdit`       | After file is edited                   | `file_path`, `edits`                       | `{}` (empty) |
| `afterAgentResponse`  | After agent sends a message            | `text`                                     | `{}` (empty) |
| `afterAgentThought`   | After agent thinking block             | `text`, `duration_ms`                      | `{}` (empty) |

### Lifecycle Events

| Event    | Description                  | Key Input Fields          | Response Format |
| -------- | ---------------------------- | ------------------------- | --------------- |
| `stop`   | Agent loop has ended         | `status`, `loop_count`    | `{}` (allow) or `{ "followup_message": "..." }` (block) |

#### Stop Hook - Agent Loop Continuation

The `stop` hook enables agent loop workflows. When a policy blocks the stop event,
Cursor will automatically submit the feedback as a new user message, causing the
agent to continue working.

**Decision mapping:**
- `Allow` → `{}` (agent stops normally)
- `Block` → `{"followup_message": "..."}` (agent continues with this message)

**Loop prevention:** Cursor enforces max 5 auto-followups via `loop_count`. Policies
should check `input.loop_count < 5` before blocking.

**Example policy:**
```rego
deny contains decision if {
    input.hook_event_name == "stop"
    input.loop_count < 5  # Respect Cursor's loop limit
    input.status == "completed"
    # Your condition to continue
    needs_more_work(input)
    decision := {
        "rule_id": "CONTINUE-WORK",
        "reason": "Tests are still failing. Please fix them.",
        "severity": "MEDIUM"
    }
}
```

This mirrors Claude Code's Stop hook where `block` + `reason` continues the agent.
The difference is mechanical: Claude Code injects feedback as context, while Cursor
submits it as a new user message.

### Common Input Fields (All Events)

All Cursor events include these common fields:

```json
{
  "conversation_id": "unique-conversation-id",
  "generation_id": "unique-generation-id",
  "workspace_roots": ["/path/to/workspace"],
  "model": "gpt-4",              // Optional: model used
  "cursor_version": "2.0.77",    // Optional: Cursor version
  "user_email": "user@example.com"  // Optional: authenticated user
}
```

### Response Field Naming

**IMPORTANT**: Cursor uses `snake_case` for response fields:
- `user_message` (not `userMessage`)
- `agent_message` (not `agentMessage`)

Example deny response:
```json
{
  "permission": "deny",
  "user_message": "This command is blocked by policy",
  "agent_message": "Policy BLOCK-001 prevented this action"
}
```

## Implementation in Cupcake

### CLI: Hook Generation

File: `cupcake-cli/src/harness_config.rs`

```rust
fn generate_hooks(&self, policy_dir: &Path, global: bool) -> Result<Value> {
    let policy_path = if global {
        // Global config - use absolute path
        fs::canonicalize(policy_dir)
            .unwrap_or_else(|_| policy_dir.to_path_buf())
            .display()
            .to_string()
    } else {
        // Project config - use relative path
        ".cupcake".to_string()  // ← Resolves via workspace cwd
    };

    Ok(json!({
        "hooks": {
            "beforeShellExecution": [{
                "command": format!("cupcake eval --harness cursor --policy-dir {}", policy_path)
            }],
            ...
        }
    }))
}
```

### Core: Event Processing

File: `cupcake-core/src/harness/mod.rs`

```rust
impl CursorHarness {
    /// Parse incoming event (receives from stdin)
    pub fn parse_event(input: &str) -> Result<CursorEvent> {
        Ok(serde_json::from_str(input)?)
    }

    /// Format response for Cursor (writes to stdout)
    pub fn format_response(event: &CursorEvent, decision: &FinalDecision) -> Result<Value> {
        let engine_decision = Self::adapt_decision(decision);
        let agent_messages = Self::extract_agent_messages(decision);
        let response = CursorResponseBuilder::build_response(&engine_decision, event, agent_messages);
        Ok(response)
    }
}
```

## Testing Policy Resolution

### Manual Test

```bash
# Create test project
mkdir -p /tmp/cursor-test/.cupcake/policies
cd /tmp/cursor-test

# Create a simple blocking policy
cat > .cupcake/policies/test.rego << 'EOF'
# METADATA
# scope: package
# custom:
#   routing:
#     required_events: ["beforeShellExecution"]
package cupcake.policies.test

import rego.v1

deny contains decision if {
    input.hook_event_name == "beforeShellExecution"
    contains(input.command, "rm -rf")
    decision := {
        "rule_id": "TEST-001",
        "reason": "rm -rf blocked by test policy",
        "severity": "CRITICAL"
    }
}
EOF

# Compile policies
opa build -t wasm -e cupcake/system/evaluate .cupcake/policies/

# Test with cupcake (cwd matters!)
cd /tmp/cursor-test  # ← This is the key
echo '{"hook_event_name":"beforeShellExecution","command":"rm -rf /tmp/test"}' | \
  cupcake eval --harness cursor --policy-dir .cupcake

# Expected: Policy blocks the command
# Output: {"permission":"deny","user_message":"rm -rf blocked by test policy",...}
```

### What if we run from wrong directory?

```bash
# Run from parent directory
cd /tmp
echo '{"hook_event_name":"beforeShellExecution","command":"rm -rf /tmp/test"}' | \
  cupcake eval --harness cursor --policy-dir .cupcake

# ERROR: Policy directory not found
# Because .cupcake resolves to /tmp/.cupcake (wrong location)
```

This demonstrates why Cursor MUST spawn hooks with cwd=workspace root.

## Multi-Project Workflow

### Scenario: Developer works on 3 projects

Each project can have its own `.cursor/hooks.json` (recommended), or a single global `~/.cursor/hooks.json` can apply to all projects.

**Project-level (recommended):**
```
projectA/.cursor/hooks.json  ← Project-specific hooks
projectB/.cursor/hooks.json  ← Project-specific hooks
```

**Global fallback:**
```
~/.cursor/hooks.json  ← Applies to all projects without local hooks
```

**Project A: `/Users/alice/projectA/`**

```
projectA/
  .cupcake/
    policies/
      security.rego      ← Blocks rm -rf
```

**Project B: `/Users/alice/projectB/`**

```
projectB/
  .cupcake/
    policies/
      database.rego      ← Blocks database operations
```

**Project C: `/Users/alice/projectC/`**

```
projectC/
  (no .cupcake/ directory)
  ← No policies, all actions allowed
```

### How it works:

1. **Open Project A in Cursor**

   - cwd = `/Users/alice/projectA/`
   - `.cupcake` → `/Users/alice/projectA/.cupcake/`
   - Loads `security.rego`

2. **Open Project B in Cursor**

   - cwd = `/Users/alice/projectB/`
   - `.cupcake` → `/Users/alice/projectB/.cupcake/`
   - Loads `database.rego`

3. **Open Project C in Cursor**
   - cwd = `/Users/alice/projectC/`
   - `.cupcake` → `/Users/alice/projectC/.cupcake/` (doesn't exist)
   - Cupcake returns error or allows all (depending on configuration)

## Edge Cases

### 1. Missing `.cupcake/` Directory

If a project doesn't have `.cupcake/`, the hook will fail:

```bash
Error: Policy directory not found: .cupcake
```

**Solution**: Run `cupcake init --harness cursor` in each project.

### 2. Symlinked Directories

If `.cupcake` is a symlink:

```bash
ln -s /shared/policies .cupcake
```

The symlink is resolved correctly because filesystem resolution handles it transparently.

### 3. Nested Workspaces

If Cursor opens a nested directory:

```
/Users/alice/monorepo/
  .cupcake/           ← Root policies
  packages/
    app1/
      .cupcake/       ← App-specific policies
```

**Cursor behavior**: Opens the directory you select. If you open `packages/app1/`, cwd is `packages/app1/`, and `.cupcake` resolves to `packages/app1/.cupcake/`.

## Key Takeaways

1. **Cursor supports both project-level and global hooks** - `.cursor/hooks.json` (preferred) or `~/.cursor/hooks.json`
2. **Process cwd is set by Cursor to the workspace root** - this is documented behavior
3. **Relative paths work because of cwd**, not environment variables
4. **Multi-project support works automatically** - each workspace has its own cwd
5. **Project-level hooks are recommended** for better isolation and version control
6. **Testing must be done from the correct directory** to match Cursor's behavior

## References

- [Cursor Hooks Documentation](https://cursor.com/docs/agent/hooks.md)
- `cupcake-cli/src/harness_config.rs` - Hook generation logic
- `cupcake-cli/src/CLAUDE.md` - Cursor-specific CLI behavior
