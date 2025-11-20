# OpenCode Integration Design

## Overview

This document outlines the design for integrating Cupcake's policy engine with OpenCode, a terminal-based AI coding agent that uses a **plugin-based architecture** instead of external hooks.

**Key Difference from Other Harnesses:**

- **Claude Code / Factory AI**: External hooks via stdin/stdout JSON
- **Cursor**: External hooks via stdin/stdout JSON
- **OpenCode**: In-process JavaScript/TypeScript plugins

This fundamental architectural difference requires a **hybrid approach**: a TypeScript plugin that bridges OpenCode's JavaScript runtime to Cupcake's Rust policy engine via shell execution.

---

## Architecture

### High-Level Flow

```
┌─────────────────────────────────────────────────────────────────┐
│                        OpenCode Process                         │
│                                                                 │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │            OpenCode Plugin System (JavaScript)             │ │
│  │                                                            │ │
│  │  ┌──────────────────────────────────────────────────────┐  │ │
│  │  │   Cupcake Plugin (@cupcake/opencode-plugin)          │  │ │
│  │  │   Location: .opencode/plugin/cupcake.ts              │  │ │
│  │  │                                                      │  │ │
│  │  │   1. Intercepts tool.execute.before event            │  │ │
│  │  │   2. Builds Cupcake event JSON payload               │  │ │
│  │  │   3. Executes: cupcake eval --harness opencode       │  │ │
│  │  │   4. Parses JSON response from stdout                │  │ │
│  │  │   5. Enforces decision:                              │  │ │
│  │  │      - Throw Error to block                          │  │ │
│  │  │      - Return void to allow                          │  │ │
│  │  └──────────────────────────────────────────────────────┘  │ │
│  └────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
                              │
                              │ Shell: cupcake eval --harness opencode
                              │ stdin: JSON event
                              ↓
┌─────────────────────────────────────────────────────────────────┐
│                    Cupcake Rust Engine                          │
│                                                                 │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │  1. Parse OpenCodeEvent from stdin                       │   │
│  │  2. Route to matching policies (O(1) lookup)             │   │
│  │  3. Gather signals (git status, file contents, etc.)     │   │
│  │  4. Evaluate policies in WASM sandbox                    │   │
│  │  5. Synthesize final decision (Halt > Deny > Ask > Allow)│   │
│  │  6. Format OpenCodeResponse JSON                         │   │
│  │  7. Write to stdout                                      │   │
│  └──────────────────────────────────────────────────────────┘   │
│                                                                 │
│  Output: { decision: "allow"|"deny"|"ask", reason: "...", ... } │
└─────────────────────────────────────────────────────────────────┘
                              │
                              │ stdout: JSON response
                              ↓
                    Plugin enforces decision
```

---

## OpenCode Plugin System

### Plugin Structure

OpenCode plugins are **JavaScript/TypeScript modules** that export plugin functions:

```typescript
// .opencode/plugin/cupcake.ts
import type { Plugin } from "@opencode-ai/plugin";

export const CupcakePlugin: Plugin = async ({ project, client, $, directory, worktree }) => {
  // Initialization code here

  return {
    // Event handlers (hooks)
    "tool.execute.before": async (input, output) => {
      // Handle pre-tool execution
    },

    "tool.execute.after": async (input, output) => {
      // Handle post-tool execution
    },

    event: async ({ event }) => {
      // Handle other events
    },
  };
};
```

### Plugin Context

Plugins receive context about the current session:

- `project` - Project information
- `directory` - Current working directory
- `worktree` - Git worktree path
- `client` - OpenCode SDK client for AI interaction
- `$` - Bun's shell API for executing commands

### Plugin Capabilities

**What plugins CAN do:**

- Intercept tool execution via `tool.execute.before`
- Inspect tool execution results via `tool.execute.after`
- Execute shell commands via Bun's `$` API
- Throw errors to **block** tool execution
- Return values to **allow** tool execution
- Access session/message metadata

