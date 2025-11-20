# OpenCode Plugin Reference

## Overview

This document provides technical reference for the Cupcake OpenCode plugin. The plugin acts as a bridge between OpenCode's JavaScript plugin system and Cupcake's Rust policy engine.

---

## Plugin Installation

### Prerequisites

- OpenCode installed and configured
- Cupcake CLI installed and in PATH
- Node.js or Bun installed (for plugin execution)

### Installation Methods

#### Method 1: NPM Package (Recommended)

```bash
# Install globally
npm install -g @cupcake/opencode-plugin

# Or install per-project
cd /path/to/project
npm install --save-dev @cupcake/opencode-plugin
```

The plugin will auto-install to `.opencode/plugin/cupcake.ts`.

#### Method 2: Manual Installation

```bash
# Copy plugin to project
mkdir -p .opencode/plugin
curl -o .opencode/plugin/cupcake.ts \
  https://raw.githubusercontent.com/cupcake/cupcake/main/plugins/opencode/cupcake.ts

# Or copy globally
mkdir -p ~/.config/opencode/plugin
cp cupcake.ts ~/.config/opencode/plugin/
```

#### Method 3: Direct from Source

```bash
# Clone Cupcake repository
git clone https://github.com/cupcake/cupcake.git
cd cupcake/plugins/opencode

# Install dependencies
npm install

# Build plugin
npm run build

# Link to OpenCode
ln -s $(pwd)/dist/cupcake.js ~/.config/opencode/plugin/cupcake.ts
```

---

## Plugin Configuration

### Basic Configuration

The plugin works out-of-the-box with default settings. For custom configuration, create a `.cupcake/opencode.json` file:

```json
{
  "enabled": true,
  "cupcake_path": "cupcake",
  "harness": "opencode",
  "log_level": "info",
  "timeout_ms": 5000,
  "fail_mode": "closed",
  "cache_decisions": false
}
```

### Configuration Options

| **Option**        | **Type** | **Default**  | **Description**                                     |
| ----------------- | -------- | ------------ | --------------------------------------------------- |
| `enabled`         | boolean  | `true`       | Enable/disable the plugin                           |
| `cupcake_path`    | string   | `"cupcake"`  | Path to cupcake CLI binary                          |
| `harness`         | string   | `"opencode"` | Harness type (always "opencode")                    |
| `log_level`       | string   | `"info"`     | Log level: "debug", "info", "warn", "error"         |
| `timeout_ms`      | number   | `5000`       | Max policy evaluation time (ms)                     |
| `fail_mode`       | string   | `"closed"`   | "open" (allow on error) or "closed" (deny on error) |
| `cache_decisions` | boolean  | `false`      | Cache policy decisions (experimental)               |

### Fail Modes

**Fail-Closed (Default)**: If policy evaluation fails (timeout, error, crash), **deny** the operation.

- **Pros**: Maximum security
- **Cons**: May block legitimate operations on transient errors

**Fail-Open**: If policy evaluation fails, **allow** the operation.

- **Pros**: More resilient to errors
- **Cons**: Security policies may not be enforced

```json
{
  "fail_mode": "open" // Allow on error
}
```

---

## Plugin Architecture

### Component Overview

```
.opencode/plugin/cupcake.ts
├── CupcakePlugin (main export)
│   ├── EventBuilder - Converts OpenCode events to Cupcake format
│   ├── Executor - Executes cupcake CLI and parses response
│   └── DecisionEnforcer - Enforces policy decisions
```

### Event Flow

```
1. OpenCode triggers tool.execute.before
        ↓
2. Plugin intercepts event
        ↓
3. EventBuilder converts to Cupcake JSON
        ↓
4. Executor runs: cupcake eval --harness opencode
        ↓
5. Cupcake evaluates policies and returns decision
        ↓
6. DecisionEnforcer interprets response:
   - "allow" → return (tool executes)
   - "deny" → throw Error (tool blocked)
   - "ask" → throw Error with approval message
        ↓
7. OpenCode receives result
```

---

## Event Handlers

### tool.execute.before

