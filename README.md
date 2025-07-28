<p align="left">
  <picture>
    <source srcset="assets/cupcake-dark.png" media="(prefers-color-scheme: dark)">
    <img src="assets/cupcake.png" alt="Cupcake logo" width="180">
  </picture>
</p>

# Cupcake - Agent Governance as Code

Policy enforcement engine that makes AI coding agents actually follow your rules, now with behavioral guidance capabilities.

> **Note**: Currently in beta with Claude Code support. The policy format is designed to be agent-agnostic, with eventual support for any coding agent hook system.

## Overview

Cupcake transforms natural language rules from CLAUDE.md files into deterministic YAML policies enforced through Claude Code's hooks system. Beyond simple enforcement, it provides behavioral guidance through context injection, enabling Claude to understand and follow complex workflows autonomously.

## Core Features

- **Behavioral Guidance**: Inject context and reminders directly into Claude's awareness
- **Project-Specific Policies**: Support for $CLAUDE_PROJECT_DIR for multi-project setups
- **MCP Tool Support**: Pattern matching for Model Context Protocol tools
- **Two-Pass Evaluation**: Aggregates all feedback before decisions
- **Performance Optimized**: Sub-100ms response times with compiled patterns

## Policy Configuration

Policies are defined in YAML format using the guardrails structure:

```yaml
# guardrails/cupcake.yaml - Root configuration
settings:
  debug_mode: false

imports:
  - "policies/*.yaml"
```

```yaml
# guardrails/policies/git-workflow.yaml - Policy fragments
PreToolUse:
  "Bash":
    - name: "Require passing tests before commit"
      description: "Block git commits when tests fail"
      conditions:
        - type: "pattern"
          field: "tool_input.command"
          regex: "^git\\s+commit"
        - type: "check"
          spec:
            mode: array
            command: ["cargo"]
            args: ["test", "--quiet"]
          expect_success: false
      action:
        type: "block_with_feedback"
        feedback_message: "Tests must pass before committing"
        include_context: true

  "Write|Edit":
    - name: "Read architecture first"
      description: "Enforce reading architecture before engine edits"
      conditions:
        - type: "pattern"
          field: "tool_input.file_path"
          regex: "^src/engine/"
        - type: "check"
          spec:
            mode: array
            command: ["test"]
            args: ["-f", "docs/architecture.md"]
          expect_success: true
      action:
        type: "block_with_feedback"
        feedback_message: "Read docs/architecture.md before editing engine"
        include_context: true

# guardrails/policies/prompt-security.yaml - UserPromptSubmit policies
UserPromptSubmit:
  "":  # Empty string or "*" matcher required in YAML format
    - name: "Block API keys in prompts"
      description: "Prevent accidental exposure of secrets"
      conditions:
        - type: "pattern"
          field: "prompt"
          regex: "(api[_-]?key|token|secret)\\s*[:=]\\s*[a-zA-Z0-9_-]{16,}"
      action:
        type: "block_with_feedback"
        feedback_message: "Detected potential secret in prompt!"
        include_context: false
```

### String Commands and Shell Execution

Beyond the exec array form for commands, Cupcake supports two other command execution modes:

```yaml
# String mode: Shell-like syntax parsed into secure commands
- type: "check"
  spec:
    mode: string
    command: "git diff --quiet && git diff --cached --quiet"
  expect_success: false

# Shell mode: Direct shell execution (requires allow_shell: true in settings)
- type: "run_command"
  spec:
    mode: shell
    script: |
      # Complex shell script with pipes, redirects, etc.
      find . -name "*.rs" | xargs cargo fmt --check
  on_failure: "block"
```

## Action Types

Cupcake supports several action types for different policy enforcement strategies:

### Hard Actions (Stop Policy Evaluation)

- **`allow`**: Explicitly permit the operation with optional reason
  ```yaml
  action:
    type: "allow"
    reason: "Safe operation - tests passed"
  ```

