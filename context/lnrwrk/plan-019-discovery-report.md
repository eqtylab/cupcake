# Cupcake Discovery Report: Claude Code July 20 Integration Analysis

Created: 2025-07-21T11:00:00Z
Type: Discovery Report

## Executive Summary

After comprehensive analysis of the Cupcake codebase, I've discovered that Cupcake is **well-positioned** to leverage Claude Code's July 20 updates. The system already has robust infrastructure for hook integration, policy evaluation, and state management. However, significant opportunities exist to enhance Cupcake's value proposition by leveraging new capabilities like context injection, permission decisions, and improved hook control.

## 1. Current State Analysis

### Hook Integration Architecture

**Current Implementation:**

- **Sync Command**: Currently a stub (`src/cli/commands/sync.rs`) - TODO implementation pending
- **Hook Registration**: TUI wizard generates stub configurations but doesn't create real Claude Code hooks
- **Hook Events**: All 7 events are defined in `HookEvent` enum, including UserPromptSubmit
- **Matcher Support**: Uses empty string `""` for non-tool events (compatible with new syntax)

**Key Findings:**

- ✅ UserPromptSubmit is already defined in the codebase
- ✅ All hook events from July 20 are present
- ❌ Sync command needs implementation to register hooks
- ❌ Hook configurations are outdated format

### Policy Execution Model

**Current Implementation:**

- **Exit Codes**: Uses 0 (allow) and 2 (block with feedback)
- **Response Types**: `PolicyDecision` enum with Allow/Block/Approve
- **JSON Support**: Basic JSON response structure exists but uses old format
- **Two-Pass System**: Sophisticated feedback aggregation already implemented

**Key Findings:**

- ✅ Exit code model aligns with Claude Code
- ✅ Two-pass evaluation perfect for new feedback model
- ❌ JSON output uses deprecated "approve"/"block" format
- ❌ No support for new "permissionDecision" fields
- ❌ No "ask" permission type

### Action System

**Current Actions:**

- `ProvideFeedback` - Soft action for feedback
- `BlockWithFeedback` - Hard action to block
- `Approve` - Hard action to approve
- `RunCommand` - Execute commands
- `UpdateState` - Modify session state
- `Conditional` - Conditional logic

**Key Findings:**

- ✅ Action system is extensible and well-designed
- ❌ No "inject_context" action type
- ❌ Actions don't map to new JSON output format

## 2. Gap Analysis

### Critical Gaps

1. **No Context Injection Capability**

   - UserPromptSubmit exists but can't inject context
   - No mechanism to add stdout to Claude's context
   - Missing "inject_context" action type

2. **Outdated JSON Response Format**

   - Still using deprecated "approve"/"block" syntax
   - No support for `permissionDecision` field
   - No support for `hookSpecificOutput` structure
   - Missing universal fields (continue/stopReason/suppressOutput)

3. **Missing Permission Model**

   - No "ask" decision type
   - Can't prompt users for confirmation
   - Binary allow/block model only

4. **No MCP Tool Support**

   - No awareness of `mcp__` naming pattern
   - Can't create policies targeting MCP tools

5. **No $CLAUDE_PROJECT_DIR Support**
   - Command executor doesn't inject this variable
   - Can't use project-relative scripts

### Architecture Limitations

1. **Hook Configuration Generation**

   - Generates old-style configurations
   - Doesn't leverage matcher flexibility
   - No support for timeout per command

2. **State Management Limitations**
   - Can't track prompt history
   - No mechanism for context-aware state
   - State queries don't support dynamic context generation

## 3. Integration Opportunities

### High-Value Quick Wins

1. **UserPromptSubmit Context Injection**

   - Add stdout → context behavior
   - Create "inject_context" action
   - Enable proactive policy guidance

2. **Update JSON Response Format**

   - Implement `permissionDecision` field
   - Add `hookSpecificOutput` support
   - Support universal control fields

3. **Add "Ask" Permission Type**
   - Extend PolicyDecision enum
   - Enable user confirmation flows
   - Improve UX for edge cases