**Fires**: Before any tool execution (bash, edit, read, write, etc.)

**Input**:

```typescript
{
  sessionID: string;
  messageID: string;
  tool: string; // e.g., "bash", "edit", "read"
  args: Record<string, any>;
}
```

**Output**:

- Normal return → Allow tool execution
- Throw Error → Block tool execution

**Example**:

```typescript
"tool.execute.before": async (input, output) => {
  // Convert to Cupcake event
  const event = {
    hook_event_name: "PreToolUse",
    session_id: input.sessionID,
    cwd: directory,
    tool: normalizeTool(input.tool),
    args: output.args
  };

  // Evaluate policy
  const decision = await evaluatePolicy(event);

  // Enforce decision
  if (decision.decision === "deny") {
    throw new Error(decision.reason);
  }
}
```

### tool.execute.after

**Fires**: After tool execution completes

**Input**:

```typescript
{
  sessionID: string;
  messageID: string;
  tool: string;
  args: Record<string, any>;
  result: {
    success: boolean;
    output?: string;
    error?: string;
    exit_code?: number;
  };
}
```

**Use Cases**:

- Validate tool output
- Check for errors or security issues
- Log tool execution for audit

**Example**:

```typescript
"tool.execute.after": async (input, output) => {
  // Build PostToolUse event
  const event = {
    hook_event_name: "PostToolUse",
    session_id: input.sessionID,
    cwd: directory,
    tool: normalizeTool(input.tool),
    args: output.args,
    result: input.result
  };

  // Evaluate policy
  const decision = await evaluatePolicy(event);

  // Note: Tool already executed, can only warn or log
  if (decision.decision === "deny") {
    console.error(`Post-execution policy violation: ${decision.reason}`);
  }
}
```

---

## API Reference

### EventBuilder

Converts OpenCode events to Cupcake JSON format.

```typescript
class EventBuilder {
  static buildPreToolUse(
    sessionId: string,
    cwd: string,
    tool: string,
    args: any,
    agent?: string,
    messageId?: string,
  ): CupcakeEvent;

  static buildPostToolUse(
    sessionId: string,
    cwd: string,
    tool: string,
    args: any,
    result: ToolResult,
    agent?: string,
    messageId?: string,
  ): CupcakeEvent;
}
```

### Executor

Executes Cupcake CLI and parses responses.

```typescript
class Executor {
  constructor(config: PluginConfig);

  async evaluate(event: CupcakeEvent): Promise<CupcakeResponse>;

  // Low-level execution
  async exec(args: string[], stdin: string): Promise<string>;
}
```

### DecisionEnforcer

Enforces policy decisions by throwing errors or allowing execution.

```typescript
class DecisionEnforcer {
  static enforce(response: CupcakeResponse): void;
  // Throws Error if decision is "deny" or "block"
  // Returns normally if decision is "allow"

  static formatError(response: CupcakeResponse): string;
  // Formats error message for user display
}
```

---

## Tool Name Normalization

OpenCode uses lowercase tool names. The plugin normalizes them for Cupcake:

```typescript
const TOOL_NAME_MAP: Record<string, string> = {
  bash: "Bash",
  edit: "Edit",
  write: "Write",
  read: "Read",
  grep: "Grep",
  glob: "Glob",
  list: "List",
  patch: "Patch",
  todowrite: "TodoWrite",
  todoread: "TodoRead",
  webfetch: "WebFetch",
};

function normalizeTool(tool: string): string {
  return TOOL_NAME_MAP[tool.toLowerCase()] || tool;
}
```

Custom tools (user-defined) are passed through as-is.

---

## Error Handling

### Policy Evaluation Errors

If `cupcake eval` fails (non-zero exit, timeout, crash):

**Fail-Closed Mode** (default):

```typescript
throw new Error(
  `Policy evaluation failed: ${error.message}\n` + `The operation is blocked for security.`,
);
```

**Fail-Open Mode**:

```typescript
console.warn(`Policy evaluation failed: ${error.message}`);
console.warn(`Allowing operation in fail-open mode.`);
// Return normally (allow)
```

### Timeout Handling

