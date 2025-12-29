# Playground Evolved - Implementation Log

## Compiler Implementation (2025-12-21)

### Summary

Implemented the Rego compiler that transforms the TypeScript DSL AST into valid Rego v1 source code. All 4 tests pass.

### Files Created

```
src/compiler/
├── index.ts              # Main compile() function + toPackageName
├── context.ts            # CompilerContext type for tracking state
├── paths.ts              # Field path → Rego path mapping
├── format.ts             # String escaping, indentation, formatting
├── expressions.ts        # Compile condition expressions to Rego
├── reason.ts             # Compile static/dynamic reasons
└── rules/
    ├── index.ts          # Rule compilation exports
    ├── deny.ts           # DenyRule → deny contains decision if {...}
    ├── allow.ts          # AllowRule → allow if {...} + default deny
    └── context.ts        # ContextRule → add_context contains ctx if {...}

src/constants/
├── index.ts              # Constants module exports
└── define.ts             # defineConstant() for named constants
```

### Key Design Decisions

1. **Indentation**: 4-space (Rego convention)
2. **Tool membership**: Sets `{}` for deny rules, arrays `[]` for allow rules
3. **Hook event check**: `input.hook_event_name == "PreToolUse"` required for deny rules
4. **Constant naming**: Derived from field name (e.g., `resolvedFilePath` → `resolved_file_paths`)
5. **Transform propagation**: `lower()` applied to both sides of containsAny comparisons

### Path Mappings

| DSL Field | Rego Path |
|-----------|-----------|
| `command` | `input.tool_input.command` |
| `path` | `input.tool_input.path` |
| `content` | `input.tool_input.content` |
| `isSymlink` | `input.is_symlink` |
| `resolvedFilePath` | `input.resolved_file_path` |
| `originalFilePath` | `input.original_file_path` |
| `hookEventName` | `input.hook_event_name` |
| Signals | `input.signals.<name>` |

### Operation Mappings

| DSL Operation | Rego Output |
|---------------|-------------|
| `.equals(val)` | `path == val` |
| `.notEquals(val)` | `path != val` |
| `.contains(val)` | `contains(path, val)` |
| `.containsAny(arr)` | `some item in arr; contains(path, item)` |
| `.in(arr)` | `path in [arr]` |
| `.startsWith(val)` | `startswith(path, val)` |
| `.endsWith(val)` | `endswith(path, val)` |
| `.lower()` | `lower(path)` |
| `.upper()` | `upper(path)` |

### Test Results

```
 ✓ src/tests/pirate.test.ts      - Context with condition
 ✓ src/tests/symlink.test.ts     - Full deny chain with dynamic reason
 ✓ src/tests/techWrite.test.ts   - Signals + unconditional context
 ✓ src/tests/techWriterOnly.test.ts - canOnly with default deny
```

### Notable Behaviors

1. **Named Constants with defineConstant()**: Use `defineConstant()` to preserve semantic constant names:
   ```typescript
   const dangerousDirectories = defineConstant('dangerous_directories', ['/etc/', '/bin/', ...]);
   resolvedFilePath.lower().containsAny(dangerousDirectories)
   ```
   Compiles to:
   ```rego
   dangerous_directories := ["/etc/", "/bin/", ...]
   some dangerous in dangerous_directories
   contains(lower(resolved_path), lower(dangerous))
   ```

2. **Local variable shortening**: Preprocessing fields like `resolvedFilePath` are shortened to `resolved_path` (preserves semantic prefix, drops redundant "file"):
   - `resolvedFilePath` → `resolved_path`
   - `originalFilePath` → `original_path`

3. **Dynamic reasons with local vars**: When a field is used in both conditions and reason, the local variable is reused:
   ```rego
   resolved_path := input.resolved_file_path
   ...
   "reason": concat("", ["...'", resolved_path, "'..."])
   ```

4. **canOnly pattern**: Automatically generates default deny when allow rules exist

### Preprocessing Fields (from Cupcake Engine)

These are enriched fields added by the engine's preprocessing layer (TOB-4 defense):

| Field | Description |
|-------|-------------|
| `input.is_symlink` | Boolean - was the original path a symlink? |
| `input.resolved_file_path` | Canonical absolute path after symlink resolution |
| `input.original_file_path` | The path as originally provided |

These are different from `input.tool_input.path` (raw tool input).

### Future Considerations

- Signal dependency extraction (for orchestrator)
- Nested object field access
- Custom rule IDs vs auto-generated snake_case
