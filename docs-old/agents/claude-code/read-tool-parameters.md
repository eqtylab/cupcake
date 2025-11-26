# Claude Code Read Tool Parameters

## Overview

The Read tool in Claude Code supports optional `offset` and `limit` parameters for efficient file navigation, especially useful for large files. These parameters are captured in Cupcake's debug logs, providing visibility into Claude's file access patterns.

## Parameter Usage

### Basic Read (No Parameters)
When reading a file without parameters, only the file path is sent:
```json
{
  "tool_input": {
    "file_path": "/path/to/file.rs"
  }
}
```
This reads up to 2000 lines from the beginning of the file.

### Targeted Read (With Parameters)
When using offset and limit, both appear in the tool input:
```json
{
  "tool_input": {
    "file_path": "/path/to/file.rs",
    "limit": 30,
    "offset": 500
  }
}
```

### Parameter Definitions
- **offset**: Line number to start reading from (1-indexed)
- **limit**: Maximum number of lines to read from that offset
- **Default behavior**: Without parameters, reads up to 2000 lines from the beginning

## Common Usage Patterns

### 1. File Header Inspection
```json
{
  "offset": 1,
  "limit": 50
}
```
Used to understand file structure, imports, and initial documentation.

### 2. Specific Section Reading
```json
{
  "offset": 500,
  "limit": 30
}
```
Targets a specific area of the file, often after using Grep to locate relevant code.

### 3. End of File Inspection
```json
{
  "offset": 1490,
  "limit": 25
}
```
Reads the conclusion of a file, often containing exports, tests, or final definitions.

## Debug Log Examples

### Example 1: Full File Read
```
===== Claude Code Event [2025-09-06 23:33:18] [019922e1-0c53-7923-9472-638f9cd91273] =====
Event Type: PreToolUse
Tool: Read
...
  "tool_input": {
    "file_path": "/Users/ramos/cupcake/cupcake-rego/cupcake-rewrite/cupcake-core/src/engine/mod.rs"
  },
```

### Example 2: Targeted Read with Parameters
```
===== Claude Code Event [2025-09-06 23:46:13] [019922ec-dfff-7422-ab5c-a0275b3e6fd1] =====
Event Type: PreToolUse
Tool: Read
...
  "tool_input": {
    "file_path": "/Users/ramos/cupcake/cupcake-rego/cupcake-rewrite/cupcake-core/src/engine/mod.rs",
    "limit": 30,
    "offset": 500
  },
```

## When Claude Uses These Parameters

### Situations for Parameterized Reads

1. **Large Files (3000+ lines)**
   - Iterative exploration using sliding windows
   - Avoids loading entire file into context

2. **Search-Guided Reading**
   - After Grep shows matches at specific lines
   - Uses offset to read context around matches

3. **Error Investigation**
   - When error messages reference specific line numbers
   - Reads targeted sections for debugging

4. **Code Navigation**
   - Following imports or function definitions
   - Reading specific classes or modules

5. **Performance Optimization**
   - Minimizing token usage in responses
   - Focusing on relevant code sections

### Typical Exploration Pattern

1. **Initial scan**: `offset: 1, limit: 100` - Understand structure
2. **Middle exploration**: `offset: 500, limit: 50` - Check implementation
3. **End check**: `offset: 1400, limit: 100` - See conclusions/tests
4. **Targeted reads**: Based on search results or error lines

## Policy Implications

When writing Cupcake policies for the Read tool, you can inspect these parameters to:

1. **Detect scanning behavior** - Multiple reads with increasing offsets might indicate file scanning
2. **Limit large reads** - Block or warn when limit exceeds certain thresholds
3. **Protect sensitive sections** - Deny reads of specific offset ranges in sensitive files
4. **Track access patterns** - Log which parts of files are being accessed

### Example Policy Using Parameters

```rego
package cupcake.policies.read_limits

import rego.v1

# Warn on very large read operations
ask contains decision if {
    input.tool_name == "Read"
    input.tool_input.limit > 500
    decision := {
        "rule_id": "READ-LARGE-001",
        "reason": sprintf("Large read operation requested: %d lines", [input.tool_input.limit]),
        "question": "This read operation will load many lines. Continue?",
        "severity": "LOW"
    }
}

# Block reading specific offset ranges in sensitive files
deny contains decision if {
    input.tool_name == "Read"
    contains(input.tool_input.file_path, ".env")
    input.tool_input.offset > 1
    input.tool_input.offset < 100  # Assuming secrets are in first 100 lines
    decision := {
        "rule_id": "READ-SENSITIVE-001",
        "reason": "Cannot read sensitive sections of environment files",
        "severity": "HIGH"
    }
}
```

## Debugging with Parameters

The debug logs capture these parameters, making it easy to:

1. **Trace file access patterns** - See exactly what Claude read and when
2. **Understand decision context** - Why Claude chose specific sections
3. **Optimize policies** - Target specific read patterns
4. **Audit file access** - Complete record of what was accessed

Enable debug logging with:
```bash
cupcake eval --debug-files < event.json
```

Debug files in `.cupcake/debug/` will show complete tool_input including any offset/limit parameters.