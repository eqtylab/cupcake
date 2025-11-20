# OpenCode Integration Research Questions

## Overview

This document tracks open research questions that need to be answered during the OpenCode integration implementation. Questions are prioritized and include investigation approaches.

---

## High Priority Questions

### Q1: Context Injection Mechanisms

**Question**: How can we inject policy context into the LLM's prompt in OpenCode?

**Background**:

- Claude Code supports `hookSpecificOutput.additionalContext`
- Factory AI has the same mechanism
- OpenCode plugins don't have an obvious equivalent

**Possible Approaches**:

1. **`tui.prompt.append` Event**
   - Hook into the prompt append event
   - Inject context strings before LLM sees prompt
   - **Test**: Create plugin that hooks this event and logs when it fires
   - **Test**: Verify injected strings appear in LLM context

2. **OpenCode Client SDK**
   - Use the `client` parameter passed to plugins
   - Check if it has methods like `addContext()` or similar
   - **Test**: Explore `client` API in TypeScript
   - **Test**: Check OpenCode source code for SDK methods

3. **Custom Tool Approach**
   - Create a custom tool called `cupcake_context`
   - LLM calls this tool to get policy context
   - Cupcake plugin responds with context strings
   - **Test**: Create prototype custom tool
   - **Test**: Verify LLM can discover and call it

4. **Session Context Store**
   - Check if OpenCode has a session-level context store
   - Write to it from plugin, LLM reads from it
   - **Test**: Search OpenCode docs for session API
   - **Test**: Test with prototype

**Investigation Plan**:

```typescript
// Test plugin to explore context injection
export const ContextTest: Plugin = async ({ client, $ }) => {
  return {
    "tui.prompt.append": async (input) => {
      console.log("tui.prompt.append fired:", input);
      // Can we modify input here?
      // Can we call client methods to add context?
    },

    event: async ({ event }) => {
      if (event.type === "session.created") {
        // Can we inject context at session start?
        console.log("Exploring client API:", Object.keys(client));
      }
    },
  };
};
```

**Success Criteria**:

- [ ] Identify at least one viable mechanism for context injection
- [ ] Prototype demonstrates context appears in LLM prompt
- [ ] Mechanism works consistently across different scenarios

**Status**: 🔴 Not Started

---

### Q2: Ask Decision Handling

**Question**: How can we implement "ask" decisions (user approval) in OpenCode?

**Background**:

- Claude Code and Cursor have native "ask" support
- OpenCode plugins can only throw errors (block) or return (allow)
- No obvious way to pause execution and prompt for approval

**Possible Approaches**:

1. **OpenCode Permission System**
   - OpenCode has `permission` config in `opencode.json`
   - Check if this can be controlled dynamically
   - **Test**: Review OpenCode permission documentation
   - **Test**: Try setting permissions via plugin API

2. **Convert to Deny with Message**
   - Simple fallback: `ask` → `deny` with explanation
   - Tell user they need to manually approve
   - **Test**: Format clear approval messages
   - **Test**: User experience testing

3. **Custom Approval Tool**
   - Create a `cupcake_approve` tool
   - User manually calls tool to approve operation
   - Plugin checks for recent approval before allowing
   - **Test**: Prototype approval tool
   - **Test**: State management between calls

4. **Interactive CLI Prompt**
   - Plugin spawns interactive prompt via `$` shell API
   - Uses `inquirer` or similar for user input
   - **Test**: Does Bun's `$` support interactive input?
   - **Test**: Will OpenCode block during interactive prompt?

**Investigation Plan**:

```typescript
// Test OpenCode permission system
import { $ } from "bun";

export const AskTest: Plugin = async ({ $ }) => {
  return {
    "tool.execute.before": async (input, output) => {
      // Approach 1: Try permission API
      // (need to find API)

      // Approach 4: Test interactive prompt
      try {
        const result = await $`echo "Approve? (y/n)" && read answer && echo $answer`.text();
        console.log("User response:", result);
      } catch (e) {
        console.error("Interactive prompt failed:", e);
      }
    },
  };
};
```

**Success Criteria**:

- [ ] Identify viable approach for user approval
- [ ] Prototype demonstrates approval flow works
- [ ] User experience is acceptable

**Fallback**: Convert all `ask` → `deny` with clear message explaining approval is needed

**Status**: 🔴 Not Started

---

### Q3: Tool Argument Modification

**Question**: Can we modify tool arguments before execution in OpenCode?

**Background**:

- Factory AI supports `updatedInput` in PreToolUse response
- Allows policies to modify commands (e.g., add flags, change paths)
- OpenCode's `tool.execute.before` appears to provide read-only access

