# Support Latest Claude Code Hook Updates

## Summary

Anthropic has released a significant overhaul of Claude Code hooks focused on adding new features and improving the developer experience. The updates include a new UserPromptSubmit event, cwd field for all hooks, and a complete documentation restructuring. Cupcake needs to be updated to support these new features.

## Problem

Cupcake currently supports the original six hook events (PreToolUse, PostToolUse, Notification, Stop, SubagentStop, PreCompact) but is missing support for:

1. **UserPromptSubmit hook event** - A major new feature that fires when users submit prompts, before Claude processes them
2. **cwd field** - All hook events now include the current working directory in their input
3. **Updated hook configuration patterns** - Events without tool matchers can now omit the matcher field
4. **Documentation improvements** - Original hooks.md split into user-friendly guide and technical reference

## Requirements

### 1. Add UserPromptSubmit Hook Event Support

The UserPromptSubmit event is a major new feature that allows hooks to:
- Validate prompts against security or style policies
- Block certain types of prompts containing sensitive information  
- Dynamically add context to the prompt before the LLM sees it
- Implement organizational prompt policies

**Input Schema:**
```json
{
  "session_id": "abc123",
  "transcript_path": "...",
  "cwd": "/Users/...",
  "hook_event_name": "UserPromptSubmit",
  "prompt": "Write a function to calculate factorial"
}
```

**Exit Code Behavior:**
- Exit code 2: Blocks prompt processing, erases prompt from context, shows stderr to user only
- Exit code 0 with stdout: Adds context to prompt before processing

**Decision Control:**
```json
{
  "decision": "block" | undefined,
  "reason": "Explanation shown to user"
}
```

### 2. Add cwd Field to All Hook Events

All hook events now include the current working directory:
```json
{
  "session_id": "string",
  "transcript_path": "string",
  "cwd": "string"  // NEW field
}
```

This enables:
- Better path-based policy conditions
- Context-aware policy decisions
- Improved debugging capabilities

### 3. Support Matcher-less Hook Configuration

For events that don't use tool matchers (UserPromptSubmit, Notification, Stop, SubagentStop), the configuration can omit the matcher field:

```json
{
  "hooks": {
    "UserPromptSubmit": [
      {
        "hooks": [  // No "matcher" field
          {
            "type": "command",
            "command": "/path/to/validator.py"
          }
        ]
      }
    ]
  }
}
```

## Implementation Tasks

- [ ] Update `HookEvent` enum to include `UserPromptSubmit` variant
- [ ] Add `prompt` field to UserPromptSubmit variant structure  
- [ ] Update `CommonFields` struct to include `cwd` field
- [ ] Update hook event JSON parsing to handle UserPromptSubmit
- [ ] Implement UserPromptSubmit-specific response handling
- [ ] Update sync command to generate proper hook configurations
- [ ] Add policy examples for prompt validation use cases
- [ ] Write unit tests for new event parsing
- [ ] Write integration tests for prompt blocking scenarios
- [ ] Update documentation with UserPromptSubmit examples

## Example Use Cases

### 1. Block Prompts with Secrets
```yaml
UserPromptSubmit:
  - name: "Block Secrets in Prompts"
    conditions:
      - type: "pattern"
        field: "prompt"
        regex: "(?i)\\b(password|secret|key|token)\\s*[:=]"
    action:
      type: "block_with_feedback"
      feedback_message: "Security violation: Remove sensitive information from your prompt"
```

### 2. Add Context Based on Prompt
```yaml
UserPromptSubmit:
  - name: "Add Project Context"
    conditions:
      - type: "pattern"
        field: "prompt"
        regex: "(?i)implement|create|build"
    action:
      type: "run_command"
      command: "echo 'Project uses Rust 1.75 with async/await patterns'"
```

### 3. Enforce Prompt Guidelines
```yaml
UserPromptSubmit:
  - name: "Enforce Clear Instructions"
    conditions:
      - type: "check"
        field: "prompt_length"
        operator: "less_than"
        value: 10
    action:
      type: "block_with_feedback"
      feedback_message: "Please provide more detailed instructions"
```

## Testing

1. **Unit Tests**
   - Parse UserPromptSubmit event JSON correctly
   - Validate cwd field in all event types
   - Test decision control for prompt blocking

2. **Integration Tests**
   - Block prompts with exit code 2
   - Add context with stdout
   - JSON decision control
   - Verify prompt is erased when blocked

3. **End-to-End Tests**
   - Full workflow with cupcake run --hook UserPromptSubmit
   - Verify Claude Code integration behavior

## Key Documentation Changes

The Claude Code hooks documentation has been restructured:
- **New Guide**: "Get started with Claude Code hooks" - User-friendly guide with quickstart tutorial
- **Enhanced Reference**: Original hooks.md updated with:
  - UserPromptSubmit details and Python examples
  - Structured debugging section (Basic + Advanced)
  - Improved organization and examples

## References

- [Claude Code Hooks Reference](https://docs.anthropic.com/en/docs/claude-code/hooks)
- [Get Started with Claude Code Hooks](https://docs.anthropic.com/en/docs/claude-code/hooks-guide)
- Original hook documentation in `context/claude-code-docs/`
- Updated documentation in `context/claude-code-docs/july18-2025/`