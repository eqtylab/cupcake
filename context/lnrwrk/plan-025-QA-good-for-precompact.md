That is an outstanding and critical follow-up question. The previous implementation log for `PreCompact` is a perfect stress test for our new plan.

**The short answer is: Yes, absolutely.**

The proposed refactor doesn't just accommodate the nuances of the `PreCompact` hook; it makes handling them dramatically cleaner, safer, and more explicit. The previous implementation was a patch that increased complexity. The new architecture provides a proper, systematic home for this special-case logic.

Let's walk through exactly how the `PreCompact` implementation will be improved and clarified by this new plan.

---

### **How the New Architecture Solves the `PreCompact` Problem**

The core issue the developer faced was that the `PreCompact` hook has a unique output contract: its "context injection" is not handled via JSON, but by printing raw text to `stdout`. The old architecture forced a messy, special-case `match` arm in the high-level `run/mod.rs` command handler.

Here is how the new plan elegantly solves this by tying behavior directly to the types.

#### **1. Centralizing the "Context Injection" Logic with a Trait**

Instead of fragile string comparisons (`hook_event.event_name() == "PreCompact"`), we will use Rust's trait system to formally declare which hooks support this behavior.

```rust
// In src/engine/events/claude_code/mod.rs

// ... (EventPayload trait, CommonEventData struct, ClaudeCodeEvent enum) ...

/// A marker trait for events that can inject context via stdout.
pub trait InjectsContext {}
```

Now, we explicitly implement this trait for the specific hook payloads that support it. This is self-documenting and type-safe.

```rust
// In src/engine/events/claude_code/pre_compact.rs
use super::{InjectsContext, EventPayload, CommonEventData};
// ... (PreCompactPayload struct definition) ...

impl EventPayload for PreCompactPayload { /* ... */ }
impl InjectsContext for PreCompactPayload {} // We declare its special capability here.

// In src/engine/events/claude_code/user_prompt_submit.rs
use super::{InjectsContext, EventPayload, CommonEventData};
// ... (UserPromptSubmitPayload struct definition) ...

impl EventPayload for UserPromptSubmitPayload { /* ... */ }
impl InjectsContext for UserPromptSubmitPayload {} // This one too.
```

#### **2. Simplifying the Engine and Command Handler**

The logic that was previously scattered across `run/engine.rs` and `run/mod.rs` can now be consolidated and clarified.

**Step A: The Engine (`run/engine.rs`)**

The engine's job is simplified. It no longer needs to know the names of the context-injecting events. It just needs to check if the action is `InjectContext`.

```rust
// In src/cli/commands/run/engine.rs (inside the `run` function)

// ... after executing actions ...

for (_policy_name, result) in action_results.iter() {
    match result {
        // This logic becomes simpler. We just check the action type.
        ActionResult::Success { feedback: Some(ctx), is_context_injection: true } => {
            context_to_inject.push(ctx.clone());
        }
        // ... other ActionResult arms ...
    }
}
```

_(Note: We'll need to slightly modify `ActionResult` to carry the `is_context_injection` boolean from the `ActionExecutor`'s evaluation of the `InjectContext` action.)_

**Step B: The Command Handler (`run/mod.rs`)**

The `run` command handler becomes the single, authoritative place to decide _how_ to render the output for different hook types. The messy `match` on strings is replaced by a much cleaner `match` on the typed enum variants.

```rust
// In src/cli/commands/run/mod.rs (inside the `execute` function)

// ... after engine.run() ...

// The new, clean way to handle hook-specific output:
match hook_event {
    // We can group all context-injecting hooks together.
    AgentEvent::ClaudeCode(ClaudeCodeEvent::PreCompact(_)) |
    AgentEvent::ClaudeCode(ClaudeCodeEvent::UserPromptSubmit(_)) |
    AgentEvent::ClaudeCode(ClaudeCodeEvent::SessionStart(_)) => {
        if !result.context_to_inject.is_empty() {
            // The logic is now unified. The developer discovered PreCompact
            // joins with \n\n, while others might join with \n. We can
            // handle that nuance right here.
            let separator = if matches!(hook_event, AgentEvent::ClaudeCode(ClaudeCodeEvent::PreCompact(_))) {
                "\n\n"
            } else {
                "\n"
            };
            let combined_context = result.context_to_inject.join(separator);
            println!("{combined_context}");
            std::process::exit(0);
        }
        // If there's no context to inject, we fall through to the default handler.
    }
    _ => {} // Fall through for non-injecting events.
}

// Default handler for all other hooks that use the standard JSON response.
ResponseHandler::new(self.debug).send_response_for_hook_with_suppress(
    result.final_decision,
    hook_event, // Pass the whole event for more robust handling
    result.suppress_output,
);
```

#### **3. The Verifiability and Clarity Win**

This new approach directly addresses the `PreCompact` nuances and makes the system better:

1.  **Discoverability:** A new developer wondering "Which hooks can inject context?" doesn't need to read the entire `run/mod.rs` file. They can simply search for `impl InjectsContext` and get an immediate, authoritative answer. The code itself becomes the documentation.

2.  **Clarity:** The special `\n\n` joiner for `PreCompact`, which the developer had to discover through reverse-engineering, is now encoded in a single, obvious place right next to the event type it applies to. It's no longer magic behavior hidden in a generic `join("\n")`.

3.  **Reduced Risk:** When a new context-injecting hook is added in the future, the compiler will not force changes everywhere. The developer will simply implement the `InjectsContext` trait for the new payload and add a new arm to the `match` statement in `run/mod.rs`. The logic is contained and the risk of breaking other hooks is near zero.

**In summary, the proposed refactor is not just compatible with the `PreCompact` findings; it is the key to managing them professionally.** It takes the developer's hard-won knowledge from reverse-engineering the SDK and encodes it into a clear, type-safe, and maintainable structure, ensuring that knowledge is never lost and that future work is built on a solid foundation.
