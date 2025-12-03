# Watchdog

[![Cupcake Watchdog architecture diagram showing how the LLM-as-a-judge evaluates AI coding agent tool calls, with rules from Claude Code, Cursor, and other agents flowing into Watchdog which makes allow or deny decisions](../assets/flow-watchdog.avif)](../assets/flow-watchdog.avif)

Watchdog is Cupcake's LLM-as-a-judge capability. It evaluates AI agent tool calls using another LLM before they execute, providing semantic security analysis that complements deterministic policy rules.

## What is LLM-as-a-Judge?

LLM-as-a-judge is a pattern where one AI model evaluates the outputs or actions of another. Instead of relying solely on pattern matching or static rules, you use an LLM's reasoning capabilities to assess whether an action is appropriate, safe, or aligned with intent.

For AI coding agents, this means:

- **Semantic understanding**: Catching threats that don't match simple patterns
- **Context awareness**: Evaluating actions against the broader conversation
- **Dynamic reasoning**: Adapting to novel situations without new rules

## Why Cupcake is Well-Positioned for This

Cupcake already sits at the chokepoint between AI agents and their tools. Every file edit, shell command, and API call flows through Cupcake's policy engine. This makes it the natural place to add LLM-based evaluation:

1. **Already intercepting events**: No additional integration work for users
2. **Structured input**: Events are already parsed and normalized
3. **Policy composition**: Watchdog results flow into the same policy system as deterministic rules
4. **Fail-safe by default**: If the LLM is unavailable, Cupcake's deterministic policies still protect you

## How It Works

When Watchdog is enabled:

1. An AI agent attempts a tool call (e.g., run a shell command)
2. Cupcake intercepts the event as usual
3. Watchdog sends the event to an LLM for evaluation
4. The LLM returns a structured judgment: allow/deny, confidence, reasoning
5. This judgment is available to your policies as `input.signals.watchdog`
6. Your policies decide the final outcome

```
Agent Action → Cupcake → Watchdog (LLM) → Policy Evaluation → Decision
```

## Use Cases

### Security

- Detecting data exfiltration attempts that don't match known patterns
- Identifying commands that seem misaligned with the user's stated intent
- Flagging suspicious sequences of actions

### Developer Experience

- Suggesting better approaches before executing suboptimal commands
- Providing context-aware warnings
- Guiding agents toward project-specific best practices

## Non-Deterministic Answer to Non-Determinism

AI agents are inherently non-deterministic. They can be prompted, confused, or manipulated in ways that deterministic rules can't anticipate. Watchdog addresses this by fighting fire with fire—using AI to evaluate AI.

This doesn't replace deterministic policies. It complements them. Use Rego rules for known patterns and hard requirements. Use Watchdog for semantic analysis and catching the unexpected.
