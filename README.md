# Cupcake

<p align="left">
  <picture>
    <source srcset="docs/docs/assets/cupcake-dark.png" media="(prefers-color-scheme: dark)">
    <img src="docs/docs/assets/cupcake.png" alt="Cupcake logo" width="180">
  </picture>
</p>

Make AI agents follow the rules.

[![Docs](https://img.shields.io/badge/docs-Start%20here-8A2BE2)](https://cupcake.eqtylab.io/)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue)](LICENSE)
[![Tests](https://img.shields.io/github/actions/workflow/status/eqtylab/cupcake/ci.yml?branch=main&label=tests)](https://github.com/eqtylab/cupcake/actions/workflows/ci.yml)
[![SLSA 3](https://slsa.dev/images/gh-badge-level3.svg)](https://github.com/eqtylab/cupcake/actions/runs/19737865145/job/56558251812)

**Policy enforcement** layer for AI agents; yielding better performance and security **without consuming model context**.

- **Deterministic rule-following** for your agents. [Interactive Examples](https://cupcake-policy-studio.vercel.app/example-policies/security/symlink-detection?harness=claude-code&format=rego)
- **Better performance** by moving rules out of context and into policy-as-code.
- **Trigger alerts** and put _bad_ agents in timeout when they repeatedly violate rules.

Cupcake intercepts agent events and evaluates them against **user-defined rules** written in **[Open Policy Agent (OPA)](https://www.openpolicyagent.org/) [Rego](https://www.openpolicyagent.org/docs/policy-language).** Agent actions can be blocked, modified, and auto-corrected by providing the agent helpful feedback. Additional benefits include reactive automation for tasks you dont need to rely on the agent to conduct (like linting after a file edit).

## Updates

**`2025-12-10`**: Official open source release. Roadmap will be produced in Q1 2026.

**`2025-04-04`**: We produce the [feature request](https://github.com/anthropics/claude-code/issues/712) for Claude Code Hooks. Runtime alignment requires integration into the agent harnesses, and we pivot away from filesystem and os-level monitoring of agent behavior (early cupcake PoC).

## Supported Agent Harnesses

Cupcake provides lightweight **native integrations** for multiple AI coding agents:

| Harness                                                                           | Status             | Integration Guide                                                            |
| --------------------------------------------------------------------------------- | ------------------ | ---------------------------------------------------------------------------- |
| **[Claude Code](https://claude.ai/code)**                                         | ✅ Fully Supported | [Setup Guide](https://cupcake.eqtylab.io/getting-started/usage/claude-code/) |
| **[Cursor](https://cursor.com)**                                                  | ✅ Fully Supported | [Setup Guide](https://cupcake.eqtylab.io/getting-started/usage/cursor/)      |
| **[Factory AI](https://docs.factory.ai/welcome)**                                 | ✅ Fully Supported | [Setup Guide](https://cupcake.eqtylab.io/getting-started/usage/factory-ai/)  |
| **[OpenCode](https://opencode.ai)**                                               | ✅ Fully Supported | [Setup Guide](https://cupcake.eqtylab.io/getting-started/usage/opencode/)    |
| **[AMP](https://ampcode.com)**                                                    | Coming soon        | [Awaiting release](https://ampcode.com/manual?internal#hooks)                |
| **[Gemini CLI](https://docs.cloud.google.com/gemini/docs/codeassist/gemini-cli)** | Coming soon        | [Awaiting release](https://github.com/google-gemini/gemini-cli/issues/2779)  |

Each harness uses native event formats. Similar to terraform, policies are separated by harness (`policies/claude/`, `policies/cursor/`, `policies/factory/`, `policies/opencode/`) to ensure clarity and full access to harness-specific capabilities. If a particular harness is not supported, it is because it has no means for runtime integration.

#### Language Bindings

Cupcake can be embedded in Python or JavaScript agent applications through native bindings. This enables integration with web-based agent frameworks like LangChain, Google ADK, NVIDIA NIM, Vercel AI SDK, and more.

| Language                                                                      | Binding        |
| ----------------------------------------------------------------------------- | -------------- |
| <img src="docs/docs/assets/python.svg" width="24" height="24"> Python         | `./cupcake-py` |
| <img src="docs/docs/assets/typescript.svg" width="24" height="24"> TypeScript | `./cupcake-ts` |

## How it Works

Cupcake acts as an enforcement layer between your coding agents and their runtime environment **via hooks** directly in the agent action path.

<img src="./docs/docs/assets/flow-cupcake.png" alt="Cupcake agent hooks security architecture" width="600"/>

`Agent → (proposed action) → Cupcake → (policy decision) → Agent runtime`

1. **Interception**: The agent prepares to execute an action/tool-call (e.g., `git push`, `fs_write`).
2. **Enrichment**: Cupcake gathers real-time **Signals**—facts from the environment such as the current Git branch, CI status, or database metadata.
3. **Evaluation**: The action and signals are packaged into a JSON input and evaluated against your Wasm policies in milliseconds.

### Deterministic and Non-Deterministic Evaluation

Cupcake supports two evaluation models:

1. **Deterministic Policies**: Policies are written in **OPA/Rego** and **compiled to WebAssembly (Wasm)** for fast, sandboxed evaluation. [Writing Policies](https://cupcake.eqtylab.io/reference/policies/custom/) guide for implementation details.
2. **LLM‑as‑Judge**: For simpler, yet more advanced, oversight of your rules, Cupake can interject via a secondary LLM or agent to evaluate how an action should proceed. [Cupcake Watchdog](https://cupcake.eqtylab.io/watchdog/getting-started/) guide for implementation details.

### Decisions & Feedback

Based on the evaluation, Cupcake returns one of five decisions to the agent runtime, along with a human-readable message:

- **Allow**: The action proceeds. Optionally, Cupcake can inject **Context** (e.g., "Remember: you're on the main branch") to guide subsequent behavior without blocking. _Note: Context injection is supported in Claude Code and Factory AI, but not Cursor._
- **Modify**: The action proceeds with transformed input. Policies can sanitize commands, add safety flags, or enforce conventions before execution. _Note: Supported in Claude Code and Factory AI only._
- **Block**: The action is stopped. Cupcake sends **Feedback** explaining _why_ it was blocked (e.g., "Tests must pass before pushing"), allowing the agent to self-correct.
- **Warn**: The action proceeds, but a warning is logged or displayed.
- **Require Review**: The action pauses until a human approves it.

## Why Cupcake?

Modern agents are powerful but inconsistent at following operational and security rules, especially as context grows. Cupcake turns the rules you already maintain (e.g., `CLAUDE.md`, `AGENT.md`, `.cursor/rules`) into **enforceable guardrails** that run before actions execute.

- **Multi-harness support** with first‑class integrations for **Claude Code**, **Cursor**, **Factory AI**, and **OpenCode**.
- **Governance‑as‑code** using OPA/Rego compiled to WebAssembly for fast, sandboxed evaluation.
- **Enterprise‑ready** controls: allow/deny/review, **enriched audit** trails for AI SOCs, and proactive warnings.

### Core Capabilities

- **Granular Tool Control**: Prevent specific tools or arguments (e.g., blocking `rm -rf /`).
- **MCP Support**: Native governance for Model Context Protocol tools (e.g., `mcp__memory__*`, `mcp__github__*`).
- **LLM‑as‑Judge**: Use a secondary LLM or agent to evaluate actions for more dynamic oversight.
- **Guardrail Libraries**: First‑class integrations with `NeMo` and `Invariant` for content and safety checks.
- **Observability**: All inputs, signals, and decisions generate structured logs and evaluation traces for debugging.

## FAQ

**Does Cupcake consume prompt/context tokens?**
No. Policies run outside the model and return structured decisions.

**Is Cupcake tied to a specific model?**
No. Cupcake supports multiple AI coding agents with harness-specific integrations.

**How fast is evaluation?**
Sub‑millisecond for cached policies in typical setups.

## Contributing

We welcome contributions! See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

[Apache 2.0](LICENSE)

---

Cupcake is developed by [EQTYLab](https://eqtylab.io/), with agentic safety research support by [Trail of Bits](https://www.trailofbits.com/).

[Follow on X](https://x.com/CupcakeSecures) for a regular updates.
