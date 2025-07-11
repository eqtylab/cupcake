# Plan 002 Pivot: 3-Primitive Condition Model

Created: 2025-01-11T14:00:00Z
Priority: CRITICAL
Type: Architecture Pivot

## Summary

Pivot from 15+ hardcoded condition types to a simple 3-primitive model that aligns with the original design intent. This maintains full support for all patterns in `command-execution-patterns.md` while dramatically simplifying the codebase.

## Background

The critical interjection identified that we misunderstood the design intent. The design documents consistently show command execution as the primary pattern for complex checks, not hardcoded condition types.

**Evidence from design docs:**
- `command-execution-patterns.md` shows all complex checks via shell commands
- Original `idea0.md` emphasizes Unix philosophy and simplicity
- Reference implementation at `~/.cupcake-rs` successfully uses 3-primitive model

**Current problems:**
- Only 4 of 15 condition types actually work
- Converter returns placeholder "never-match" for 11 types
- 1,067 lines of complex condition code, mostly non-functional
- Every new requirement needs core changes

## The 3-Primitive Model

```rust
enum Condition {
    // Field matching (covers 90% of use cases)
    Match { 
        field: String,      // "tool_name", "event_type", etc.
        value: String       // "Bash", "PreToolUse", etc.
    },
    
    // Pattern matching with regex
    Pattern { 
        field: String,      // "command", "file_path", "content", etc.
        regex: String       // Any regex pattern
    },
    
    // Command execution (covers everything else)
    Check { 
        command: String,           // Shell command to execute
        expect_success: bool       // true = exit 0 means match
    },
    
    // Composition (unchanged)
    Not { condition: Box<Condition> },
    And { conditions: Vec<Condition> },
    Or { conditions: Vec<Condition> },
}
```

## Design Alignment

This model **fully supports** all patterns from `command-execution-patterns.md`:

### 1. File Matching (Pattern primitive)
```toml
# Old way (still works)
conditions = [
  { type = "filepath_glob", value = "*.tsx" }
]

# New way
conditions = [
  { type = "pattern", field = "tool_input.file_path", regex = "\\.tsx$" }
]
```

### 2. Command Execution (Check primitive)
```toml
# Exactly as shown in command-execution-patterns.md
conditions = [
  { type = "check", command = "./scripts/validate.sh {{tool_input.file_path}}", expect_success = true }
]
```

### 3. Content Matching (Pattern primitive)
```toml
# Old way
conditions = [
  { type = "file_content_regex", value = "console\\.log" }
]

# New way
conditions = [
  { type = "pattern", field = "tool_input.content", regex = "console\\.log" }
]
```

### 4. Complex Checks (Check primitive)
```toml
# All the examples from design docs work unchanged
conditions = [
  # Day of week check
  { type = "check", command = "[ $(date +%u) -le 5 ]", expect_success = true },
  
  # Time window check
  { type = "check", command = "hour=$(date +%H); [ $hour -ge 9 ] && [ $hour -lt 17 ]", expect_success = true },
  
  # State query
  { type = "check", command = "grep -q 'Read.*README.md' .cupcake/state/{{session_id}}.json", expect_success = true },
  
  # File exists
  { type = "check", command = "[ -f /path/to/file ]", expect_success = true },
  
  # Environment check
  { type = "check", command = "[ \"$NODE_ENV\" = \"production\" ]", expect_success = true }
]
```

## Implementation Plan

### Phase 1: Core Refactoring (Day 1-2)

1. **Create unified condition type** (`src/config/conditions.rs`):
   - Delete all 15+ existing condition types
   - Implement single 3-primitive enum
   - Keep logical operators (And/Or/Not)

2. **Implement evaluation** (`src/engine/conditions.rs`):
   - Field extraction from EvaluationContext
   - Pattern matching with regex caching
   - Command execution with proper escaping
   - Template variable substitution

3. **Remove converter complexity** (`src/engine/evaluation.rs`):
   - Delete the broken `convert_condition_for_evaluation` function
   - Use conditions directly without conversion
   - Simplify PolicyEvaluator

### Phase 2: Integration (Day 2-3)

1. **Wire up Check execution**:
   - Reuse existing RunCommand infrastructure
   - Add timeout support (default 5s for checks)
   - Capture exit codes for expect_success logic

2. **Update state integration**:
   - State queries become Check commands
   - Keep existing StateManager for automatic tracking
   - Document grep patterns for common queries

3. **Update tests**:
   - Convert existing tests to 3-primitive model
   - Add comprehensive Check command tests
   - Ensure all design doc examples work

### Phase 3: Polish (Day 3-4)

1. **Update documentation**:
   - Revise policy-schema.md
   - Add migration examples
   - Create cookbook of common patterns

2. **Performance validation**:
   - Benchmark Check command overhead
   - Ensure sub-100ms target maintained
   - Add caching for repeated checks

## Benefits

- **70% code reduction**: Eliminate ~750 lines of complex condition code
- **100% functionality**: All use cases work immediately
- **Zero maintenance**: No new condition types ever needed
- **User empowerment**: Any check possible via shell commands
- **Design alignment**: Matches original Unix philosophy intent

## Example: Complete Policy

Shows how all design patterns work with 3-primitive model:

```toml
[[policy]]
name = "TypeScript validation pipeline"
hook_event = "PreToolUse"
matcher = "Write|Edit"
conditions = [
  # File matching
  { type = "pattern", field = "tool_input.file_path", regex = "\\.tsx?$" },
  
  # Working hours only
  { type = "check", command = "hour=$(date +%H); [ $hour -ge 9 ] && [ $hour -lt 17 ]", expect_success = true },
  
  # Must have read docs first
  { type = "check", command = "grep -q 'Read.*coding-standards.md' .cupcake/state/{{session_id}}.json", expect_success = true }
]

# Action unchanged - run_command already perfect
action = {
  type = "run_command",
  command = """
    pnpm lint {{tool_input.file_path}} && \
    pnpm typecheck {{tool_input.file_path}} && \
    ./scripts/custom-validation.sh {{tool_input.file_path}}
  """,
  on_failure = "block",
  on_failure_feedback = "Validation failed. See errors above."
}
```

## Risk Mitigation

- **No breaking changes**: New model supports all existing patterns
- **Gradual rollout**: Can run both models during transition
- **Escape hatch**: Check primitive can implement any logic

## Success Criteria

- All examples from `command-execution-patterns.md` work
- Performance remains under 100ms
- Code complexity reduced by >50%
- No loss of functionality
- Tests pass with new model

## Next Steps

1. Create feature branch for pivot
2. Implement Phase 1 (core refactoring)
3. Validate all design doc examples work
4. Complete integration and testing
5. Update documentation

This pivot aligns perfectly with the original design vision while maintaining full compatibility with all documented patterns.