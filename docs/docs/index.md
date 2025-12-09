---
layout: "@/layouts/mdx-layout.astro"
title: "Cupcake"
heading: "Cupcake"
description: "Make AI agents follow the rules"
---

## Introduction

Cupcake is a policy enforcement layer for AI coding agents that delivers better performance, reliability, and security _without_ consuming model context.

- **Deterministic rule-following** for your agents.
- **Better performance** by moving rules out of context and into policy-as-code.
- **LLM-as-a-judge** for more dynamic governance.
- **Trigger alerts** and put _bad_ agents in timeout when they repeatedly violate rules.

Cupcake intercepts agent events and evaluates them against **user-defined rules** written in **[Open Policy Agent (OPA)](https://www.openpolicyagent.org/) [Rego](https://www.openpolicyagent.org/docs/policy-language).** Agent actions can be blocked, modified, and auto-corrected by providing the agent helpful feedback. Additional benefits include reactive automation for tasks you dont need to rely on the agent to conduct (like linting after a file edit).

## Updates

**`2025-12-09`**: Official open source release. Roadmap will be produced in Q1 2026.

**`2025-04-04`**: We produce the [feature request](https://github.com/anthropics/claude-code/issues/712) for Claude Code Hooks. Runtime alignment requires integration into the agent harnesses, and we pivot away from filesystem and os-level monitoring of agent behavior (early cupcake PoC).

## How It Works

[![Cupcake architecture diagram showing policy-based control flow: rules from Claude Code, Cursor and other agents are compiled to OPA Rego policies via WebAssembly, then Cupcake evaluates agent events and returns allow, deny, or halt decisions](assets/flow-cupcake.png)](assets/flow-cupcake.png)

Cupcake sits in the agent hook path. When an agent proposes an action (e.g., run a shell command, edit a file), the details are sent to Cupcake. Cupcake evaluates your policies and returns a decision in milliseconds:

**Allow** · **Deny** · **Halt** · **Modify** · etc

A core pillar of Cupcake is **deterministic guarantees**—policies that always behave the same way given the same input. However, the nature of AI and where it's headed requires more dynamic policy gating. Agents can be prompted, confused, or manipulated in ways that static rules can't anticipate. That's why we developed [**Cupcake Watchdog**](/watchdog/getting-started.md), a built-in feature that uses LLM-as-a-judge to evaluate your rules and context and make intelligent determinations on the fly.

## Getting Started

Install Cupcake and set up your first policy in minutes. Check out our [Installation Guide](/getting-started/installation.md) to get started.

## Supported Harnesses

Cupcake provides native integrations for multiple AI coding agents:

<!-- If you update this table you should also update it on usage page! -->

| Harness                                                                                                                                                                              | Status                         | Guide                                                |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------ | ---------------------------------------------------- |
| <img src="../../assets/claude-light.svg#only-light" alt="Claude Code" width="90"><img src="../../assets/claude-dark.svg#only-dark" width="90" aria-hidden="true">                    | :lucide-check: Fully Supported | [Setup Guide](/getting-started/usage/claude-code.md) |
| <img src="../../assets/cursor-light.svg#only-light" alt="Cursor" width="90"><img src="../../assets/cursor-dark.svg#only-dark" width="90" aria-hidden="true">                         | :lucide-check: Fully Supported | [Setup Guide](/getting-started/usage/cursor.md)      |
| <img src="../../assets/opencode-wordmark-light.svg#only-light" alt="OpenCode" width="90"><img src="../../assets/opencode-wordmark-dark.svg#only-dark" width="90" aria-hidden="true"> | :lucide-check: Fully Supported | [Setup Guide](/getting-started/usage/opencode.md)    |
| <img src="../../assets/factory-light.svg#only-light" alt="Factory AI" width="100"><img src="../../assets/factory-dark.svg#only-dark" width="100" aria-hidden="true">                 | :lucide-check: Fully Supported | [Setup Guide](/getting-started/usage/factory-ai.md)  |

Each harness uses native event formats—no normalization layer. Policies are physically separated by harness (`policies/claude/`, `policies/cursor/`) to ensure clarity and full access to harness-specific capabilities.

## Language Bindings

Cupcake can be embedded in Python or JavaScript agent applications through native bindings. This enables integration with web-based agent frameworks like LangChain, Google ADK, NVIDIA NIM, Vercel AI SDK, and more.

| Language                                                      | Binding            |
| ------------------------------------------------------------- | ------------------ |
| ![Python](assets/python.svg){ width="24" } Python             | `cupcake`          |
| ![TypeScript](assets/typescript.svg){ width="24" } TypeScript | `@eqtylab/cupcake` |

## Why Cupcake?

Modern AI agents are powerful but inconsistent at following operational and security rules, especially as context grows. Cupcake turns the rules you already maintain (e.g., `CLAUDE.md`, `AGENT.md`, `.cursor/rules`) into enforceable guardrails that run before actions execute.

- **Multi-harness support** with first-class integrations for Claude Code and Cursor
- **Governance-as-code** using OPA/Rego compiled to WebAssembly for fast, sandboxed evaluation
- **Enterprise-ready controls:** allow/deny/review, audit trails, and proactive warnings

### Core Capabilities

- **Granular Tool Control**: Prevent specific tools or arguments (e.g., blocking `rm -rf /`).
- **MCP Support**: Native governance for Model Context Protocol tools (e.g., `mcp__memory__*`, `mcp__github__*`).
- **LLM‑as‑Judge**: Use a secondary LLM or agent to evaluate actions for more dynamic oversight.
- **Guardrail Libraries**: First‑class integrations with `NeMo` and `Invariant` for content and safety checks.
- **Observability**: All inputs, signals, and decisions generate structured logs and evaluation traces for debugging.

### Deterministic and Non-Deterministic Evaluation

Cupcake supports two evaluation models:

1. **Deterministic Policies**: Policies are written in **OPA/Rego** and **compiled to WebAssembly (Wasm)** for fast, sandboxed evaluation. [Writing Policies](https://cupcake.eqtylab.io/reference/policies/custom/) guide for implementation details.
2. **LLM‑as‑Judge**: For simpler, yet more advanced, oversight of your rules, Cucpake can interject via a secondary LLM or agent to evaluate how an action should proceed. [Cupcake Watchdog](https://cupcake.eqtylab.io/watchdog/getting-started/) guide for implementation details.

### Decisions & Feedback

Based on the evaluation, Cupcake returns one of five decisions to the agent runtime, along with a human-readable message:

- **Allow**: The action proceeds. Optionally, Cupcake can inject **Context** (e.g., "Remember: you're on the main branch") to guide subsequent behavior without blocking. _Note: Context injection is supported in Claude Code and Factory AI, but not Cursor._
- **Modify**: The action proceeds with transformed input. Policies can sanitize commands, add safety flags, or enforce conventions before execution. _Note: Supported in Claude Code and Factory AI only._
- **Block**: The action is stopped. Cupcake sends **Feedback** explaining _why_ it was blocked (e.g., "Tests must pass before pushing"), allowing the agent to self-correct.
- **Warn**: The action proceeds, but a warning is logged or displayed.
- **Require Review**: The action pauses until a human approves it.

## Built By

Cupcake is developed by [EQTYLab](https://eqtylab.io/), with agentic safety research support by [Trail of Bits](https://www.trailofbits.com/).
