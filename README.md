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
- **Stateful Workflows**: Track tool usage history and enforce time-based policies
- **Project-Specific Policies**: Support for $CLAUDE_PROJECT_DIR for multi-project setups
- **MCP Tool Support**: Pattern matching for Model Context Protocol tools
- **Two-Pass Evaluation**: Aggregates all feedback before decisions
- **Performance Optimized**: Sub-100ms response times with compiled patterns

## Policy Configuration

Policies are defined in YAML format using the guardrails structure:

```yaml
# guardrails/cupcake.yaml - Root configuration
settings:
  audit_logging: true
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
        message: "Tests must pass before committing"
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
            command: ["cupcake"]
            args: ["state", "has-read-file", "docs/architecture.md"]
          expect_success: false
      action:
        type: "block_with_feedback"
        message: "Read docs/architecture.md before editing engine"
        include_context: true

# guardrails/policies/prompt-security.yaml - UserPromptSubmit policies
UserPromptSubmit:
  "":  # Empty string matcher required for non-tool events
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

## Architecture

- **CLI Binary**: Single Rust executable with init, sync, run, validate commands
- **Hook Integration**: Registers with Claude Code's lifecycle events
- **State Management**: Session tracking in `.cupcake/state/`
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

Cupcake can inject context directly into Claude's prompt processing, providing gentle guidance without blocking:

```yaml
UserPromptSubmit:
  "*":
    - name: test-reminder
      description: Remind to run tests before committing
      conditions:
        - type: pattern
          field: prompt
          regex: "(?i)commit"
        - type: not
          condition:
            type: state_query
            filter:
              tool: Bash
              command_contains: "test"
              result: success
              within_minutes: 15
      action:
        type: inject_context
        context: |
          📋 Pre-commit checklist:
          ✗ Tests haven't been run recently
          
          Consider running tests before committing.
        use_stdout: true
```

### Stateful Workflows with StateQuery

Track tool usage history and make decisions based on past actions:

```yaml
conditions:
  - type: state_query
    filter:
      tool: Bash                    # Tool name
      command_contains: "npm test"  # Command pattern (optional)
      result: success              # "success" or "failure" (optional)
      within_minutes: 30           # Time window (optional)
    expect_exists: true            # true = must exist, false = must not exist
```

This enables sophisticated workflows like:
- Ensure tests pass before allowing commits
- Prevent dangerous operations after specific actions
- Enforce time-based cool-downs between operations
- Track and audit complex multi-step processes

### Project-Specific Policies

Set `$CLAUDE_PROJECT_DIR` to use project-specific policies:

```bash
export CLAUDE_PROJECT_DIR=/path/to/project
```

Cupcake will first check `$CLAUDE_PROJECT_DIR/guardrails/cupcake.yaml` before searching upward from the current directory.

### MCP Tool Pattern Matching

Support for Model Context Protocol tools with pattern matching:

```yaml
PreToolUse:
  # Match all MCP tools
  "mcp__.*":
    - name: audit-mcp
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
   ```

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
File Creation Confirmation PostToolUse Write      provide_feedback    tool_name = "Write"

Total: 4 policies
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

Response handling:
- **Exit code 0**: Soft feedback (transcript only)
- **Exit code 2**: Hard block (Claude sees feedback)

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
├── policy.cache         # Binary cache
├── state/               # Session tracking
└── audit.log           # Optional audit trail
.claude/
└── settings.json       # Hook configuration
```

## Performance

Sub-100ms response times through:

- Binary policy cache
- Compiled regex patterns
- Lazy state loading
- Static binary with zero runtime dependencies

## Documentation

- [Policy Format](docs/policy-format.md) - Writing YAML policies
- [Secure Command Execution](docs/secure-command-execution.md) - Array and string command modes
- [Shell Escape Hatch](docs/shell-escape-hatch.md) - Shell mode with security controls
- [Command Execution Reference](docs/command-execution-reference.md) - Technical details

## License

TBD