If policy evaluation exceeds `timeout_ms`:

```typescript
throw new Error(
  `Policy evaluation timed out after ${config.timeout_ms}ms.\n` +
    `The operation is blocked. Consider optimizing policies or increasing timeout.`,
);
```

### Parse Errors

If Cupcake returns invalid JSON:

```typescript
throw new Error(
  `Failed to parse policy response: ${parseError.message}\n` +
    `Raw output: ${stdout}\n` +
    `The operation is blocked for security.`,
);
```

---

## Debugging

### Enable Debug Logging

Set `log_level: "debug"` in `.cupcake/opencode.json`:

```json
{
  "log_level": "debug"
}
```

Debug output is written to **stderr** (visible in OpenCode logs):

```
[cupcake-plugin] DEBUG: Evaluating PreToolUse event
[cupcake-plugin] DEBUG: Tool: Bash, Args: {"command": "git status"}
[cupcake-plugin] DEBUG: Cupcake response: {"decision": "allow"}
[cupcake-plugin] DEBUG: Allowing tool execution
```

### Test Plugin Manually

Execute the plugin logic outside OpenCode:

```bash
# Create test event
cat > event.json <<EOF
{
  "hook_event_name": "PreToolUse",
  "session_id": "test",
  "cwd": "$(pwd)",
  "tool": "Bash",
  "args": {"command": "git commit --no-verify"}
}
EOF

# Test with Cupcake directly
cupcake eval --harness opencode < event.json

# Should output:
# {"decision": "deny", "reason": "..."}
```

### Common Issues

**Issue**: Plugin not firing

- **Check**: Is the plugin file in `.opencode/plugin/` or `~/.config/opencode/plugin/`?
- **Check**: Is OpenCode restarted after plugin installation?
- **Check**: Are there syntax errors in the plugin file?

**Issue**: Policies not evaluating

- **Check**: Is `cupcake` in PATH?
- **Check**: Run `cupcake eval --harness opencode --help` to verify installation
- **Check**: Are policies in `.cupcake/policies/` directory?

**Issue**: Slow performance

- **Check**: Enable `cache_decisions` (experimental)
- **Check**: Increase `timeout_ms` if policies are complex
- **Check**: Use `--skip-signals` flag for fast policies (future)

---

## Performance Optimization

### Expected Latency

| **Scenario**                  | **Target** | **Typical** |
| ----------------------------- | ---------- | ----------- |
| Simple policy (no signals)    | < 100ms    | 50-80ms     |
| Complex policy (with signals) | < 500ms    | 200-400ms   |
| Cached decision (future)      | < 10ms     | 5-8ms       |

### Optimization Strategies

**1. Minimize Signals**

Only request signals you actually use:

```rego
# BAD: Requests all signals
git_status := input.signals.git_status

# GOOD: Only request what you need
# Signal configuration in rulebook.yml is precise
```

**2. Use Routing Metadata**

Ensure policies are only evaluated when needed:

```rego
# METADATA
# scope: package
# custom:
#   routing:
#     required_events: ["PreToolUse"]
#     required_tools: ["Bash"]
package cupcake.policies.git_safety
```

**3. Enable Caching (Experimental)**

```json
{
  "cache_decisions": true
}
```

**Note**: Caching is experimental. It may not be suitable for dynamic policies.

---

## Security Considerations

### Fail Mode Selection

**Production Environments**: Use `fail_mode: "closed"` to ensure all policy violations are blocked even on transient errors.

**Development Environments**: Consider `fail_mode: "open"` to reduce friction, but understand the security trade-off.

### Timeout Configuration

**Too Low**: May cause legitimate policies to timeout and operations to be blocked.
**Too High**: May cause OpenCode to hang during policy evaluation.

**Recommended**: 5000ms (5 seconds) for most use cases.

### Plugin Trust

The Cupcake plugin executes arbitrary shell commands (`cupcake eval`). Ensure:

- Cupcake binary is from a trusted source
- Plugin code is reviewed before installation
- File permissions prevent unauthorized modification

---

## Advanced Usage

### Custom Tool Integration