**What plugins CANNOT do:**

- Directly inject context into the LLM prompt (no equivalent to `hookSpecificOutput`)
- Show interactive approval prompts (no native `ask` mechanism)
- Modify tool arguments (read-only in `before` hook)
- Return complex structured responses (only throw/return)

---

## Event Mapping

### OpenCode Events → Cupcake Events

| **OpenCode Event**    | **Cupcake Event**  | **Purpose**                    | **Priority** |
| --------------------- | ------------------ | ------------------------------ | ------------ |
| `tool.execute.before` | `PreToolUse`       | Block tools before execution   | **HIGH**     |
| `tool.execute.after`  | `PostToolUse`      | Validate after execution       | **HIGH**     |
| `session.created`     | `SessionStart`     | Initialize session context     | MEDIUM       |
| `session.idle`        | `SessionEnd`       | Cleanup on session end         | LOW          |
| `tui.prompt.append`   | `UserPromptSubmit` | Inject context on user prompts | MEDIUM       |
| `session.compacted`   | `PreCompact`       | Handle memory compaction       | LOW          |

**Phase 1 Focus**: `tool.execute.before` (PreToolUse) - this is the critical path for policy enforcement.

### Tool Names

OpenCode uses lowercase tool names. We need normalization:

| **OpenCode Tool** | **Cupcake Tool** | **Notes**                              |
| ----------------- | ---------------- | -------------------------------------- |
| `bash`            | `Bash`           | Execute shell commands                 |
| `edit`            | `Edit`           | Modify existing files                  |
| `write`           | `Write`          | Create/overwrite files                 |
| `read`            | `Read`           | Read file contents                     |
| `grep`            | `Grep`           | Search file contents                   |
| `glob`            | `Glob`           | Find files by pattern                  |
| `list`            | `List`           | List directory contents                |
| Custom tools      | `<name>`         | User-defined via `@opencode-ai/plugin` |

---

## Event Payload Structure

### PreToolUse Event

**OpenCode → Cupcake:**

```json
{
  "hook_event_name": "PreToolUse",
  "session_id": "abc123",
  "cwd": "/home/user/project",
  "agent": "main",
  "message_id": "msg_456",
  "tool": "bash",
  "args": {
    "command": "git commit --no-verify",
    "description": "Commit changes",
    "timeout": 120000
  }
}
```

**Rust Structure:**

```rust
pub struct PreToolUsePayload {
    pub common: CommonOpenCodeData,
    pub tool: String,              // e.g., "bash", "edit", "read"
    pub args: serde_json::Value,   // Tool-specific arguments
}

pub struct CommonOpenCodeData {
    pub session_id: String,
    pub cwd: String,
    pub agent: Option<String>,
    pub message_id: Option<String>,
}
```

### PostToolUse Event

```json
{
  "hook_event_name": "PostToolUse",
  "session_id": "abc123",
  "cwd": "/home/user/project",
  "agent": "main",
  "message_id": "msg_456",
  "tool": "bash",
  "args": {
    "command": "npm test"
  },
  "result": {
    "success": false,
    "output": "Test failed: expected 5, got 3",
    "exit_code": 1
  }
}
```

---

## Response Format

### Cupcake → Plugin Response

Unlike Claude Code's complex response format, OpenCode requires a simple JSON response:

```json
{
  "decision": "allow" | "deny" | "block" | "ask",
  "reason": "Human-readable explanation",
  "context": ["Optional", "context", "strings"]
}
```

### Plugin Decision Enforcement

```typescript
const response = await $`cupcake eval --harness opencode`.stdin("json", event).json();

if (response.decision === "deny" || response.decision === "block") {
  throw new Error(response.reason || "Policy violation");
}

if (response.decision === "ask") {
  // OpenCode doesn't have native ask support
  // Convert to deny with explanation
  throw new Error(`[APPROVAL REQUIRED] ${response.reason}`);
}

// decision === "allow" - return normally (implicit allow)
```

