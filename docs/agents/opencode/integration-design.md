# OpenCode Integration Design

Technical architecture for integrating Cupcake with OpenCode.

## Architecture Overview

Unlike Claude Code and Cursor which use external hooks (stdin/stdout JSON), OpenCode uses an **in-process plugin architecture**. This requires a hybrid approach:

```
┌─────────────────────────────────────────────────────────────────┐
│                        OpenCode Process                         │
│                                                                 │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │            Cupcake Plugin (TypeScript)                     │ │
│  │            Location: .opencode/plugins/cupcake/            │ │
│  │                                                            │ │
│  │   1. Intercepts tool.execute.before event                  │ │
│  │   2. Builds Cupcake event JSON payload                     │ │
│  │   3. Executes: cupcake eval --harness opencode             │ │
│  │   4. Parses JSON response from stdout                      │ │
│  │   5. Enforces decision (throw Error or return)             │ │
│  └────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
                              │
                              │ Shell: cupcake eval --harness opencode
                              │ stdin: JSON event
                              ↓
┌─────────────────────────────────────────────────────────────────┐
│                    Cupcake Rust Engine                          │
│                                                                 │
│   1. Parse OpenCodeEvent from stdin                             │
│   2. Preprocess: Map tool names (bash → Bash)                   │
│   3. Route to matching policies (O(1) lookup)                   │
│   4. Gather signals (git status, file contents, etc.)           │
│   5. Evaluate policies in WASM sandbox                          │
│   6. Synthesize final decision (Halt > Deny > Ask > Allow)      │
│   7. Format OpenCodeResponse JSON to stdout                     │
└─────────────────────────────────────────────────────────────────┘
```

## Event Flow

### PreToolUse (tool.execute.before)

1. OpenCode triggers `tool.execute.before` hook
2. Plugin intercepts with tool name and arguments
3. Plugin builds Cupcake JSON event:
   ```json
   {
     "hook_event_name": "PreToolUse",
     "session_id": "abc123",
     "cwd": "/home/user/project",
     "tool": "bash",
     "args": {"command": "git commit --no-verify"}
   }
   ```
4. Plugin spawns: `cupcake eval --harness opencode`
5. Cupcake preprocesses event (adds `tool_name`, `tool_input` fields)
6. Cupcake evaluates policies and returns:
   ```json
   {"decision": "deny", "reason": "..."}
   ```
7. Plugin enforces decision:
   - `"allow"` → return (tool executes)
   - `"deny"` / `"block"` → throw Error (tool blocked)
   - `"ask"` → throw Error with approval message

### PostToolUse (tool.execute.after)

Same flow but includes tool execution result:
```json
{
  "hook_event_name": "PostToolUse",
  "tool": "bash",
  "args": {"command": "npm test"},
  "result": {
    "success": false,
    "output": "Test failed",
    "exit_code": 1
  }
}
```

## Tool Name Mapping

OpenCode uses lowercase tool names. Preprocessing converts them:

| OpenCode | Cupcake | Description |
|----------|---------|-------------|
| `bash` | `Bash` | Shell commands |
| `edit` | `Edit` | File editing |
| `write` | `Write` | File creation |
| `read` | `Read` | File reading |
| `grep` | `Grep` | Content search |
| `glob` | `Glob` | File pattern matching |
| `list` | `List` | Directory listing |
| `patch` | `Patch` | Apply patches |
| `todowrite` | `TodoWrite` | Task management |
| `todoread` | `TodoRead` | Task reading |
| `webfetch` | `WebFetch` | Web requests |

## Response Format

Simple JSON response (unlike Claude Code's complex format):

```json
{
  "decision": "allow" | "deny" | "block" | "ask",
  "reason": "Human-readable explanation",
  "context": ["Optional", "context", "strings"]
}
```

## Plugin Components

```
plugins/opencode/
├── src/
│   ├── index.ts        # Main plugin export, hooks
│   ├── types.ts        # Type definitions, config
│   ├── event-builder.ts # OpenCode → Cupcake event conversion
│   ├── executor.ts     # Spawns cupcake CLI process
│   └── enforcer.ts     # Enforces policy decisions
├── dist/               # Compiled JavaScript
├── package.json
└── tsconfig.json
```

## Rust Harness Components

```
cupcake-core/src/harness/
├── events/opencode/
│   ├── mod.rs          # OpenCodeEvent enum
│   ├── common.rs       # CommonOpenCodeData struct
│   ├── pre_tool_use.rs # PreToolUsePayload
│   └── post_tool_use.rs# PostToolUsePayload
├── response/opencode/
│   └── mod.rs          # OpenCodeResponse struct
└── mod.rs              # OpenCodeHarness impl
```

## Preprocessing

The preprocessing module (`cupcake-core/src/preprocessing/mod.rs`) handles OpenCode-specific transformations:

1. **Tool Name Mapping**: `tool: "bash"` → `tool_name: "Bash"`
2. **Field Renaming**: `args` → `tool_input`
3. **Whitespace Normalization**: For Bash commands
4. **Symlink Resolution**: For file paths (TOB-4 defense)

## Limitations

### Ask Decisions

OpenCode plugins cannot prompt for user approval. Ask decisions are converted to deny with a message:

```
Approval Required

[Policy reason]

This operation requires manual approval. Review the policy 
and re-run the command if appropriate.
```

### Context Injection

No direct equivalent to Claude Code's `hookSpecificOutput.additionalContext`. Context strings are returned but not automatically injected into the LLM prompt.

### Argument Modification

Tool arguments are read-only in `tool.execute.before`. Cannot modify commands like Factory AI's `updatedInput`.

## Configuration

Plugin config in `.cupcake/opencode.json`:

```json
{
  "enabled": true,
  "cupcakePath": "cupcake",
  "logLevel": "info",
  "timeoutMs": 5000,
  "failMode": "closed"
}
```

### Fail Modes

- **closed** (default): Deny on any error (max security)
- **open**: Allow on error (dev-friendly)

## Comparison with Other Harnesses

| Feature | Claude Code | Cursor | OpenCode |
|---------|-------------|--------|----------|
| Integration | External hooks | External hooks | In-process plugin |
| Communication | stdin/stdout JSON | stdin/stdout JSON | Function calls + shell |
| Blocking | `{continue: false}` | `{permission: "deny"}` | `throw Error` |
| Ask Support | Native | Native | Converted to deny |
| Context Injection | `additionalContext` | Limited | Not supported |
| Arg Modification | No | No | No |

## Performance

| Scenario | Target | Typical |
|----------|--------|---------|
| Simple policy (no signals) | < 100ms | 50-80ms |
| Complex policy (with signals) | < 500ms | 200-400ms |

The main overhead is process spawn time for `cupcake eval`. Future optimization could use a persistent daemon.

## Security

- **Fail-closed by default**: Errors result in deny
- **Timeout protection**: Configurable max evaluation time
- **WASM sandbox**: Policies execute in isolated environment
- **Input preprocessing**: Protects against bypass attacks
