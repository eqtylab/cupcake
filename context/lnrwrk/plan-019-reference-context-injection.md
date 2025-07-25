# Context Injection Reference - Claude Code July 20

Created: 2025-01-25T12:00:00Z
Type: Technical Reference for Implementation

## What is Context Injection?

Context injection is the ability to add information to Claude's conversation context **before** Claude processes a user's prompt. This feature is unique to the UserPromptSubmit hook event.

## How It Works

### Method 1: Simple stdout (UserPromptSubmit Only)
```bash
#!/bin/bash
# When exit code is 0, stdout goes to Claude's context
echo "PROJECT STATUS: 5 tests failing, last commit broke build"
echo "CODING STANDARD: Use 4-space indentation"
exit 0
```

### Method 2: JSON hookSpecificOutput
```json
{
  "hookSpecificOutput": {
    "hookEventName": "UserPromptSubmit",
    "additionalContext": "PROJECT STATUS: 5 tests failing\nCODING STANDARD: Use 4-space indentation"
  }
}
```

Both methods achieve identical results - the text is added to Claude's context before processing the prompt.

## Why Anthropic Added This

### Documented Purpose
From the official docs: *"add additional context based on the prompt/conversation, validate prompts, or block certain types of prompts"*

### Design Intent (Inferred)
1. **Dynamic Context Enhancement** - Add relevant information based on user's query
2. **Session-Aware Guidance** - Inject context from conversation history
3. **Proactive Error Prevention** - Remind Claude of rules before mistakes
4. **Workflow Continuity** - Maintain context without user repetition

### The "Whisper in Claude's Ear" Pattern
- User doesn't see injected context (invisible augmentation)
- Shapes Claude's response before generation
- Enables dynamic adaptation based on session state

## Implementation Requirements for Cupcake

### 1. Add InjectContext Action
```rust
pub enum Action {
    // ... existing actions ...
    InjectContext {
        context: String,
        #[serde(default)]
        use_stdout: bool,  // true = stdout method, false = JSON method
    },
}
```

### 2. Update Response Handler
For UserPromptSubmit events when using InjectContext action:
- If `use_stdout`: Print context to stdout, exit 0
- If not: Generate JSON with hookSpecificOutput

### 3. Context Generation Engine
Build dynamic context based on:
- Session state (recent violations, tool usage)
- Prompt analysis (keywords, patterns)
- Project state (test results, build status)
- User patterns (common mistakes)

## Use Cases for Cupcake

### 1. Violation Reminder
```yaml
UserPromptSubmit:
  "":
    - name: "Inject recent violations"
      conditions:
        - type: "state_exists"
          query: "recent_policy_violations"
      action:
        type: "inject_context"
        context: "Recent issues: Missing tests, unsafe command usage"
```

### 2. Project Status Context
```yaml
UserPromptSubmit:
  "":
    - name: "Add project status"
      conditions:
        - type: "pattern"
          field: "prompt"
          regex: "fix|debug|help"
      action:
        type: "inject_context"
        context: "Current build status: FAILING\nTest coverage: 67%"
```

### 3. Coding Standards Reminder
```yaml
UserPromptSubmit:
  "":
    - name: "Coding standards context"
      conditions:
        - type: "pattern"
          field: "prompt"
          regex: "write|create|implement"
      action:
        type: "inject_context"
        context: "Remember: Use TypeScript, 4-space indents, comprehensive tests"
```

## Technical Considerations

### Exit Code Behavior
- **Exit 0 + stdout** = Context injection (UserPromptSubmit only)
- **Exit 2 + stderr** = Block with feedback
- **Other hooks**: stdout goes to transcript, not context

### JSON vs stdout Trade-offs
- **stdout method**: Simple, direct, easy to implement
- **JSON method**: More control, composable with other fields, structured

### Performance Impact
- Context injection happens synchronously
- Keep injected context concise
- Consider caching frequently used context

## Security Implications

1. **Information Leakage**: Injected context could reveal sensitive project info
2. **Context Pollution**: Too much context could confuse Claude
3. **Prompt Manipulation**: Malicious context could influence Claude's behavior

## Integration Strategy

### Phase 1: Basic Context Injection
- Implement InjectContext action
- Support stdout method only
- Static context strings

### Phase 2: Dynamic Context
- Add template support with variables
- Pull context from session state
- Support JSON method

### Phase 3: Intelligent Context
- Analyze prompts for context needs
- Build context from multiple sources
- Learn from effectiveness

## Success Metrics

1. **Reduced Violations**: Fewer policy violations after context injection
2. **Better First Attempts**: Claude gets it right first time more often
3. **Less User Repetition**: Users don't need to repeat context
4. **Improved Productivity**: Faster, more accurate responses

## Conclusion

Context injection transforms Cupcake from a reactive enforcer to a proactive guide. It's the single most important feature from the July 20 updates because it enables Cupcake to shape AI behavior before actions are taken, not just block bad actions after the fact.