- **`ask`**: Request user confirmation before proceeding
  ```yaml
  action:
    type: "ask"
    reason: "Please confirm this {{tool_name}} operation"
  ```

- **`block_with_feedback`**: Block operation and provide feedback to Claude
  ```yaml
  action:
    type: "block_with_feedback"
    feedback_message: "Operation blocked: {{reason}}"
    include_context: true
  ```

### Soft Actions (Continue Policy Evaluation)

- **`provide_feedback`**: Provide transcript-only feedback
  ```yaml
  action:
    type: "provide_feedback"
    message: "Consider using {{tool_name}} carefully"
    include_context: false
  ```

- **`inject_context`**: Add context to Claude's awareness
  ```yaml
  action:
    type: "inject_context"
    context: "Remember to follow coding standards"
    use_stdout: true
  ```

- **`run_command`**: Execute commands with conditional blocking
  ```yaml
  action:
    type: "run_command"
    spec:
      mode: array
      command: ["cargo", "fmt", "--check"]
    on_failure: "block"
    on_failure_feedback: "Code must be formatted"
  ```

## Architecture

- **CLI Binary**: Single Rust executable with init, sync, run, validate commands
- **Hook Integration**: Registers with Claude Code's lifecycle events
- **Policy Cache**: Binary serialization for fast loading
- **Two-pass evaluation**: Collects all feedback, then checks for blocks

## Installation

```bash
cargo install --path .
```

## Usage

1. **Initialize policies from CLAUDE.md:**

   ```bash
   cupcake init
   ```

2. **Sync with Claude Code hooks:**

   ```bash
   cupcake sync
   ```

3. **Inspect loaded policies:**

   ```bash
   cupcake inspect
   # Or with specific config file
   cupcake inspect --config my-policy.yaml
   ```

## Advanced Features

### Behavioral Guidance with Context Injection

Cupcake can inject context directly into Claude's prompt processing, providing gentle guidance without blocking. This feature supports two modes to match your workflow:

**Simple Mode (Default)** - Lightweight Unix-style injection:
```yaml
UserPromptSubmit:
  "*":
    - name: test-reminder
      description: Remind to run tests before committing
      conditions:
        - type: pattern
          field: prompt
          regex: "(?i)commit"
        - type: pattern
          field: prompt
          regex: "(?!.*test).*"
      action:
        type: inject_context
        context: |
          📋 Pre-commit checklist:
          
          Remember to run tests before committing!
        use_stdout: true  # Default - simple stdout injection
```

**Advanced Mode** - Structured JSON responses:
```yaml
UserPromptSubmit:
  "*":
    - name: architecture-guidance
      description: Provide architectural context
      conditions:
        - type: pattern
          field: prompt
          regex: "(?i)(refactor|architecture|design)"
      action:
        type: inject_context
        context: |
          🏗️ Architecture Guidelines:
          - Follow SOLID principles
          - Prefer composition over inheritance
          - Keep modules loosely coupled
        use_stdout: false  # Use JSON with additionalContext field
```

Both modes are fully supported by Claude Code. Choose simple mode for quick context additions, or advanced mode when you need structured responses.


### Project-Specific Policies

The `$CLAUDE_PROJECT_DIR` environment variable enables powerful project-aware features:

**Policy Discovery**:
```bash
export CLAUDE_PROJECT_DIR=/path/to/project
```

Cupcake will first check `$CLAUDE_PROJECT_DIR/guardrails/cupcake.yaml` before searching upward from the current directory.

**Project-Relative Commands**:
```yaml
# Use project-specific scripts in policies
action:
  type: run_command
  spec:
    mode: array
    command: ["{{env.CLAUDE_PROJECT_DIR}}/scripts/validate.sh"]
    args: ["{{file_path}}"]

# Check project-specific files
conditions:
  - type: check
    spec:
      mode: string
      command: "test -f {{env.CLAUDE_PROJECT_DIR}}/.cupcake/config.json"
    expect_success: true
```

This variable is automatically set by Claude Code when spawning hooks, making your policies portable across different machines and environments.