### Error Display

When the plugin throws an error, OpenCode displays it to the user:

```
❌ Policy Violation: GIT-NO-VERIFY

Attempted to run 'git commit --no-verify'

Reason: The --no-verify flag bypasses pre-commit hooks and security checks.
This is blocked by your organization's security policy.

To fix: Run 'git commit' without the --no-verify flag.
```

---

## Critical Design Challenges

### 1. Ask Decision Handling

**Problem**: OpenCode plugins cannot interactively prompt users for approval.

**Solutions Considered:**

| **Option** | **Approach**                        | **Pros**           | **Cons**                                | **Verdict**        |
| ---------- | ----------------------------------- | ------------------ | --------------------------------------- | ------------------ |
| **A**      | Convert `ask` → `deny` with message | Simple, immediate  | No approval flow                        | ✅ **Phase 1**     |
| **B**      | Use OpenCode's `permission` config  | Native to OpenCode | Requires tool-level config, not dynamic | 🔍 **Investigate** |
| **C**      | Create custom approval tool         | Flexible           | Complex, requires UI work               | ⏰ **Future**      |

**Phase 1 Implementation**: Convert `ask` to `deny` with clear message:

```
[APPROVAL REQUIRED]
Rule: SENSITIVE-FILE-READ
Attempting to read: ~/.ssh/id_rsa

This requires manual approval. To proceed:
1. Review the request
2. Temporarily disable this policy if appropriate
3. Re-run the command
```

### 2. Context Injection

**Problem**: No direct equivalent to Claude Code's `hookSpecificOutput.additionalContext`.

**Solutions Considered:**

| **Option** | **Approach**                     | **Pros**          | **Cons**                        | **Verdict**        |
| ---------- | -------------------------------- | ----------------- | ------------------------------- | ------------------ |
| **A**      | Return context in error messages | Visible to user   | Not in LLM context              | ❌ Not viable      |
| **B**      | Create custom tool for injection | Full control      | Requires tool implementation    | 🔍 **Investigate** |
| **C**      | Use `tui.prompt.append` event    | Native event      | Limited to prompt time          | ✅ **Phase 2**     |
| **D**      | Modify session context via SDK   | Direct LLM access | Need to verify SDK capabilities | 🔍 **Investigate** |

**Open Questions:**

- ✅ Does OpenCode's `client` SDK support context injection?
- ✅ Can `tui.prompt.append` be used to inject policy context?
- ✅ Is there a session-level context store accessible to plugins?

**Phase 1**: No context injection (return empty `context` array)
**Phase 2**: Implement via `tui.prompt.append` or custom tool

### 3. Performance & Latency

**Problem**: Spawning `cupcake eval` on every tool call adds latency.

**Measurements Needed:**

- ✅ Benchmark: Policy evaluation time (WASM compilation + execution)
- ✅ Benchmark: Process spawn overhead
- ✅ Benchmark: JSON serialization/deserialization
- ✅ Target: < 100ms for simple policies (no signals)
- ✅ Target: < 500ms for complex policies (with signals)

**Optimizations:**

| **Strategy**                        | **Impact** | **Complexity** | **Phase** |
| ----------------------------------- | ---------- | -------------- | --------- |
| Cache WASM compilation              | High       | Low            | Phase 1   |
| `--skip-signals` flag for fast path | Medium     | Low            | Phase 1   |
| Persistent Cupcake daemon           | Very High  | High           | Future    |
| Plugin-side caching of decisions    | Medium     | Medium         | Phase 3   |

### 4. Tool Argument Modification

**Problem**: OpenCode's `tool.execute.before` provides **read-only** access to tool arguments. We cannot modify them like Factory AI's `updatedInput`.

**Impact**: Policies that want to **modify** tool behavior (e.g., add flags, change paths) cannot do so.

