# Type-Safe Response Builder Refactoring Analysis

## Current Problem

The `ContextInjectionResponseBuilder` (lines 54-61 in `context_injection.rs`) handles both `UserPromptSubmit` and `SessionStart` events with a boolean flag `is_user_prompt_submit`. When an `Ask` decision arrives, it creates a response with the **wrong event type**:

```rust
EngineDecision::Ask { reason } => {
    // BUG: Always creates UserPromptSubmit, even for SessionStart
    tracing::warn!("Ask action not supported for UserPromptSubmit - treating as Allow with context");
    response.hook_specific_output = Some(HookSpecificOutput::UserPromptSubmit {
        additional_context: Some(reason.clone()),
    });
}
```

**Root Cause**: The function accepts `EngineDecision::Ask` but neither UserPromptSubmit nor SessionStart support Ask decisions according to Claude Code spec.

## Claude Code Event Capabilities Matrix

| Event Type           | Allow | Block | Ask | Context Injection |
|---------------------|-------|-------|-----|-------------------|
| **PreToolUse**      | ✅    | ✅    | ✅  | ❌                |
| **PostToolUse**     | ✅    | ✅    | ❌  | ❌                |
| **Stop**            | ✅    | ✅    | ❌  | ❌                |
| **SubagentStop**    | ✅    | ✅    | ❌  | ❌                |
| **UserPromptSubmit**| ✅    | ✅    | ❌  | ✅                |
| **SessionStart**    | ✅    | ❌    | ❌  | ✅                |
| **PreCompact**      | ✅    | ❌    | ❌  | ✅                |
| **Notification**    | ✅    | ❌    | ❌  | ❌                |

## Current Architecture

```
ClaudeCodeResponseBuilder (dispatcher)
    ├── PreToolUseResponseBuilder     → Allow/Block/Ask
    ├── FeedbackLoopResponseBuilder   → Allow/Block (PostToolUse, Stop, SubagentStop)
    ├── ContextInjectionResponseBuilder → Allow/Block/Ask (UserPromptSubmit, SessionStart)
    ├── GenericResponseBuilder        → Allow (Notification)
    └── GenericResponseBuilder::precompact → Allow (PreCompact)
```

