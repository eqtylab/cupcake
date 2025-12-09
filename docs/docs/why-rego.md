---
layout: "@/layouts/mdx-layout.astro"
title: "Why Rego"
heading: "Why Rego"
description: "Why Cupcake uses Open Policy Agent's Rego language for policy definitions"
---

## Why Rego?

Cupcake uses [Open Policy Agent (OPA)](https://www.openpolicyagent.org/) and its Rego policy language as the foundation for expressing governance rules.

Many frontier agent security research papers present similar architectures where the policy layer is introduced and integrated at runtime. This typically entails an invented DSL. Rego presents an industry-robust and widely-adopted capability across enterprise DevSecOps. Cupcake is oriented towards the enterprise.

Additionally, Rego offers unique, purpose-built advantages for defining, managing, and enforcing policy as code.

### Declarative Policy Expression

Rego is a **declarative language** designed specifically for expressing policy logic. Instead of writing imperative code that describes _how_ to check something, you write rules that describe _what_ should be allowed or denied:

```rego
# Declarative: "deny if command contains rm -rf and targets root"
deny contains decision if {
    input.tool_name == "Bash"
    contains(input.tool_input.command, "rm -rf")
    startswith(input.tool_input.command, "rm -rf /")
}
```

This declarative approach makes policies **easier to read** and audit and **self-documenting.** Teams deploying agents further benefit from policy being unambiguous; the policy logic is separated from the application code.

### Purpose-Built for Authorization

Rego was designed from the ground up for policy decisions. It includes:

**Set-based logic** for expressing complex conditions naturally:

```rego
# Check if tool is in a set of dangerous tools
dangerous_tools := {"rm", "dd", "mkfs", "fdisk"}
deny contains decision if {
    input.tool_name in dangerous_tools
}
```

**Built-in functions** for string matching, path operations, and data manipulation:

```rego
# String matching and path operations
deny contains decision if {
    startswith(input.file_path, "/etc/")
    endswith(input.file_path, ".conf")
    contains(input.command, "sudo")
}
```

**Partial evaluation** for understanding which inputs affect decisions:

```rego
# Rego can determine that only `input.branch` matters for this rule
deny contains decision if {
    input.branch == "main"
    contains(input.command, "git push --force")
}
```

**Deterministic execution** guaranteeing the same input always produces the same output:

```rego
# No randomness, no side effects - same input = same decision every time
deny contains decision if {
    input.tool_name == "Bash"
    input.tool_input.command == "rm -rf /"
}
```

### Dynamically Adapted for AI

While Rego provides the foundation, Cupcake extends it with **decision verbs** designed specifically for AI agent governance:

| Verb          | Purpose                                             |
| ------------- | --------------------------------------------------- |
| `halt`        | Immediately terminate the agent session             |
| `deny`        | Pre-execution rejection with feedback (the bouncer) |
| `block`       | Flow control of either the agent or user            |
| `ask`         | Pause and request human confirmation                |
| `modify`      | Transform tool input before execution               |
| `add_context` | Inject guidance into the agent's context            |

These verbs let you express nuanced governance beyond simple allow/deny:

```rego
# Ask for confirmation on production deployments
ask contains decision if {
    input.tool_name == "Bash"
    contains(input.command, "kubectl apply")
    contains(input.command, "prod")
    decision := {
        "rule_id": "PROD-DEPLOY",
        "reason": "Production deployment detected",
        "question": "Approve deployment to production?"
    }
}

# Inject context to guide agent behavior
add_context contains message if {
    input.hook_event_name == "UserPromptSubmit"
    message := "Remember: Always run tests before committing changes."
}
```

At the core, Cupcake includes a **system evaluate policy** that automatically discovers and aggregates all policy decisions:

```rego
package cupcake.system

# Walk through all policies and collect decisions by verb
evaluate := {
    "halts": collect_verbs("halt"),
    "denials": collect_verbs("deny"),
    "blocks": collect_verbs("block"),
    "asks": collect_verbs("ask"),
    "modifications": collect_verbs("modify"),
    "add_context": collect_verbs("add_context")
}

collect_verbs(verb_name) := result if {
    verb_sets := [value |
        walk(data.cupcake.policies, [path, value])
        path[count(path) - 1] == verb_name
    ]
    # Flatten all sets into a single array
    result := [decision | some set in verb_sets; some decision in set]
}
```

This means you write focused, single-purpose policies—Cupcake handles the orchestration, priority resolution (halt > deny > ask > allow), and response formatting automatically.

Cupcake also enriches policy evaluation with **[Signals](reference/policies/signals.md)**—arbitrary programs that fetch additional context at runtime. Signals enable integration with external systems like git, APIs, linters, or even LLM-as-judge evaluators (as used by [Watchdog](reference/watchdog.md)).

### WebAssembly Compilation

OPA compiles Rego policies to **WebAssembly (WASM)**, enabling:

- **Sub-millisecond evaluation** - typical policies evaluate in under 1ms
- **Sandboxed execution** - policies cannot access the filesystem, network, or system resources
- **Portable deployment** - the same compiled policy runs identically everywhere
- **Memory safety** - WASM's memory model prevents buffer overflows and other vulnerabilities

### Separation of Concerns

By using a dedicated policy language, Cupcake achieves clean separation:

| Concern                   | Responsibility   |
| ------------------------- | ---------------- |
| **What rules to enforce** | Rego policies    |
| **How to evaluate rules** | OPA/WASM runtime |
| **When to apply rules**   | Cupcake engine   |
| **Where rules integrate** | Harness adapters |

This separation means you can modify policies without touching code, and the engine can optimize evaluation without affecting policy semantics.

### Purpose-Built Features for Governance

Rego is designed specifically for policy on structured data (like JSON or YAML), making it excellent for AI agent governance use cases such as:

- Data Access Control: Ensuring agents only query or modify data for which they have explicit authorization.
- Guardrails: Defining constraints like "No agent can access production data outside of business hours" or "A customer service agent can only issue refunds up to $500."
- Uniform Enforcement: A single Rego policy can be used across multiple services and agents (even if they are written in different languages), ensuring consistent rule interpretation.

### Learn More

- [Writing Custom Policies](reference/policies/custom.md) - Start writing your own Rego policies
- [OPA Documentation](https://www.openpolicyagent.org/docs/latest/) - Deep dive into Rego syntax
- [Policy Playground](https://play.openpolicyagent.org/) - Experiment with Rego interactively
