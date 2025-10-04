<p align="left">
  <picture>
    <source srcset="assets/cupcake-dark.png" media="(prefers-color-scheme: dark)">
    <img src="assets/cupcake.png" alt="Cupcake logo" width="180">
  </picture>
</p>

# Cupcake

> Make your AI agents follow your rules

[![Tests](https://img.shields.io/github/actions/workflow/status/eqtylab/cupcake/ci.yml?branch=main&label=tests)](https://github.com/eqtylab/cupcake/actions/workflows/ci.yml)
[![Docs](https://img.shields.io/badge/docs-Start%20here-8A2BE2)](./docs/README.md)
[![License](https://img.shields.io/badge/license-MIT-blue)](LICENSE)

Cupcake is a **policy enforcment** and **early warning** system for AI agents. It is a performance enhancer as well as security tool at the cost of 0 context.

- **Make agents follow your rules in deterministic fashion.**
- **Be alerted when an agent continously violates rules.**
- **Cupcake enables your agents to perform better without added context; and even reduces context spent on describing rules.**

## Agent Policy Enforcement

_Currently in beta with first-class support for Claude Code; designed to be agent-agnostic._

### How It Works

Cupcake runs in the agent hook path and can inject context for nuanced, behavior-guiding prompts.

#### Core Capabilities

**Block any tool call**  
Prevent the use of specific tools or commands based on your policies.

**Behavioral Guidance**  
Inject context and reminders directly into Claude's awareness.

**MCP Support**  
Works seamlessly with Model Context Protocol tools (e.g., `mcp__memory__*`, `mcp__github__*`).

**LLM as a Judge**  
Cupcake makes it easy to integrate other AI agents/LLMs to review actions.

**Guardrail Libraries**  
Cupcake provides first-class support for: `NeMo` and `Invariant` guardrails.

---

It works with the rules you already write (`CLAUDE.md`, `AGENT.md`, `.cursor/rules`) and turns them into **enforceable guardrails**.

#### Three Principles

##### Start with plain English

Cupcake works out of the box with the rules you've already written. Run a single command to get up and running to create the guardrails and automatic feedback to make your agents work better.

##### Governance-as-Code

Under the hood, rules become OPA/Rego policies compiled to WebAssembly for fast, sandboxed checks — including context aware _signals_ to enable intelligent decision making.

##### Enterprise-Ready Security

Enforce consistent allow / deny / require-review decisions, monitor for violations, and prevent dangerous operations like deleting production data or exposing secrets.

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
