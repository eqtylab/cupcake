---
title: "Policies"
description: "Policy configuration reference for Cupcake"
---

# Policies

Cupcake uses policies to control what AI coding agents can and cannot do.

## [Built-in Policies](builtins.md)

Pre-built security policies that you can enable and configure in your `rulebook.yml`. Battle-tested rules for common security scenarios.

## [Custom Policies](custom.md)

Write your own policies in OPA Rego for complete control over agent behavior. Define exactly what tools and commands are allowed.

## [Signals](signals.md)

Extend policy evaluation with external data and capabilities. Signals are arbitrary programs that collect additional context—from git status to LLM-as-judge evaluations.