**Investigation Plan**:

```typescript
export const ArgModTest: Plugin = async ({ $ }) => {
  return {
    "tool.execute.before": async (input, output) => {
      console.log("Original args:", output.args);

      // Try to modify args
      output.args = { ...output.args, modified: true };

      // Will OpenCode use modified args?
      console.log("Modified args:", output.args);
    },
  };
};
```

**Test Cases**:

1. Modify bash command (e.g., remove `--no-verify` flag)
2. Change file path in edit tool
3. Add arguments to existing command

**Success Criteria**:

- [ ] Determine if argument modification is possible
- [ ] If possible: Document how to modify args safely
- [ ] If not possible: Document limitation and workarounds

**Workarounds if Not Possible**:

- Block operation with message instructing user to modify
- Create wrapper custom tools that enforce constraints
- Request feature from OpenCode maintainers

**Status**: 🔴 Not Started

---

## Medium Priority Questions

### Q4: Performance Benchmarking

**Question**: What is the actual performance overhead of the plugin approach?

**Measurements Needed**:

1. **Process Spawn Time**: Time to spawn `cupcake eval` process
2. **WASM Compilation**: Time to compile policy to WASM (should be cached)
3. **Policy Evaluation**: Time to evaluate policies in WASM
4. **Signal Gathering**: Time to execute signal commands (git, file reads, etc.)
5. **Total Latency**: End-to-end time from tool trigger to decision

**Test Setup**:

```bash
# Simple policy (no signals)
time cupcake eval --harness opencode < simple_event.json

# Complex policy (with signals)
time cupcake eval --harness opencode < complex_event.json

# Measure components separately
cupcake eval --harness opencode --benchmark < event.json
```

**Platform Testing**:

- macOS (Intel)
- macOS (Apple Silicon)
- Linux (Ubuntu, Debian)
- Windows (WSL, native)

**Target Latency**:

- Simple policy: < 100ms
- Complex policy: < 500ms
- User perception: "Instant" (< 200ms ideal)

**Success Criteria**:

- [ ] Baseline measurements on all platforms
- [ ] Identify bottlenecks
- [ ] Create optimization plan if targets not met

**Status**: 🟡 Needs Baseline Implementation

---

### Q5: Error Handling Strategy

**Question**: What should happen when policy evaluation fails?

**Failure Scenarios**:

1. **Timeout**: Policy evaluation takes too long
2. **Crash**: Cupcake process crashes or exits non-zero
3. **Parse Error**: Invalid JSON response from Cupcake
4. **No Policies**: No policies match the event
5. **Signal Failure**: Signal command fails (e.g., git not available)

**Fail Mode Options**:

**Option A: Fail-Closed (Deny)**

- Block operation on any error
- Maximum security
- May frustrate users on transient errors
- **Good for**: Production, regulated environments

**Option B: Fail-Open (Allow)**

- Allow operation on error
- User-friendly, resilient
- May miss security violations
- **Good for**: Development, fast iteration

**Option C: Fail-Warn (Allow + Log)**

- Allow operation but log error prominently
- Middle ground approach
- **Good for**: Transitioning from dev to prod

**Investigation**:

```typescript
async function evaluateWithErrorHandling(event) {
  try {
    const response = await cupcakeEval(event, { timeout: 5000 });
    return response;
  } catch (error) {
    if (error instanceof TimeoutError) {
      // Handle timeout
      if (config.fail_mode === "closed") {
        throw new Error(`Policy timeout: ${error.message}`);
      } else {
        console.warn(`Policy timeout, allowing in fail-open mode`);
        return { decision: "allow" };
      }
    }

    if (error instanceof CrashError) {
      // Handle crash
      // ...
    }

    // etc.
  }
}
```

**Success Criteria**:

- [ ] Document error handling strategy
- [ ] Implement fail modes
- [ ] Test all error scenarios
- [ ] User-friendly error messages

**Status**: 🟡 Needs Design Decision

---

### Q6: Multi-Agent Support

**Question**: How does OpenCode handle subagents and can we route policies based on agent context?

**Background**:

- Claude Code has Task tool that spawns subagents
- Policies can route based on agent context
- Unknown if OpenCode has similar concept

**Investigation**:

```typescript
export const AgentTest: Plugin = async ({ client, $ }) => {
  return {
    "tool.execute.before": async (input, output) => {
      console.log("Session ID:", input.sessionID);
      console.log("Message ID:", input.messageID);
      console.log("Agent context:", input.agent || "unknown");

      // Can we detect subagents?
      // Can we get agent name/type?
    },
  };
};
```

**Questions**:

- Does OpenCode support subagents?
- Is there an agent identifier in events?
- Can policies route based on agent?

**Success Criteria**:

- [ ] Document agent model in OpenCode
- [ ] Determine if agent-based routing is possible
- [ ] Update routing system if needed

**Status**: 🟡 Needs Investigation

---

## Low Priority Questions

### Q7: Session State Management

**Question**: Can plugins maintain state across tool executions?

**Use Cases**:

- Track approval history (for ask decisions)
- Count violations per session
- Remember user preferences
- Cache expensive computations

**Investigation**:

```typescript
const sessionState = new Map<string, any>();

export const StateTest: Plugin = async ({ $ }) => {
  return {
    "tool.execute.before": async (input, output) => {
      const state = sessionState.get(input.sessionID) || {};

      state.toolCount = (state.toolCount || 0) + 1;
      sessionState.set(input.sessionID, state);

      console.log(`Tool #${state.toolCount} in session ${input.sessionID}`);
    },
  };
};
```

**Questions**:

- Are plugin variables persistent across calls?
- Does plugin reload on each tool call?
- Is there a session-level storage API?

**Success Criteria**:

- [ ] Determine if state management is possible
- [ ] Document state management patterns
- [ ] Create state management utilities if needed

**Status**: 🟢 Low Priority

---

### Q8: Concurrent Tool Execution

**Question**: Can multiple tools execute concurrently and does this affect policy evaluation?

**Scenarios**:

1. LLM calls multiple tools in parallel
2. Multiple OpenCode sessions running
3. Subagent and main agent running simultaneously

**Concerns**:

- Race conditions in signal gathering
- Concurrent Cupcake process spawns
- File system contention

**Investigation**:

```typescript
let activeEvaluations = 0;

export const ConcurrencyTest: Plugin = async ({ $ }) => {
  return {
    "tool.execute.before": async (input, output) => {
      activeEvaluations++;
      console.log(`Active evaluations: ${activeEvaluations}`);

      try {
        await evaluatePolicy(input);
      } finally {
        activeEvaluations--;
      }
    },
  };
};
```

**Success Criteria**:

- [ ] Test concurrent tool execution
- [ ] Identify any race conditions
- [ ] Document concurrency behavior
- [ ] Add locking if needed

**Status**: 🟢 Low Priority

---

### Q9: Custom Tools Integration

**Question**: Do custom OpenCode tools work seamlessly with Cupcake?

**Test Cases**:

1. Create custom tool
2. Verify `tool.execute.before` fires for custom tool
3. Verify tool name is passed correctly
4. Verify args are accessible

**Investigation**:

```typescript
// Create custom tool
export const deploy = tool({
  description: "Deploy to environment",
  args: {
    env: tool.schema.string()
  },
  async execute(args) {
    return `Deploying to ${args.env}`;
  }
});

// Test in Cupcake plugin
"tool.execute.before": async (input, output) => {
  if (input.tool === "deploy") {
    console.log("Custom tool detected!");
    console.log("Args:", output.args);
  }
}
```

**Success Criteria**:

- [ ] Custom tools trigger plugin hooks
- [ ] Tool names are passed correctly
- [ ] Policies can route to custom tools

**Status**: 🟢 Low Priority

---

## Investigation Tracking

### Testing Framework

All research questions should follow this testing framework:

1. **Create Prototype**: Minimal plugin to test specific behavior
2. **Document Findings**: Record observations in this document
3. **Create Test Case**: Add to integration test suite if behavior is important
4. **Update Design**: Revise integration design based on findings

### Research Priorities

**Week 1**: Q1 (Context Injection), Q2 (Ask Decisions), Q3 (Arg Modification)
**Week 2**: Q4 (Performance), Q5 (Error Handling)
**Week 3**: Q6 (Multi-Agent), Others as needed

---

## Documentation Updates

As questions are answered, update these documents:

- `integration-design.md` - Update with findings
- `plugin-reference.md` - Add capabilities discovered
- `limitations.md` - Document what's not possible
- `IMPLEMENTATION_PLAN.md` - Adjust phases based on findings

---

## Status Legend

- 🔴 Not Started - Question not yet investigated
- 🟡 In Progress - Currently investigating
- 🟢 Low Priority - Can be deferred
- ✅ Completed - Question answered and documented
- ❌ Not Possible - Determined to be impossible, workaround documented

---

## Next Steps

1. **Set up test environment** with OpenCode installed
2. **Create prototype plugin** for Q1-Q3 testing
3. **Run experiments** and document findings
4. **Update integration design** based on results
5. **Proceed with Phase 1 implementation** once high-priority questions answered