**Workarounds:**

- Use `deny` decision to block and instruct user to modify command
- For common patterns, create custom tools that wrap built-in tools
- Future: Request OpenCode team to add argument modification support

**Phase 1**: Document limitation, only support allow/deny decisions

---

## Integration Points with Existing Cupcake Features

### Routing System

OpenCode events should work seamlessly with Cupcake's routing:

```rego
# METADATA
# scope: package
# custom:
#   routing:
#     required_events: ["PreToolUse"]
#     required_tools: ["Bash"]
package cupcake.policies.git_safety
```

**No changes needed** - routing is harness-agnostic.

### Signal System

OpenCode events should access all existing signals:

```rego
git_branch := input.signals.git_branch
file_contents := input.signals.file_read["/path/to/file"]
```

**No changes needed** - signals are harness-agnostic.

### Builtin Policies

All builtin policies should work with OpenCode:

- ✅ `git_pre_check` - Works (PreToolUse event)
- ✅ `protected_paths` - Works (PreToolUse event)
- ✅ `git_block_no_verify` - Works (Bash tool)
- ⚠️ `claude_code_always_inject_on_prompt` - Needs OpenCode equivalent
- ⚠️ `claude_code_enforce_full_file_read` - Needs tool name mapping

**Action**: Update builtin routing to include OpenCode-specific events where needed.

---

## Implementation Phases

### Phase 1: Core Harness Support (MVP)

**Goal**: Basic OpenCode harness that can evaluate policies and return allow/deny decisions.

**Deliverables:**

1. **Rust Implementation**:
   - Add `OpenCode` to `HarnessType` enum
   - Create `cupcake-core/src/harness/events/opencode/` module
     - `mod.rs` - Event enum
     - `common.rs` - Shared structures
     - `pre_tool_use.rs` - PreToolUse payload
     - `post_tool_use.rs` - PostToolUse payload
   - Create `cupcake-core/src/harness/response/opencode/` module
     - `mod.rs` - Response builder
     - Simple JSON format: `{decision, reason, context}`
   - Implement `OpenCodeHarness` in `cupcake-core/src/harness/mod.rs`
   - Update CLI to accept `--harness opencode`

2. **TypeScript Plugin**:
   - Create `cupcake-plugin-opencode/` package
   - Implement `tool.execute.before` hook
   - Convert OpenCode events to Cupcake JSON
   - Execute `cupcake eval --harness opencode`
   - Parse response and enforce decisions
   - Handle errors gracefully

3. **Testing**:
   - Unit tests for event parsing
   - Unit tests for response formatting
   - Integration test: Simple deny policy
   - Integration test: Allow policy
   - Integration test: Ask → Deny conversion

**Success Criteria**:

- ✅ `cupcake eval --harness opencode` accepts JSON events
- ✅ Plugin successfully blocks denied tools
- ✅ Plugin allows approved tools
- ✅ Error messages are clear and actionable

**Estimated Effort**: 3-5 days

---

### Phase 2: Session Events & Context

**Goal**: Support session lifecycle events and explore context injection.

**Deliverables:**

1. **Session Events**:
   - Implement `SessionStart` event
   - Implement `SessionEnd` event
   - Test with session-aware policies

2. **Context Injection Research**:
   - Test `tui.prompt.append` for context injection
   - Test OpenCode `client` SDK capabilities
   - Document viable approaches

3. **Example Policies**:
   - Session initialization policy
   - Session cleanup policy
   - Context injection examples (if viable)

**Open Questions to Answer**:

- ✅ Can we inject context into the LLM prompt via `tui.prompt.append`?
- ✅ Does the `client` SDK expose session context mutation?
- ✅ Are there other events that support context injection?

**Success Criteria**:

- ✅ Session events fire correctly
- ✅ Session-aware policies work
- ✅ Context injection mechanism identified (or documented as unsupported)