### Medium-Term Enhancements

1. **MCP Tool Integration**

   - Pattern matching for `mcp__` tools
   - Policy templates for common MCP servers
   - Cross-server policy support

2. **Enhanced State Management**

   - Track prompts in session state
   - Build context from session history
   - Dynamic context generation

3. **Project Directory Support**
   - Inject `$CLAUDE_PROJECT_DIR` in command executor
   - Enable portable hook scripts
   - Support project-relative policies

### Long-Term Strategic Features

1. **Behavioral Guidance System**

   - Proactive context injection based on patterns
   - Learning from session history
   - Adaptive policy enforcement

2. **Multi-Modal Enforcement**
   - UserPromptSubmit for guidance
   - PreToolUse for enforcement
   - PostToolUse for learning

## 4. Technical Recommendations

### Immediate Changes (Phase 1)

1. **Update Response Handler** (`src/engine/response.rs`)

   ```rust
   pub enum PolicyDecision {
       Allow,
       Block { feedback: String },
       Approve { reason: Option<String> },
       Ask { reason: String }, // NEW
   }

   pub struct CupcakeResponse {
       // Add new fields
       pub permission_decision: Option<String>,
       pub permission_decision_reason: Option<String>,
       pub hook_specific_output: Option<HookSpecificOutput>,
   }
   ```

2. **Add Context Injection Action** (`src/config/actions.rs`)

   ```rust
   pub enum Action {
       // ... existing actions ...
       InjectContext {
           context: String,
           #[serde(default)]
           use_stdout: bool,
       },
   }
   ```

3. **Implement Sync Command** (`src/cli/commands/sync.rs`)
   - Generate proper hook configurations
   - Use new JSON format
   - Support all hook events with correct matchers

### Core Enhancements (Phase 2)

1. **Enhanced State Management**

   - Add prompt tracking to SessionState
   - Create context generation from state
   - Support dynamic context queries

2. **MCP Tool Support**

   - Update matcher logic for `mcp__` pattern
   - Add MCP-aware policy templates
   - Document MCP integration patterns

3. **Command Executor Updates**
   - Inject `$CLAUDE_PROJECT_DIR` environment variable
   - Support per-command timeouts
   - Add timeout configuration to hook registration

### Advanced Features (Phase 3)

1. **Behavioral Guidance Engine**

   - Context injection based on session patterns
   - Proactive policy suggestions
   - Learning system for effective guidance

2. **Ask Permission Flow**
   - UI/UX for confirmation dialogs
   - Reason display system
   - Fallback handling

## 5. Implementation Roadmap

### Week 1: Foundation Updates

- [ ] Update PolicyDecision enum with Ask variant
- [ ] Implement new JSON response format
- [ ] Add InjectContext action type
- [ ] Update response handler for new fields

### Week 2: Hook Integration

- [ ] Implement sync command properly
- [ ] Generate new-format hook configurations
- [ ] Add UserPromptSubmit context injection
- [ ] Test with Claude Code

### Week 3: State & Context

- [ ] Enhance state management for prompts
- [ ] Build context generation system
- [ ] Implement dynamic context injection
- [ ] Add session-aware policies

### Week 4: MCP & Advanced Features

- [ ] Add MCP tool pattern support
- [ ] Implement $CLAUDE_PROJECT_DIR injection
- [ ] Create behavioral guidance templates
- [ ] Documentation and examples

## Conclusion

Cupcake's architecture is fundamentally sound and ready for enhancement. The July 20 Claude Code updates don't diminish Cupcake's value - they amplify it. By implementing context injection, the new permission model, and MCP support, Cupcake can transform from a policy enforcer into an intelligent behavioral guidance system.

The most critical enhancement is UserPromptSubmit context injection, which enables Cupcake to proactively shape AI behavior rather than just reactively blocking. This single feature transforms the entire value proposition.

With these changes, Cupcake becomes not just a guardrail, but a guide - making AI agents more effective while keeping them safe.
