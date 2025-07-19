# Plan 016: Support Claude Code Hook Updates

Enables: Enhanced hook support with new UserPromptSubmit event and cwd field

## Goal

Update Cupcake to support the latest Claude Code hook changes, including the new UserPromptSubmit event, cwd field in all hook inputs, and improved hook configuration patterns.

## Success Criteria

- UserPromptSubmit hook event fully supported
- cwd field available in all hook event contexts
- Support for hooks without matchers (UserPromptSubmit, Notification, Stop, SubagentStop)
- Enhanced hook configuration examples
- All hook events properly handle new exit code behaviors
- Integration tests cover new functionality
- Documentation updated with examples

## Context

Anthropic has released a significant overhaul of Claude Code hooks focused on adding new features and improving the developer experience:

1. **New UserPromptSubmit event** - Major new feature that intercepts user prompts before Claude processes them
2. **cwd field** - All hooks now receive current working directory in input
3. **Matcher-less configuration** - Events that don't use tool matchers can omit the matcher field
4. **Documentation split** - Original hooks.md split into:
   - User-friendly "Get started with Claude Code hooks" guide with quickstart tutorial
   - In-depth reference documentation with enhanced debugging sections
5. **Enhanced security warnings** - Stronger emphasis on hook security implications

## Key Changes to Implement

### 1. New UserPromptSubmit Hook Event

- Runs when user submits a prompt, BEFORE Claude processes it
- Major new feature enabling:
  - Validate prompts against security or style policies
  - Block certain types of prompts
  - Dynamically add context to the prompt before the LLM sees it
- Exit code 2 blocks prompt and erases it from context
- Input includes `prompt` field with user's text
- Decision control supports blocking with reason shown to user

### 2. Updated Common Fields

All hook events now include:

```json
{
  "session_id": "string",
  "transcript_path": "string",
  "cwd": "string" // NEW: current working directory
}
```

### 3. Hook Configuration Updates

For events without tool matchers:

```json
{
  "hooks": {
    "UserPromptSubmit": [
      {
        "hooks": [
          // No matcher field needed
          {
            "type": "command",
            "command": "/path/to/script.py"
          }
        ]
      }
    ]
  }
}
```

### 4. Exit Code 2 Behavior Updates

| Hook Event         | Behavior                                                           |
| ------------------ | ------------------------------------------------------------------ |
| `UserPromptSubmit` | Blocks prompt processing, erases prompt, shows stderr to user only |

## Technical Requirements

1. **Update HookEvent enum**

   - Add UserPromptSubmit variant
   - Add prompt field to the variant

2. **Update CommonFields struct**

   - Add cwd field
   - Ensure backward compatibility

3. **Update hook event parsing**

   - Handle UserPromptSubmit JSON structure
   - Parse prompt field correctly

4. **Update response handling**

   - UserPromptSubmit exit code 2 behavior
   - Decision control for prompt blocking

5. **Policy examples**

   - Prompt validation (secrets detection)
   - Context injection based on prompt
   - Blocking inappropriate prompts

6. **Testing**
   - Unit tests for UserPromptSubmit parsing
   - Integration tests for prompt blocking
   - Test cwd field in all events

## Implementation Notes

- UserPromptSubmit doesn't use matchers (no tool involved)
- Blocked prompts are erased from Claude's context
- stdout can add context that gets prepended to the prompt
- JSON decision control allows sophisticated validation
- The cwd field enables better path-based conditions
- Enhanced debugging sections in docs: Basic Troubleshooting + Advanced Debugging
- Documentation includes practical Python examples for UserPromptSubmit validation
