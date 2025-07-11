# CRITICAL: Fundamental Architecture Misalignment

Created: 2025-01-11T12:00:00Z
Priority: CRITICAL
Type: Architecture Course Correction

## Executive Summary

We built the wrong thing. We hardcoded 15 specific condition types when we needed 3 generic ones.

## The Problem

### What We Built
```rust
enum Condition {
    CommandRegex { ... },        // git commit
    FilepathRegex { ... },       // \.rs$
    StateExists { tool, path },  // Did Claude read X?
    FileModifiedWithin { ... },  // File changed recently?
    DayOfWeek { days },         // No weekend commits
    TimeWindow { ... },         // 9-5 only
    EnvVarEquals { ... },       // NODE_ENV=production
    // ... 15 total types
}
```

### Why It's Wrong
1. **Mistook Examples for Requirements**: Design docs showed "no weekend commits" as an EXAMPLE, not a feature request
2. **Hardcoded Instead of Enabled**: Built `DayOfWeek` instead of letting users run `date +%a`
3. **Incomplete Implementation**: Converter only handles 4/15 types - rest return never-matching placeholders
4. **Maintenance Nightmare**: Every new check type requires core changes

## The Solution

### What We Should Build
```rust
enum Condition {
    // Pattern matching (90% of use cases)
    Match { field: String, value: String },           // tool_name == "Bash"
    Pattern { field: String, regex: String },         // command =~ "git.*"
    
    // Command execution (everything else)
    Check { command: String, expect_success: bool },  // ./scripts/validate.sh
    
    // Composition
    Not { condition: Box<Condition> },
    And { conditions: Vec<Condition> },
    Or { conditions: Vec<Condition> },
}
```

### Why It's Right
1. **Universal**: Can express ANY policy with 3 primitives
2. **Repository-Native**: Leverages existing scripts/tools
3. **Simple**: No complex state queries or custom events
4. **Maintainable**: No new core types needed, ever

## Real Example

### Current (Broken)
```toml
# This doesn't work - converter returns placeholder
conditions = [
  { type = "state_exists", tool = "Read", path = "README.md" }
]
```

### Proposed (Working)
```toml
conditions = [
  { type = "check", command = "grep -q 'Read.*README.md' .cupcake/state/{{session_id}}.json" }
]
```

## Impact

- **Code Reduction**: ~70% less code
- **Bug Reduction**: No converter complexity
- **Feature Velocity**: Users can implement any check without waiting for us
- **Migration Path**: Existing policies can be mechanically converted

## Recommendation

Pause current development. Refactor to 3-primitive model before proceeding.

## Evidence

- Design doc shows command execution as primary pattern
- Current implementation already has working `run_command` 
- 11/15 condition types are just shell one-liners
- Converter is already a critical bug source

## Decision Required

Continue building complex hardcoded types or pivot to simple generic model?