<p align="left">
  <picture>
    <source srcset="assets/cupcake-dark.png" media="(prefers-color-scheme: dark)">
    <img src="assets/cupcake.png" alt="Cupcake logo" width="180">
  </picture>
</p>

# Cupcake

> Make your AI agents follow your rules

[![Tests](https://img.shields.io/github/actions/workflow/status/eqtylab/cupcake/ci.yml?branch=main&label=tests)](https://github.com/eqtylab/cupcake/actions/workflows/ci.yml)
[![Docs](https://img.shields.io/badge/docs-Start%20here-brightgreen)](./docs/README.md)
[![License](https://img.shields.io/badge/license-MIT-blue)](LICENSE)

Cupcake is a **policy engine** for AI coding agents. It works with the rules you already write (`CLAUDE.md`, `AGENT.md`, `.cursor/rules`) and turns them into **enforceable guardrails**.

- **Start with plain English.** Cupcake works out of the box with the rules you've already written. Run a single command to get up and running to create the guardrails and automatic feedback to make your agents work better.
- **Powerful Governance-as-Code.** Under the hood, rules become OPA/Rego policies compiled to WebAssembly for fast, sandboxed checks — including context aware _signals_ to enable intelligent decision making.
- **Enterprise-Ready Security.** enforce consistent allow / deny / require-review decisions, monitor for violations, and prevent dangerous operations like deleting production data or exposing secrets.

Cupcake runs in the agent hook path and can inject context for nuanced, behavior-guiding prompts.

- **Block any tool call**: Prevent the use of specific tools or commands based on your policies.
- **Behavioral Guidance**: Inject context and reminders directly into Claude's awareness.
- **MCP Support**: Works seamlessly with Model Context Protocol tools (e.g., `mcp__memory__*`, `mcp__github__*`).
- **LLM as a Judge**: Cupcake makes it easy to integrate other AI agents/LLMs to review actions.
- **Guardrail Libraries**: Cupcake provides first-class support for: `NeMo` and `Invariant` guardrails.

> Currently in beta with first-class support for Claude Code; designed to be agent-agnostic.

[Getting Started](#getting-started) · [Examples](./examples) · [Policy Author’s Guide](./POLICIES.md) · [Security Model](./docs/SECURITY.md) · [Roadmap](./ROADMAP.md)

## How It Works

Cupcake integrates with Claude Code's [hooks system](https://docs.anthropic.com/claude-code/docs/hooks-guide). When Claude is about to perform an action (like running a shell command or editing a file), it sends the details to Cupcake. Cupcake evaluates the action against your policies and instantly sends back a decision: **Allow**, **Block**, or **Warn**.

This happens in milliseconds, providing seamless, real-time governance without interrupting the developer's flow.

## Core Features

- **Powerful Policy Engine**: Write policies in OPA Rego, the industry standard for policy-as-code. Go beyond simple patterns to express complex, context-aware rules.
- **Real-Time Context**: Cupcake's "Signals" can gather facts from your environment—like the current Git branch or database info use them in policy decisions. (Similar for security-awareness circumstances)
- **High Performance**: Sub-millisecond evaluation times for cached policies. A highly optimized WebAssembly (WASM) runtime ensures governance never slows you down.
- **Production Ready**: Built with structured logging, robust error handling, a comprehensive test suite, and policy evaluation tracing to be a reliable and trusted part of your ai workflows.

## Use Cases

What can you build with Cupcake?

- **Command Safety:**

  - `"Block dangerous commands like 'rm -rf /'."`
  - `"Require user confirmation before running any 'sudo' command."`

- **Best Practice Reminders:**
  - `"If the agent tries to commit code, remind it to run the linter first."`

Simple examples:

- **File and Directory Protection:**

  - `"Prevent any edits to files inside the .aws/ or .ssh/ directories."`
  - `"Allow file writes, but only to the src/ directory."`

- **Git Workflow Enforcement:**

  - `"Block 'git merge' if the current branch is not 'develop'."`
  - `"Warn if 'git push' is attempted before tests have passed."`

- **MCP Tool Governance:**

  - `"Block storing sensitive data in MCP memory tools."`
  - `"Require confirmation for destructive MCP GitHub operations."`
  - `"Prevent MCP filesystem access to system directories."`

## Development

### Running Tests

**IMPORTANT**: Tests MUST be run with the `deterministic-tests` feature flag AND with global config disabled. This ensures:
1. Deterministic HMAC key generation for reliable test execution
2. No interference from developer's personal Cupcake configuration

```bash
# Run all tests (REQUIRED for correct behavior)
CUPCAKE_GLOBAL_CONFIG=/nonexistent cargo test --features deterministic-tests

# Or use the Just commands (automatically handles both requirements)
just test              # Run all tests
just test-unit        # Run unit tests only
just test-integration # Run integration tests only
just test-one <name>  # Run specific test

# Alias for quick testing
cargo t  # Configured alias that includes required flags
```

### Releasing

To create a new release, push a version tag: `git tag v0.1.8 && git push origin v0.1.8`. See [Development Guide](./docs/development/DEVELOPMENT.md#release-process) for details.

#### Why Global Config Must Be Disabled

If you use Cupcake as a developer, you likely have a global configuration at `~/Library/Application Support/cupcake` (macOS) or `~/.config/cupcake` (Linux). This global config is designed to override project configs for organizational policy enforcement.

However, during testing, this causes issues:
- Tests expect specific builtin configurations
- Global configs override the test's project configs
- Tests fail with unexpected policy decisions

Setting `CUPCAKE_GLOBAL_CONFIG=/nonexistent` ensures tests run in isolation. Global tests that need global configs create their own temporary configurations.

The feature flag ensures deterministic HMAC key generation for reliable test execution. Without it, integration tests will experience race conditions and cryptographic verification failures due to non-deterministic key derivation in production mode.

### Building

```bash
# Development build
cargo build

# Release build
cargo build --release
```
