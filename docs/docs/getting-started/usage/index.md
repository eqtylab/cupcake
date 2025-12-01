---
title: "Usage"
description: "Get up and running with Cupcake"
---

# Usage

After [installation](../installation/), you're ready to set up Cupcake for your project. The first step is choosing which AI coding agent (harness) you're using.

## Select Your Harness

<div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(140px, 1fr)); gap: 1.5rem; margin: 2rem 0;">
  <a href="claude-code/" style="text-decoration: none; color: inherit;">
    <div style="border: 1px solid var(--md-default-fg-color--lightest); border-radius: 8px; padding: 1.5rem; text-align: center;">
      <img src="../../assets/claude-light.svg#only-light" alt="Claude Code" width="100">
      <img src="../../assets/claude-dark.svg#only-dark" alt="Claude Code" width="100">
    </div>
  </a>
  <a href="cursor/" style="text-decoration: none; color: inherit;">
    <div style="border: 1px solid var(--md-default-fg-color--lightest); border-radius: 8px; padding: 1.5rem; text-align: center;">
      <img src="../../assets/cursor-light.svg#only-light" alt="Cursor" width="100">
      <img src="../../assets/cursor-dark.svg#only-dark" alt="Cursor" width="100">
    </div>
  </a>
  <a href="opencode/" style="text-decoration: none; color: inherit;">
    <div style="border: 1px solid var(--md-default-fg-color--lightest); border-radius: 8px; padding: 1.5rem; text-align: center;">
      <img src="../../assets/opencode-wordmark-light.svg#only-light" alt="OpenCode" width="100">
      <img src="../../assets/opencode-wordmark-dark.svg#only-dark" alt="OpenCode" width="100">
    </div>
  </a>
  <a href="factory-ai/" style="text-decoration: none; color: inherit;">
    <div style="border: 1px solid var(--md-default-fg-color--lightest); border-radius: 8px; padding: 1.5rem; text-align: center;">
      <img src="../../assets/factory.svg" alt="Factory AI" width="100">
    </div>
  </a>
</div>

| Harness         | Status          | Guide                       |
| --------------- | --------------- | --------------------------- |
| **Claude Code** | Fully Supported | [Setup Guide](claude-code/) |
| **Cursor**      | Fully Supported | [Setup Guide](cursor/)      |
| **OpenCode**    | Fully Supported | [Setup Guide](opencode/)    |
| **Factory AI**  | Fully Supported | [Setup Guide](factory-ai/)  |

## Next Steps

After setting up your harness, learn how to configure policies:

- **[Built-in Policies](../../reference/policies/builtins/)** — Enable pre-built security policies
- **[Custom Policies](../../reference/policies/custom/)** — Write your own policies in Rego
