# Cupcake Policy DSL (TypeScript)

A type-safe TypeScript DSL for writing Cupcake policies that compile to OPA Rego.

## Overview

This package provides a fluent builder API for defining AI agent policies without writing Rego directly. Policies are type-checked at compile time and generate valid Rego v1 output.

## Features

- **Type-safe builders**: `cant()`, `canOnly()`, `mustHalt()`, `mustAsk()`, `mustModify()`, `mustBlock()`, `addContext()`
- **Event targeting**: `.on('PreToolUse')`, `.on('PostToolUse')`, `.on('UserPromptSubmit')`, etc.
- **Tool-specific fields**: Autocomplete for tool parameters (e.g., `command` for Bash, `path` for Read/Write)
- **Dynamic reasons**: Template literals with field interpolation
- **Condition expressions**: `.equals()`, `.contains()`, `.startsWith()`, `.in()`, `.containsAny()`, etc.
- **Constants**: `defineConstant()` for hoisted arrays

## Example

```typescript
import { policy, cant, canOnly, mustHalt, compile, reason, defineConstant } from '@cupcake/policy-evolved';

const dangerousPaths = defineConstant('dangerous_paths', ['/etc/', '/bin/', '/usr/']);

const securityPolicy = policy(
  'system protection',

  // Block writes to system directories
  cant('write to system paths', ['Write', 'Edit'])
    .severity('CRITICAL')
    .when(({ resolvedFilePath }) => [
      resolvedFilePath.lower().containsAny(dangerousPaths)
    ])
    .reason(({ resolvedFilePath }) =>
      reason`Blocked write to protected path: ${resolvedFilePath}`
    ),

  // Only allow specific git commands
  canOnly('safe git operations', 'Bash')
    .when(({ command }) => [
      command.in(['git status', 'git diff', 'git log'])
    ]),
);

console.log(compile(securityPolicy));
```

## Architecture

```
src/
├── builders/       # Fluent API builders (cant, canOnly, mustHalt, etc.)
├── compiler/       # Rego code generation
│   └── rules/      # Per-rule-type compilers
├── core/           # Type definitions (Rule, Tool, Event, Severity)
├── expressions/    # Condition expression system (StringExpr, BooleanExpr)
├── fields/         # Tool-specific field definitions
├── constants/      # Named constant support
└── tests/          # Vitest test suite
```

## Supported Events

| Event | Description | Available Verbs |
|-------|-------------|-----------------|
| `PreToolUse` | Before tool execution | cant, canOnly, mustHalt, mustAsk, mustModify |
| `PostToolUse` | After tool execution | cant, mustHalt, mustBlock |
| `PermissionRequest` | Permission prompt | cant, mustHalt |
| `UserPromptSubmit` | User sends prompt | mustHalt, mustBlock, addContext |
| `Stop` | Agent stopping | mustHalt, mustBlock |
| `SubagentStop` | Subagent stopping | mustHalt, mustBlock |
| `SessionStart` | Session begins | mustHalt, addContext |

## Implementation History

### Phase 1: Core Verbs
- `cant()` - Deny rules with conditions and reasons
- `canOnly()` - Allow rules with automatic default deny
- `addContext()` - Context injection rules
- `mustHalt()` - Immediate halt rules
- `mustAsk()` - User confirmation prompts
- `mustModify()` - Input transformation rules

### Phase 2: Event System
- Event type definitions and compatibility matrix
- `.on(event)` method for event targeting
- `mustBlock()` - Post-execution blocking
- Event-specific field types (toolResponse, submittedPrompt, etc.)

### Phase 3: Context Event Targeting
- `addContext().on('SessionStart')` for session-scoped context
- `addContext().on('UserPromptSubmit')` for prompt-scoped context

### Bug Fixes (Code Review)
1. Added `input.hook_event_name` check to allow rules (CLAUDE.md compliance)
2. Fixed single quote escaping in `escapeRegoString()`
3. Added `event` field to `AllowRule` type
4. Changed allow rules to use set notation `{...}` instead of arrays
5. Added type guard validation in reason compiler
6. Fixed PostToolUse proxy reuse in `mustHalt()` builder
7. Constrained `TransformResult` type to valid values
8. Added null/empty validation in `compileTransformResult()`
9. Fixed `.in()` operator to use set notation for membership testing

## CLAUDE.md Compliance

All generated policies follow Cupcake CLAUDE.md requirements:
- Every rule includes `input.hook_event_name == "EventName"` check
- Uses Rego v1 syntax: `deny contains decision if { ... }`
- Tool membership uses set notation: `input.tool_name in {"Read", "Write"}`
- Decision objects include `rule_id`, `reason`, `severity`

## Development

```bash
npm install
npm run typecheck   # TypeScript validation
npm test            # Run test suite
npm run build       # Build distribution
```

## Test Coverage

36 tests across 10 test files covering:
- All rule types (deny, allow, halt, ask, modify, block, context)
- Event targeting and conditions
- Dynamic reason templates
- Tool-specific field access
- Constant hoisting