**Problems**:
1. `ContextInjectionResponseBuilder` accepts `Ask` but shouldn't
2. Boolean flag `is_user_prompt_submit` hides which event type we're handling
3. No compile-time prevention of invalid decision/event combinations
4. SessionStart has different blocking rules than UserPromptSubmit (can't block at all!)

## Proposed Type-Safe Architecture

### Step 1: Define Decision Capability Types

```rust
/// Decisions that can block tool execution
pub enum ToolDecision {
    Allow { reason: Option<String> },
    Deny { feedback: String },
    Ask { reason: String },
}

/// Decisions for feedback loops (post-execution)
pub enum FeedbackDecision {
    Allow { reason: Option<String> },
    Block { feedback: String },
}

/// Decisions for context-only events (read-only)
pub enum ContextDecision {
    Allow { reason: Option<String> },
}

/// Decisions for prompt processing
pub enum PromptDecision {
    Allow { reason: Option<String> },
    Block { feedback: String },
}
```

### Step 2: Create Type-Safe Builders

```rust
// cupcake-core/src/harness/response/claude_code/tool_event.rs
pub struct ToolEventResponseBuilder;

impl ToolEventResponseBuilder {
    /// Build response for PreToolUse (supports Ask)
    pub fn build_pre_tool_use(
        decision: &ToolDecision,
        suppress_output: bool,
    ) -> CupcakeResponse {
        match decision {
            ToolDecision::Allow { .. } => {
                // permissionDecision: "allow"
            }
            ToolDecision::Deny { feedback } => {
                // permissionDecision: "deny"
            }
            ToolDecision::Ask { reason } => {
                // permissionDecision: "ask"
            }
        }
    }
}
```

```rust
// cupcake-core/src/harness/response/claude_code/feedback_loop.rs
pub struct FeedbackLoopResponseBuilder;

impl FeedbackLoopResponseBuilder {
    /// Build response for PostToolUse/Stop/SubagentStop
    pub fn build(
        decision: &FeedbackDecision,
        suppress_output: bool,
    ) -> CupcakeResponse {
        match decision {
            FeedbackDecision::Allow { .. } => {
                // Empty response (allow by default)
            }
            FeedbackDecision::Block { feedback } => {
                // decision: "block", reason: feedback
            }
        }
        // Ask is impossible here - compile error if attempted
    }
}
```

```rust
// cupcake-core/src/harness/response/claude_code/user_prompt.rs
pub struct UserPromptResponseBuilder;

impl UserPromptResponseBuilder {
    /// Build response for UserPromptSubmit
    pub fn build(
        decision: &PromptDecision,
        context_to_inject: Option<Vec<String>>,
        suppress_output: bool,
    ) -> CupcakeResponse {
        match decision {
            PromptDecision::Allow { .. } => {
                // Add context if provided
                if let Some(contexts) = context_to_inject {
                    response.hook_specific_output = Some(HookSpecificOutput::UserPromptSubmit {
                        additional_context: Some(contexts.join("\n")),
                    });
                }
            }
            PromptDecision::Block { feedback } => {
                // decision: "block", reason: feedback
            }
        }
        // Ask is impossible - type system prevents it
    }
}
```

```rust
// cupcake-core/src/harness/response/claude_code/session_start.rs
pub struct SessionStartResponseBuilder;

impl SessionStartResponseBuilder {
    /// Build response for SessionStart (context injection only, cannot block)
    pub fn build(
        decision: &ContextDecision,
        context_to_inject: Option<Vec<String>>,
        suppress_output: bool,
    ) -> CupcakeResponse {
        // SessionStart can ONLY inject context, never block
        match decision {
            ContextDecision::Allow { .. } => {
                if let Some(contexts) = context_to_inject {
                    response.hook_specific_output = Some(HookSpecificOutput::SessionStart {
                        additional_context: Some(contexts.join("\n")),
                    });
                }
            }
        }
        // Block and Ask are impossible - type system prevents them
    }
}
```

### Step 3: Update Decision Synthesis

The engine's synthesis layer needs to convert from unified `PolicyDecision` to event-specific decision types:

```rust
// In cupcake-core/src/engine/synthesis.rs or similar

impl Engine {
    /// Convert PolicyDecision to appropriate decision type for the event
    fn synthesize_for_event(
        &self,
        policy_decision: &PolicyDecision,
        event: &ClaudeCodeEvent,
    ) -> Result<EventDecision> {
        match event {
            ClaudeCodeEvent::PreToolUse(_) => {
                let tool_decision = match policy_decision {
                    PolicyDecision::Allow => ToolDecision::Allow { reason: None },
                    PolicyDecision::Deny(msg) => ToolDecision::Deny { feedback: msg.clone() },
                    PolicyDecision::Ask(msg) => ToolDecision::Ask { reason: msg.clone() },
                    PolicyDecision::Halt(msg) => ToolDecision::Deny { feedback: msg.clone() },
                };
                Ok(EventDecision::Tool(tool_decision))
            }

            ClaudeCodeEvent::PostToolUse(_)
            | ClaudeCodeEvent::Stop(_)
            | ClaudeCodeEvent::SubagentStop(_) => {
                let feedback_decision = match policy_decision {
                    PolicyDecision::Allow => FeedbackDecision::Allow { reason: None },
                    PolicyDecision::Deny(msg) | PolicyDecision::Halt(msg) => {
                        FeedbackDecision::Block { feedback: msg.clone() }
                    }
                    PolicyDecision::Ask(_) => {
                        // Ask not supported - log warning and treat as Allow
                        tracing::warn!(
                            "Ask decision not supported for {} - treating as Allow",
                            event.event_name()
                        );
                        FeedbackDecision::Allow { reason: None }
                    }
                };
                Ok(EventDecision::Feedback(feedback_decision))
            }

            ClaudeCodeEvent::UserPromptSubmit(_) => {
                let prompt_decision = match policy_decision {
                    PolicyDecision::Allow => PromptDecision::Allow { reason: None },
                    PolicyDecision::Deny(msg) | PolicyDecision::Halt(msg) => {
                        PromptDecision::Block { feedback: msg.clone() }
                    }
                    PolicyDecision::Ask(_) => {
                        tracing::warn!("Ask decision not supported for UserPromptSubmit");
                        PromptDecision::Allow { reason: None }
                    }
                };
                Ok(EventDecision::Prompt(prompt_decision))
            }

            ClaudeCodeEvent::SessionStart(_) => {
                let context_decision = match policy_decision {
                    PolicyDecision::Allow => ContextDecision::Allow { reason: None },
                    PolicyDecision::Deny(msg)
                    | PolicyDecision::Halt(msg)
                    | PolicyDecision::Ask(msg) => {
                        // SessionStart cannot block - only inject context
                        tracing::warn!(
                            "SessionStart cannot block - treating as Allow. Message logged: {}",
                            msg
                        );
                        ContextDecision::Allow { reason: None }
                    }
                };
                Ok(EventDecision::Context(context_decision))
            }

            ClaudeCodeEvent::PreCompact(_) | ClaudeCodeEvent::Notification(_) => {
                // These events only support Allow
                Ok(EventDecision::Context(ContextDecision::Allow { reason: None }))
            }
        }
    }
}

/// Event-specific decision types
pub enum EventDecision {
    Tool(ToolDecision),
    Feedback(FeedbackDecision),
    Prompt(PromptDecision),
    Context(ContextDecision),
}
```

### Step 4: Updated Dispatcher

```rust
impl ClaudeCodeResponseBuilder {
    pub fn build_response(
        event_decision: EventDecision,
        hook_event: &ClaudeCodeEvent,
        context_to_inject: Option<Vec<String>>,
        suppress_output: bool,
    ) -> CupcakeResponse {
        match (event_decision, hook_event) {
            (EventDecision::Tool(decision), ClaudeCodeEvent::PreToolUse(_)) => {
                ToolEventResponseBuilder::build_pre_tool_use(&decision, suppress_output)
            }

            (EventDecision::Feedback(decision), ClaudeCodeEvent::PostToolUse(_))
            | (EventDecision::Feedback(decision), ClaudeCodeEvent::Stop(_))
            | (EventDecision::Feedback(decision), ClaudeCodeEvent::SubagentStop(_)) => {
                FeedbackLoopResponseBuilder::build(&decision, suppress_output)
            }

            (EventDecision::Prompt(decision), ClaudeCodeEvent::UserPromptSubmit(_)) => {
                UserPromptResponseBuilder::build(&decision, context_to_inject, suppress_output)
            }

            (EventDecision::Context(decision), ClaudeCodeEvent::SessionStart(_)) => {
                SessionStartResponseBuilder::build(&decision, context_to_inject, suppress_output)
            }

            (EventDecision::Context(decision), ClaudeCodeEvent::PreCompact(_)) => {
                PreCompactResponseBuilder::build(&decision, context_to_inject, suppress_output)
            }

            (EventDecision::Context(_), ClaudeCodeEvent::Notification(_)) => {
                NotificationResponseBuilder::build(suppress_output)
            }

            // This match is exhaustive - any mismatch is a compile error
            _ => unreachable!("Invalid decision/event combination - should be caught by synthesis"),
        }
    }
}
```

## Benefits of Type-Safe Approach

### 1. Compile-Time Safety
```rust
// ❌ COMPILE ERROR - Cannot create Ask decision for SessionStart
let decision = ContextDecision::Ask { reason: "test".to_string() };
// Error: no variant named `Ask` in enum `ContextDecision`

// ✅ OK - Can only create valid decisions
let decision = ContextDecision::Allow { reason: None };
```

### 2. Impossible States Become Unrepresentable

The current bug (creating `UserPromptSubmit` output for `SessionStart` event with `Ask` decision) becomes **impossible** because:
- `ContextDecision` enum doesn't have `Ask` variant
- `SessionStartResponseBuilder` only accepts `ContextDecision`
- Type checker enforces this at compile time

### 3. Self-Documenting Code

```rust
// Current (unclear what's valid):
fn build(decision: &EngineDecision, ..., is_user_prompt_submit: bool)

// Type-safe (crystal clear):
fn build_session_start(decision: &ContextDecision, ...)
// ^ Can only pass ContextDecision - Ask is impossible
```

### 4. Better Error Messages

Instead of runtime warnings:
```
tracing::warn!("Ask action not supported for UserPromptSubmit - treating as Allow with context");
```

You get compile-time errors:
```
error[E0308]: mismatched types
  --> src/engine/mod.rs:123:45
   |
   | SessionStartResponseBuilder::build(&tool_decision, ...)
   |                                     ^^^^^^^^^^^^^^ expected `ContextDecision`, found `ToolDecision`
```

### 5. Prevents Accidental Bugs in Future Code

New developers can't make the same mistake - the compiler stops them:
```rust
// New developer tries to add Ask support to SessionStart
impl SessionStartResponseBuilder {
    pub fn build(decision: &ContextDecision, ...) {
        match decision {
            ContextDecision::Allow { .. } => { ... }
            ContextDecision::Ask { .. } => { ... }  // ❌ COMPILE ERROR
        }
    }
}
```

## Migration Path

### Phase 1: Add new decision types (non-breaking)
- Define `ToolDecision`, `FeedbackDecision`, `PromptDecision`, `ContextDecision`
- Keep `EngineDecision` for backwards compatibility

### Phase 2: Add new builders (non-breaking)
- Create `UserPromptResponseBuilder`
- Create `SessionStartResponseBuilder`
- Keep old `ContextInjectionResponseBuilder` for now

### Phase 3: Update synthesis layer
- Convert from `PolicyDecision` → event-specific decision types
- Add warnings for invalid combinations

### Phase 4: Switch dispatcher
- Update `ClaudeCodeResponseBuilder` to use new builders
- Remove old `ContextInjectionResponseBuilder`

### Phase 5: Remove old types
- Delete `EngineDecision` enum
- Update all callers

## Testing Strategy

### 1. Compile-Time Tests
The type system itself provides testing - invalid combinations won't compile.

### 2. Runtime Tests
```rust
#[test]
fn test_session_start_only_allows_context_injection() {
    let decision = ContextDecision::Allow { reason: None };
    let context = vec!["Session context".to_string()];

    let response = SessionStartResponseBuilder::build(
        &decision,
        Some(context),
        false
    );

    // Verify only context injection, no blocking
    match response.hook_specific_output {
        Some(HookSpecificOutput::SessionStart { additional_context }) => {
            assert_eq!(additional_context, Some("Session context".to_string()));
        }
        _ => panic!("Expected SessionStart output"),
    }
    assert_eq!(response.decision, None); // Cannot block
    assert_eq!(response.continue_execution, None);
}
```

### 3. Integration Tests
Test the full flow from policy decision to response:
- PolicyDecision::Ask + SessionStart → ContextDecision::Allow (with warning)
- PolicyDecision::Ask + PreToolUse → ToolDecision::Ask
- PolicyDecision::Ask + PostToolUse → FeedbackDecision::Allow (with warning)

## Conclusion

**Should we do this refactoring?**

**✅ YES** - This is exactly the kind of architectural improvement that makes bugs impossible rather than just unlikely.

**Effort**: Medium (2-3 hours)
- Create 4 new decision enums
- Split `ContextInjectionResponseBuilder` into separate builders
- Update synthesis layer
- Add tests

**Value**: HIGH
- Eliminates entire class of bugs
- Makes code self-documenting
- Prevents future mistakes
- No runtime performance cost (zero-cost abstraction)

**Risk**: LOW
- Can be done incrementally
- Changes are localized to response module
- Type checker verifies correctness
- Existing tests will catch any issues

The reviewer was on the right track but didn't go far enough. The real fix isn't just parameterizing the event type - it's **making invalid combinations impossible to express in the type system**.