### MCP Tool Pattern Matching

Support for Model Context Protocol tools with pattern matching:

```yaml
PreToolUse:
  # Match all MCP tools
  "mcp__.*":
    - name: validate-mcp
      ...
  
  # Match specific MCP server
  "mcp__github__.*":
    - name: github-policies
      ...
  
  # Match specific patterns
  "mcp__.*(create|delete).*":
    - name: dangerous-mcp-ops
      ...
```

See [MCP Tool Patterns Guide](docs/mcp-tool-patterns.md) for detailed examples and best practices.

## Commands

- `cupcake init` - Generate policies from CLAUDE.md files
- `cupcake sync` - Update Claude Code hooks configuration
- `cupcake run` - Runtime policy enforcement (called by hooks)
- `cupcake validate` - Validate policy syntax
- `cupcake inspect` - View loaded policies in compact table format

### Policy Inspection

The `inspect` command provides a compact view of all loaded policies:

```bash
$ cupcake inspect
NAME                       EVENT       TOOL       ACTION              CONDITIONS
-------------------------- ----------- ---------- ------------------- ----------
Git Commit Reminder        PreToolUse  Bash       provide_feedback    tool_input.command ~ "git\s+commit"
Dangerous Command Warning  PreToolUse  Bash       block_with_feedback tool_input.command ~ "^(rm|dd)\s.*"
Rust File Formatting       PreToolUse  Edit|Write provide_feedback    tool_input.file_path ~ "\.rs$"
Permission Request          PreToolUse  Bash       ask                 tool_input.command ~ "^sudo\s"
File Creation Confirmation PostToolUse Write      provide_feedback    tool_name = "Write"

Total: 5 policies
```

Perfect for debugging and understanding which policies are active.

## Integration

Cupcake integrates with Claude Code through hooks:

- **PreToolUse**: Block operations before execution
- **PostToolUse**: Provide feedback after execution
- **UserPromptSubmit**: Intercept and validate user prompts before processing
- **Notification**: React to Claude Code notifications
- **Stop/SubagentStop**: Handle session termination events
- **PreCompact**: Manage context compaction events

Response handling via JSON protocol (Claude Code July 20):
- **Soft feedback**: JSON with context injection for transcript visibility
- **Hard decisions**: JSON with `permissionDecision` field ("allow", "deny", "ask")
- **Context injection**: JSON with `additionalContext` for behavioral guidance

### Available Fields for Conditions

Common fields available for all events:
- `event_type` - The hook event name (e.g., "PreToolUse", "UserPromptSubmit")
- `session_id` - Unique session identifier
- `env.*` - Environment variables (e.g., `env.USER`, `env.PATH`)

Tool-specific fields (PreToolUse/PostToolUse only):
- `tool_name` - Name of the tool being invoked (e.g., "Bash", "Write")
- `tool_input.*` - Tool input parameters (e.g., `tool_input.command`, `tool_input.file_path`)

UserPromptSubmit-specific fields:
- `prompt` - The user's input text

Note: The `cwd` field from hook data is used internally as the authoritative working directory for all command executions.

## File Structure

```
guardrails/
├── cupcake.yaml          # Root configuration
└── policies/            # Policy fragments
    ├── git-workflow.yaml
    ├── code-quality.yaml
    └── security-checks.yaml
.cupcake/
└── policy.cache         # Binary cache
.claude/
└── settings.json       # Hook configuration
```

## Performance

Sub-100ms response times through:

- Binary policy cache
- Compiled regex patterns
- Static binary with zero runtime dependencies

## Documentation

- [Policy Format](docs/policy-format.md) - Writing YAML policies
- [MCP Tool Patterns](docs/mcp-tool-patterns.md) - Matching and controlling MCP tools
- [Secure Command Execution](docs/secure-command-execution.md) - Array and string command modes
- [Shell Escape Hatch](docs/shell-escape-hatch.md) - Shell mode with security controls
- [Command Execution Reference](docs/command-execution-reference.md) - Technical details

## License

TBD
