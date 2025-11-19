# @eqtylab/cupcake

> Policy enforcement for AI agents and automation tools - TypeScript/Node.js bindings

[![npm version](https://badge.fury.io/js/@eqtylab%2Fcupcake.svg)](https://www.npmjs.com/package/@eqtylab/cupcake)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)

**Make AI agents follow the rules.**

This package provides TypeScript/Node.js bindings for the [Cupcake](https://github.com/eqtylab/cupcake) policy engine. Embed policy evaluation in your custom AI agents, automation tools, and applications.

## Features

- 🚀 **Async-first API** - Non-blocking policy evaluation for servers
- 🔒 **Secure by design** - Policies run in sandboxed WebAssembly
- ⚡ **High performance** - Sub-millisecond evaluation with native Rust engine
- 🧩 **Framework agnostic** - Works with any TypeScript/Node.js application
- 📝 **OPA Rego policies** - Industry-standard policy language
- 🔄 **Auto OPA installation** - Automatically downloads and verifies OPA binary

## Use Cases

Unlike native integrations for Claude Code and Cursor, these bindings enable **custom applications** to embed Cupcake:

- **AI Agent Frameworks** - Vercel AI SDK, LangChain, AutoGPT
- **Automation Tools** - CI/CD pipelines, deployment systems, IaC tools
- **API Gateways** - Policy-enforced endpoints for AI actions
- **Developer Tools** - CLI tools, build systems, code generators

## Installation

```bash
npm install @eqtylab/cupcake
```

### Requirements

- **Node.js**: v16 or later
- **Platforms**: macOS (x64/arm64), Linux (x64/arm64), Windows (x64)
- **OPA**: Auto-installed on first use (v1.71.0)

## Quick Start

### 1. Initialize Cupcake

```typescript
import { Cupcake } from '@eqtylab/cupcake';

const cupcake = new Cupcake();
await cupcake.init('.cupcake'); // Path to policy directory
```

### 2. Define Your Policies

Create policies in `.cupcake/policies/`:

```rego
# .cupcake/policies/example.rego
package cupcake.policies.example

import rego.v1

deny contains decision if {
    input.kind == "shell"
    input.command == "rm"
    "-rf" in input.args

    decision := {
        "rule_id": "DANGEROUS_RM",
        "reason": "Recursive rm command is not allowed",
        "severity": "CRITICAL"
    }
}
```

### 3. Evaluate Events

```typescript
const decision = await cupcake.evaluate({
  kind: 'shell',
  command: 'rm',
  args: ['-rf', '/important/data'],
});

if (decision.decision === 'Deny') {
  console.error('Blocked:', decision.reason);
  throw new Error('Policy violation');
}

// Action is allowed, proceed
console.log('Action allowed');
```

## API Reference

### Cupcake Class

```typescript
class Cupcake {
  // Initialize (async, recommended)
  async init(path?: string, harness?: 'claude' | 'cursor'): Promise<void>

  // Initialize (sync, blocks event loop - use only in CLI scripts)
  initSync(path?: string, harness?: 'claude' | 'cursor'): void

  // Evaluate (async, non-blocking - recommended)
  async evaluate(event: HookEvent): Promise<Decision>

  // Evaluate (sync, blocks event loop - use only in CLI scripts)
  evaluateSync(event: HookEvent): Decision

  // Get version
  get version(): string

  // Check if ready
  get isReady(): boolean
}
```

### Module-Level API

```typescript
// Convenience functions using a singleton instance
async function init(path?: string, harness?: 'claude' | 'cursor'): Promise<void>
async function evaluate(event: HookEvent): Promise<Decision>
function version(): string
function isReady(): boolean
```

### Types

```typescript
interface HookEvent {
  [key: string]: any; // Your application defines the structure
}

interface Decision {
  decision: 'Allow' | 'Deny' | 'Halt' | 'Ask';
  reason?: string;
  context?: string[];
  question?: string;
  severity?: 'LOW' | 'MEDIUM' | 'HIGH' | 'CRITICAL';
  rule_id?: string;
  [key: string]: any;
}
```

## Examples

### Express Server

```typescript
import express from 'express';
import { init, evaluate } from '@eqtylab/cupcake';

await init('.cupcake');

app.post('/execute', async (req, res) => {
  const decision = await evaluate(req.body);

  if (decision.decision === 'Deny') {
    return res.status(403).json({ error: decision.reason });
  }

  // Execute action...
  res.json({ status: 'success' });
});
```

### Vercel AI SDK Integration

```typescript
import { Cupcake } from '@eqtylab/cupcake';

const cupcake = new Cupcake();
await cupcake.init();

function policyEnforcedTool(name, schema, execute) {
  return {
    name,
    parameters: schema,
    execute: async (input) => {
      const decision = await cupcake.evaluate({
        kind: 'tool_use',
        tool: name,
        input,
      });

      if (decision.decision !== 'Allow') {
        throw new Error(`Policy blocked: ${decision.reason}`);
      }

      return execute(input);
    },
  };
}

const tools = {
  shell: policyEnforcedTool('shell', shellSchema, executeShell),
  database: policyEnforcedTool('database', dbSchema, executeQuery),
};
```

See [`examples/`](./examples/) for complete examples.

## Performance

### Async vs Sync

| Method | Event Loop | Use Case |
|--------|-----------|----------|
| `evaluate()` (async) | ✅ Non-blocking | **Recommended** - Web servers, APIs, concurrent apps |
| `evaluateSync()` (sync) | ❌ Blocks | CLI scripts, startup validation, simple tools |

### Benchmarks

- **Initialization**: ~100-200ms (one-time, includes policy compilation)
- **Evaluation**: <1ms for simple policies, 1-5ms for complex policies
- **Concurrency**: Thousands of concurrent evaluations

## Error Handling

```typescript
import { CupcakeError } from '@eqtylab/cupcake';

try {
  await cupcake.init('./bad-path');
} catch (error) {
  if (error instanceof CupcakeError) {
    console.error('Cupcake error:', error.code, error.message);
  }
}
```

Error codes:
- `NOT_INITIALIZED` - Engine not initialized
- `ALREADY_INITIALIZED` - Already initialized
- `INIT_FAILED` - Initialization failed
- `EVALUATION_FAILED` - Policy evaluation failed

## Development

### Building from Source

```bash
# Install dependencies
npm install

# Build native module
npm run build

# Run tests
npm test

# Format code
npm run format
```

### Local Development

```bash
# Build in debug mode (faster)
npm run build:debug

# Watch mode
npm run build -- --watch
```

## Platform Support

Pre-built binaries are provided for:

- **macOS**: x64 (Intel), arm64 (Apple Silicon)
- **Linux**: x64, arm64 (glibc and musl)
- **Windows**: x64

If your platform is not supported, the package will attempt to build from source (requires Rust toolchain).

## License

MIT © [EQTYLab](https://eqtylab.io/)

## Links

- [Cupcake Documentation](https://github.com/eqtylab/cupcake)
- [OPA Documentation](https://www.openpolicyagent.org/docs/latest/)
- [Issue Tracker](https://github.com/eqtylab/cupcake/issues)
- [npm Package](https://www.npmjs.com/package/@eqtylab/cupcake)

## Support

- GitHub Issues: https://github.com/eqtylab/cupcake/issues
- Documentation: https://github.com/eqtylab/cupcake/tree/main/docs

## Citation

```bibtex
@software{Cupcake2025,
  author = {Ramos, Michael and EQTYLab},
  title = {{Cupcake: Policy enforcement for AI agents}},
  year = {2025},
  publisher = {EQTYLab},
  url = {https://github.com/eqtylab/cupcake}
}
```