**Estimated Effort**: 2-3 days

---

### Phase 3: Advanced Events & Optimization

**Goal**: Support all OpenCode events and optimize performance.

**Deliverables:**

1. **Additional Events**:
   - `PromptAppend` (`tui.prompt.append`)
   - `PreCompact` (`session.compacted`)
   - File events (if needed)

2. **Performance Optimization**:
   - Implement WASM compilation caching
   - Add `--skip-signals` flag for fast policies
   - Benchmark and optimize critical path

3. **Plugin Enhancements**:
   - Plugin-side decision caching (optional)
   - Better error formatting
   - Debug mode for troubleshooting

**Success Criteria**:

- ✅ All relevant events supported
- ✅ Policy evaluation < 100ms for simple policies
- ✅ Policy evaluation < 500ms for complex policies

**Estimated Effort**: 3-4 days

---

### Phase 4: Documentation & Examples

**Goal**: Comprehensive documentation and example policies.

**Deliverables:**

1. **User Documentation**:
   - `docs/agents/opencode/installation.md` - Setup guide
   - `docs/agents/opencode/plugin-reference.md` - Plugin API reference
   - `docs/agents/opencode/policy-examples.md` - Example policies
   - `docs/agents/opencode/troubleshooting.md` - Common issues

2. **Example Policies**:
   - Create `examples/opencode/` directory
   - Git safety policies
   - File protection policies
   - Workflow enforcement policies
   - Builtin usage examples

3. **Integration Tests**:
   - `cupcake-core/tests/opencode_integration_test.rs`
   - Test all decision types
   - Test all event types
   - Test error scenarios

**Success Criteria**:

- ✅ Complete installation guide
- ✅ Working example project with policies
- ✅ All integration tests passing

**Estimated Effort**: 2-3 days

---

### Phase 5: Advanced Features (Future)

**Goal**: Leverage OpenCode-specific capabilities.

**Potential Features**:

1. **Native Ask Support**:
   - Investigate using OpenCode's `permission` system
   - Create custom approval UI if needed

2. **Advanced Context Injection**:
   - Custom tool for context injection
   - Session-level context management

3. **LSP Integration**:
   - Hook into `lsp.client.diagnostics` for code quality policies
   - Real-time linting policy enforcement

4. **File Watcher Policies**:
   - Use `file.watcher.updated` for monitoring
   - React to file changes in real-time

5. **Persistent Daemon**:
   - Long-running Cupcake process
   - Eliminate process spawn overhead
   - Sub-10ms policy evaluation

**Estimated Effort**: TBD based on requirements

---

## Open Questions & Research Needed

### High Priority

1. **Context Injection Mechanism**:
   - ❓ Can we inject context via `tui.prompt.append` event?
   - ❓ Does OpenCode's `client` SDK support context mutation?
   - ❓ Is there a session-level context store we can write to?
   - **Action**: Build prototype plugin to test these approaches

2. **Ask Decision Handling**:
   - ❓ Does OpenCode's `permission` config support dynamic rules?
   - ❓ Can we create a custom tool that prompts for approval?
   - ❓ Is there a way to pause execution and wait for user input?
   - **Action**: Review OpenCode source code or contact maintainers