If you create custom OpenCode tools, they'll automatically work with Cupcake:

```typescript
// .opencode/tool/deploy.ts
export default tool({
  description: "Deploy to production",
  args: {
    environment: tool.schema.string(),
  },
  async execute(args) {
    // Your deployment logic
  },
});
```

The Cupcake plugin will intercept `tool.execute.before` for `deploy` tool and evaluate policies.

### Multi-Agent Support

If OpenCode supports subagents (like Claude Code's Task tool), the plugin should work automatically:

```typescript
// Plugin tracks agent context
const event = {
  hook_event_name: "PreToolUse",
  session_id: input.sessionID,
  agent: input.agent, // "main" or subagent name
  // ...
};
```

Policies can route based on agent name (future feature).

---

## Troubleshooting

### Plugin Not Loading

**Symptoms**: Tools execute without policy evaluation.

**Diagnosis**:

```bash
# Check plugin is in correct location
ls -la .opencode/plugin/cupcake.ts
# or
ls -la ~/.config/opencode/plugin/cupcake.ts

# Check for syntax errors
npx tsc --noEmit .opencode/plugin/cupcake.ts
```

**Solution**: Ensure plugin file exists and has no syntax errors.

---

### Cupcake Not Found

**Symptoms**: Error: `cupcake: command not found`

**Diagnosis**:

```bash
# Check cupcake is in PATH
which cupcake

# Check cupcake works
cupcake --version
```

**Solution**:

- Install Cupcake: `curl -fsSL https://cupcake.sh/install | bash`
- Or specify full path in config:
  ```json
  {
    "cupcake_path": "/usr/local/bin/cupcake"
  }
  ```

---

### Policies Not Evaluating

**Symptoms**: All tools allowed despite deny policies.

**Diagnosis**:

```bash
# Test policy directly
cupcake eval --harness opencode --debug < test_event.json

# Check policy routing
cupcake eval --harness opencode --debug-routing < test_event.json
```

**Solution**:

- Verify policies are in `.cupcake/policies/`
- Check routing metadata matches event/tool
- Review policy syntax for errors

---

### Performance Issues

**Symptoms**: OpenCode hangs or is slow during tool execution.

**Diagnosis**:

```bash
# Benchmark policy evaluation
time cupcake eval --harness opencode < test_event.json
```

**Solution**:

- Increase `timeout_ms` in config
- Optimize policies (reduce signals, simplify logic)
- Enable caching (experimental)
- Consider persistent daemon (future)

---

## API Compatibility

### OpenCode Version Support

| **OpenCode Version** | **Plugin Version** | **Status**     |
| -------------------- | ------------------ | -------------- |
| 1.0.x                | 1.0.x              | ✅ Supported   |
| 0.9.x                | -                  | ⚠️ Untested    |
| < 0.9                | -                  | ❌ Unsupported |

### Breaking Changes

**Plugin v1.x → v2.x** (future):

- May require OpenCode 1.1.x or later
- May change event format or response schema
- Migration guide will be provided

---

## Getting Help

### Documentation

- Main docs: https://docs.cupcake.sh/agents/opencode
- Policy guide: https://docs.cupcake.sh/policies
- Examples: https://github.com/cupcake/cupcake/tree/main/examples/opencode

### Support Channels

- GitHub Issues: https://github.com/cupcake/cupcake/issues
- Discord: https://discord.gg/cupcake
- Email: support@cupcake.sh

### Reporting Bugs

Include in your bug report:

1. OpenCode version (`opencode --version`)
2. Cupcake version (`cupcake --version`)
3. Plugin configuration (`.cupcake/opencode.json`)
4. Policy files (if relevant)
5. Debug logs (`log_level: "debug"`)
6. Steps to reproduce

---

## Changelog

### v1.0.0 (TBD)

- Initial release
- Support for `tool.execute.before` event
- Support for `tool.execute.after` event
- Basic deny/allow decisions
- Ask → Deny conversion
- Configurable fail modes
- Debug logging

### Future Versions

- Session event support
- Context injection
- Decision caching
- Persistent daemon mode
- Native ask support (if OpenCode adds capability)
