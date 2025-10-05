<p align="left">
  <picture>
    <source srcset="assets/cupcake-dark.png" media="(prefers-color-scheme: dark)">
    <img src="assets/cupcake.png" alt="Cupcake logo" width="180">
  </picture>
</p>

# Cupcake

> Make AI agents follow the rules.

[![Tests](https://img.shields.io/github/actions/workflow/status/eqtylab/cupcake/ci.yml?branch=main&label=tests)](https://github.com/eqtylab/cupcake/actions/workflows/ci.yml)
[![Docs](https://img.shields.io/badge/docs-Start%20here-8A2BE2)](./docs/README.md)
[![License](https://img.shields.io/badge/license-MIT-blue)](LICENSE)

Cupcake is a **policy enforcement** and **early‑warning** layer for AI agents. It strengthens security and reliability **without consuming model context**.

- **Deterministic rule‑following** for your agents.
- **Immediate alerts** when an agent repeatedly violates policies.
- **Higher performance with less boilerplate**, by moving rules out of prompts and into guardrails.

## Why Cupcake?

Modern agents are powerful but inconsistent at following operational and security rules, especially as prompts grow. Cupcake turns the rules you already maintain (e.g., `CLAUDE.md`, `AGENT.md`, `.cursor/rules`) into **enforceable guardrails** that run before actions execute.

- **Agent‑agnostic design** with first‑class support for **Claude Code hooks**.
- **Governance‑as‑code** using OPA/Rego compiled to WebAssembly for fast, sandboxed evaluation.
- **Enterprise‑ready** controls: allow/deny/review, audit trails, and proactive warnings.

## How it Works

Cupcake sits in the agent hook path. When an agent proposes an action (e.g., run a shell command, edit a file, call a tool), the details are sent to Cupcake. Cupcake evaluates your policies and returns a decision in milliseconds:

**Allow** · **Block** · **Warn** (and optionally **Require Review**)

```text
Agent → (proposed action) → Cupcake → (policy decision) → Agent runtime
```

### Core Capabilities

- **Block specific tool calls**
  Prevent use of particular tools or arguments based on policy.

- **Behavioral guidance**
  Inject lightweight, contextful reminders back to the agent (e.g., "run tests before pushing").

- **MCP support**
  Govern Model Context Protocol tools (e.g., `mcp__memory__*`, `mcp__github__*`).

- **LLM‑as‑Judge**
  Chain in a review step by another model/agent when policies say a human‑style check is needed.

- **Guardrail libraries**
  First‑class integrations with `NeMo` and `Invariant` for content and safety checks.

- **Signals (real‑time context)**
  Pull facts from the environment (current Git branch, changed files, deployment target, etc.) and make policy decisions on them.

## Architecture

- **Policies**: OPA/Rego compiled to **WASM** and executed in a sandbox.
- **Signals**: Extensible providers (Git, CI, DB metadata, feature flags) available to policy via `input.signals`.
- **Decisions**: `allow | block | warn | require_review` plus a human‑readable message.
- **Observability**: Structured logs and optional evaluation traces for debugging.

```jsonc
// Example input passed to policy evaluation
{
  "kind": "shell", // action type (shell, fs_read, fs_write, mcp_call, ...)
  "command": "git push",
  "args": [],
  "signals": { "tests_passed": false, "git_branch": "feature/x" },
  "actor": { "id": "agent-1", "session": "abc123" }
}
```

## Security Model

- **Sandboxed evaluation** of untrusted inputs.
- **Allow‑by‑default** or **deny‑by‑default** modes configurable per project.
- **No secret ingestion** by default; policies can only read what signals expose.
- **Auditability** through logs and optional review workflows.

See the full [Security Model](./docs/SECURITY.md).

## FAQ

**Does Cupcake consume prompt/context tokens?**
No. Policies run outside the model and return structured decisions.

**Is Cupcake tied to a specific model?**
No. It’s agent‑agnostic; Claude Code support ships first.

**How fast is evaluation?**
Sub‑millisecond for cached policies in typical setups.

## License

[MIT](LICENSE)