3. **Tool Argument Access**:
   - ❓ Is `tool.execute.before` truly read-only or can we modify `output.args`?
   - ❓ Can we replace tool calls (like Factory AI's approach)?
   - **Action**: Test with prototype plugin

### Medium Priority

4. **Performance Characteristics**:
   - ❓ What is the process spawn overhead on different platforms?
   - ❓ How much latency is acceptable to users?
   - ❓ Should we implement a persistent daemon from the start?
   - **Action**: Benchmark on macOS, Linux, Windows

5. **Error Handling**:
   - ❓ What happens if `cupcake eval` crashes or times out?
   - ❓ Should we fail-open or fail-closed?
   - ❓ How do we communicate timeout errors to users?
   - **Action**: Test error scenarios

6. **Multi-Agent Support**:
   - ❓ How does OpenCode handle subagents (like the `task` tool)?
   - ❓ Do subagents have separate plugin contexts?
   - ❓ Can we route policies based on agent name?
   - **Action**: Review OpenCode agent documentation

### Low Priority

7. **Session State**:
   - ❓ Can plugins maintain state across tool executions?
   - ❓ Is there a session-level store for policy state?
   - **Action**: Review plugin SDK documentation

8. **Concurrent Execution**:
   - ❓ Can multiple tools execute simultaneously?
   - ❓ Do we need to handle concurrent policy evaluation?
   - **Action**: Test with parallel tool calls

---

## Success Metrics

### Phase 1 (MVP)

- ✅ Plugin successfully blocks policy violations
- ✅ Plugin allows compliant tool executions
- ✅ Error messages are clear and actionable
- ✅ No false positives on simple policies
- ✅ Policy evaluation completes (no crashes)

### Phase 2 (Session Support)

- ✅ Session events fire correctly
- ✅ Session-aware policies work as expected
- ✅ Context injection mechanism identified or documented as unsupported

### Phase 3 (Performance)

- ✅ < 100ms latency for simple policies (no signals)
- ✅ < 500ms latency for complex policies (with signals)
- ✅ All OpenCode events supported

### Phase 4 (Production Ready)

- ✅ Complete documentation
- ✅ Example policies for common use cases
- ✅ All integration tests passing
- ✅ User feedback incorporated

---

## Risk Assessment

| **Risk**                        | **Likelihood** | **Impact** | **Mitigation**                                       |
| ------------------------------- | -------------- | ---------- | ---------------------------------------------------- |
| Context injection not possible  | Medium         | High       | Document limitation, explore alternatives in Phase 2 |
| Ask decisions not viable        | Medium         | Medium     | Use deny with clear messages, explore custom tools   |
| Performance too slow            | Low            | High       | Implement caching and optimizations in Phase 3       |
| Plugin API changes              | Low            | Medium     | Version pin `@opencode-ai/plugin`, track updates     |
| OpenCode limitations discovered | Medium         | Medium     | Document workarounds, engage with maintainers        |

---

## Next Steps

1. **Build Phase 1 Prototype** (Week 1):
   - Implement basic harness in Rust
   - Create minimal TypeScript plugin
   - Test with simple deny policy

2. **Answer Open Questions** (Week 1-2):
   - Test context injection approaches
   - Test ask decision handling
   - Benchmark performance

3. **Complete Phase 1** (Week 2):
   - Full harness implementation
   - Plugin npm package
   - Basic integration tests

4. **Review & Iterate** (Week 3):
   - User testing with example policies
   - Address discovered issues
   - Plan Phase 2 based on findings

---

## Appendix: Comparison with Other Harnesses

| **Feature**           | **Claude Code**     | **Cursor**             | **Factory AI**       | **OpenCode**           |
| --------------------- | ------------------- | ---------------------- | -------------------- | ---------------------- |
| Integration Method    | External hooks      | External hooks         | External hooks       | In-process plugins     |
| Communication         | stdin/stdout JSON   | stdin/stdout JSON      | stdin/stdout JSON    | Function calls + shell |
| Blocking Mechanism    | `{continue: false}` | `{permission: "deny"}` | `{continue: false}`  | Throw Error            |
| Ask Support           | Native              | Native                 | Native               | ⚠️ Limited/None        |
| Context Injection     | `additionalContext` | Limited                | `additionalContext`  | ❓ Unknown             |
| Argument Modification | No                  | No                     | Yes (`updatedInput`) | ❓ Unknown             |
| Performance           | Process spawn       | Process spawn          | Process spawn        | In-process (faster?)   |

**Key Takeaway**: OpenCode's in-process architecture may offer performance benefits but requires a different integration pattern.
