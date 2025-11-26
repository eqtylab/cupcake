# OpenCode Plugin Reference

Technical reference for the Cupcake OpenCode plugin.

## Installation

### From Source

```bash
cd /path/to/cupcake/cupcake-plugins/opencode
npm install
npm run build

# Install to project
mkdir -p /your/project/.opencode/plugins/cupcake
cp -r dist/* /your/project/.opencode/plugins/cupcake/
cp package.json /your/project/.opencode/plugins/cupcake/
```

### Plugin Location

- **Project-level**: `.opencode/plugins/cupcake/`
- **Global**: `~/.config/opencode/plugins/cupcake/`

Project plugins override global plugins.

## Configuration

Create `.cupcake/opencode.json`:

```json
{
  "enabled": true,
  "cupcakePath": "cupcake",
  "harness": "opencode",
  "logLevel": "info",
  "timeoutMs": 5000,
  "failMode": "closed",
  "cacheDecisions": false
}
```

### Options

| Option           | Type    | Default      | Description                         |
| ---------------- | ------- | ------------ | ----------------------------------- |
| `enabled`        | boolean | `true`       | Enable/disable plugin               |
| `cupcakePath`    | string  | `"cupcake"`  | Path to cupcake binary              |
| `harness`        | string  | `"opencode"` | Harness type (always opencode)      |
| `logLevel`       | string  | `"info"`     | Log level: debug, info, warn, error |
| `timeoutMs`      | number  | `5000`       | Max evaluation time (ms)            |
| `failMode`       | string  | `"closed"`   | Error handling: "open" or "closed"  |
| `cacheDecisions` | boolean | `false`      | Cache decisions (experimental)      |

### Fail Modes

**closed** (default, recommended for production):

- Policy error → Deny operation
- Maximum security

**open** (for development):

- Policy error → Allow operation
- Logs warning

## Event Handlers

### tool.execute.before

Fires before any tool execution.

**Input**:

```typescript
{
  sessionID: string;
  messageID: string;
  tool: string; // "bash", "edit", "read", etc.
  args: Record<string, any>;
}
```

**Behavior**:

- Return normally → Allow tool execution
- Throw Error → Block tool execution

### tool.execute.after

Fires after tool execution completes.

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

- Audit logging
- Post-execution validation
- Error tracking

## Tool Name Mapping

The plugin normalizes tool names from OpenCode format to Cupcake format:

```typescript
const TOOL_NAME_MAP = {
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
```

Unknown tools pass through unchanged.

## Cupcake Event Format

The plugin builds events in this format:

```json
{
  "hook_event_name": "PreToolUse",
  "session_id": "abc123",
  "cwd": "/path/to/project",
  "tool": "bash",
  "args": {
    "command": "git status"
  }
}
```

## Cupcake Response Format

Expected response from `cupcake eval`:

```json
{
  "decision": "allow" | "deny" | "block" | "ask",
  "reason": "Human-readable explanation",
  "context": ["Optional", "context", "strings"]
}
```

## Error Handling

### Timeout

If evaluation exceeds `timeoutMs`:

- **closed mode**: Throws error, blocks operation
- **open mode**: Logs warning, allows operation

### Parse Errors

If cupcake returns invalid JSON:

- **closed mode**: Throws error with raw output
- **open mode**: Logs warning, allows operation

### Process Errors

If cupcake crashes or returns non-zero:

- **closed mode**: Throws error
- **open mode**: Logs warning, allows operation

## Debugging

### Enable Debug Logging

```json
{
  "logLevel": "debug"
}
```

Debug output goes to stderr:

```
[cupcake-plugin] DEBUG: Evaluating PreToolUse event
[cupcake-plugin] DEBUG: Tool: Bash, Args: {"command": "git status"}
[cupcake-plugin] DEBUG: Response: {"decision": "allow"}
```

### Test Manually

```bash
# Create test event
echo '{
  "hook_event_name": "PreToolUse",
  "session_id": "test",
  "cwd": "'$(pwd)'",
  "tool": "bash",
  "args": {"command": "git commit --no-verify"}
}' | cupcake eval --harness opencode
```

## Troubleshooting

### Plugin Not Loading

1. Check plugin location:

   ```bash
   ls -la .opencode/plugins/cupcake/
   # Should contain: index.js, package.json
   ```

2. Restart OpenCode after installing plugin

3. Check for syntax errors:
   ```bash
   node .opencode/plugins/cupcake/index.js
   ```

### cupcake Not Found

Specify full path in config:

```json
{
  "cupcakePath": "/usr/local/bin/cupcake"
}
```

### Policies Not Evaluating

1. Verify policies exist:

   ```bash
   ls .cupcake/policies/opencode/
   ```

2. Check routing metadata in policies

3. Enable debug logging:
   ```bash
   cupcake eval --harness opencode --log-level debug < event.json
   ```

### Performance Issues

1. Increase timeout:

   ```json
   { "timeoutMs": 10000 }
   ```

2. Check policy complexity

3. Benchmark:
   ```bash
   time cupcake eval --harness opencode < event.json
   ```

## API Types

```typescript
interface CupcakeConfig {
  enabled: boolean;
  cupcakePath: string;
  harness: string;
  logLevel: "debug" | "info" | "warn" | "error";
  timeoutMs: number;
  failMode: "open" | "closed";
  cacheDecisions: boolean;
}

interface CupcakeEvent {
  hook_event_name: "PreToolUse" | "PostToolUse";
  session_id: string;
  cwd: string;
  tool: string;
  args: Record<string, any>;
  result?: {
    success: boolean;
    output?: string;
    error?: string;
    exit_code?: number;
  };
}

interface CupcakeResponse {
  decision: "allow" | "deny" | "block" | "ask";
  reason?: string;
  context?: string[];
}
```
