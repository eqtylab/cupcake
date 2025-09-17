# Cupcake Documentation

Welcome to the Cupcake documentation! This guide will help you find the information you need.

## Quick Links

- **[5-Minute Quick Start](./user-guide/quick-start.md)** - Get up and running fast
- **[CLI Commands](./user-guide/cli/commands-reference.md)** - Complete CLI reference
- **[Builtin Policies](./user-guide/policies/builtin-policies-reference.md)** - All 11 builtin policies explained
- **[Writing Policies](./user-guide/policies/writing-policies.md)** - Create custom Rego policies

## Documentation by Role

### For New Users

Start here to understand Cupcake and get your first policy running:

1. **[Installation & Setup](./user-guide/cli/init.md)**

   - Installing Cupcake
   - Running `cupcake init`
   - Enabling builtin policies with `--builtins`

2. **[Core Concepts](./user-guide/configuration/guidebook.md)**

   - What is a policy engine?
   - Understanding signals and actions
   - How Cupcake integrates with AI agents

3. **[First Policy](./user-guide/policies/writing-policies.md#getting-started)**
   - Writing your first Rego policy
   - Testing with `cupcake verify`
   - Common patterns and examples

### For Policy Authors

Detailed guides for creating and managing policies:

- **[Policy Writing Guide](./user-guide/policies/writing-policies.md)** - Complete guide to Rego policies
- **[Builtin Policies](./user-guide/policies/builtin-policies-reference.md)** - Use pre-built policies
- **[Metadata System](./user-guide/policies/metadata-system.md)** - Routing and metadata
- **[Signals & Actions](./user-guide/configuration/signals.md)** - Dynamic data and responses

### For Integrators

Connect Cupcake with your tools:

- **[Claude Code Integration](./agents/claude-code/)** - Hooks and configuration
- **[Harness Integration](./cli/HARNESS_INTEGRATION_SPEC.md)** - Agent integration spec
- **[Guardrail Integrations](./user-guide/configuration/guardrail-integrations.md)** - NeMo & Invariant support
- **[MCP Tool Governance](../README.md#mcp-support)** - Model Context Protocol support

### For Developers

Contributing to or extending Cupcake:

- **[Development Setup](./development/DEVELOPMENT.md)** - Build from source
- **[Architecture](./reference/architecture.md)** - Technical design
- **[Debug Logging](./developer/debugging.md)** - Troubleshooting

## Complete Documentation Map

### CLI Commands

- [`cupcake init`](./user-guide/cli/init.md) - Initialize a project with optional `--builtins`
- `cupcake eval` - Evaluate events against policies
- `cupcake verify` - Verify policy configuration
- `cupcake validate` - Validate policy syntax
- `cupcake inspect` - Inspect policy metadata
- [`cupcake trust`](./user-guide/configuration/trust.md) - Manage script integrity

### Configuration

- [Guidebook.yml](./user-guide/configuration/guidebook.md) - Main configuration file
- [Builtin Policies](./user-guide/policies/builtin-policies-reference.md) - 11 pre-built policies
- [Signals](./user-guide/configuration/signals.md) - External data providers
- [Actions](./user-guide/configuration/signals.md#actions) - Response scripts

### Policy Development

- [Writing Policies](./user-guide/policies/writing-policies.md) - Rego policy authoring
- [Policy Examples](./user-guide/policies/writing-policies.md#examples) - Common patterns
- [Metadata System](./user-guide/policies/metadata-system.md) - Policy routing
- [Why Rego?](./reference/design-decisions.md) - Design decision

### Advanced Topics

- [Trust System](./user-guide/configuration/trust.md) - Script integrity verification
- [WASM Limitations](./reference/wasm-limitations.md) - WebAssembly constraints
- [Distribution](./reference/distribution.md) - Release and packaging
- [Guardrail Integrations](./user-guide/configuration/guardrail-integrations.md) - NeMo & Invariant

## Getting Help

- **Issues:** [GitHub Issues](https://github.com/eqtylab/cupcake/issues)
- **Discussions:** [GitHub Discussions](https://github.com/eqtylab/cupcake/discussions)
- **Examples:** [`/examples/`](../cupcake-rewrite/examples/) directory

## Contributing

Help improve these docs! See [DEVELOPMENT.md](./development/DEVELOPMENT.md) for setup instructions.
