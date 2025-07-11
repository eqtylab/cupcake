# Plan 002 Implementation Understanding

Created: 2025-07-11T21:30:00Z

## Overview

Plan 002 implements the runtime evaluation engine for Cupcake - the core logic that transforms policy definitions into active enforcement. This is the heart of the system where policies come alive to guide and control Claude Code's behavior.

## Core Requirements Analysis

### Primary Goal
Build a fully functional, headless policy engine that can:
1. Receive hook events from Claude Code via stdin
2. Load and evaluate policies from multiple sources
3. Execute the two-pass evaluation model
4. Perform all V1 actions with proper semantics
5. Return decisions to Claude Code via exit codes/output

### Success Criteria Deep Dive

**Hook Event Processing**:
- Must handle all 6 hook event types: PreToolUse, PostToolUse, Notification, Stop, SubagentStop, PreCompact
- JSON deserialization from stdin with robust error handling
- Support for all tool types (Bash, Read, Write, Edit, Task, etc.)

**Policy Loading**:
- Load from `./cupcake.toml` (project policies) and `~/.claude/cupcake.toml` (user policies)
- Maintain proper ordering: project policies first, then user policies
- Handle missing files gracefully
- Support schema validation

**Two-Pass Evaluation Model**:
- **Pass 1**: Collect ALL soft feedback from ALL matching policies
- **Pass 2**: Find FIRST hard action from the ordered policy list
- Never stop early in Pass 1 - complete evaluation is critical
- Proper result aggregation as defined in feedback-aggregation.md

**Action Execution**:
- ProvideFeedback (soft) - just collect message
- BlockWithFeedback/Approve (hard) - make final decision
- RunCommand - execute with proper exit code handling, timeout, background support
- UpdateState - record custom events for future conditions
- Conditional - runtime condition evaluation with branching

**State Management**:
- Track all tool usage automatically (Read, Write, Edit, Bash operations)
- Store in `.cupcake/state/<session_id>.json` format
- Append-only event logging for session history
- Support state queries with time constraints

**Output Protocol**:
- Exit code 0: Allow operation (stdout to transcript mode only)
- Exit code 2: Block operation (stderr to Claude)
- Other exit codes: Non-blocking errors
- Support for advanced JSON output format

## Design Alignment Analysis

### Architecture Document Alignment
The implementation must strictly follow the runtime flow defined in architecture.md:
1. Receive hook event JSON via stdin ✓
2. Load policies from cache/files ✓
3. Build ordered policy list (project → user) ✓
4. Pass 1: Collect all soft feedback ✓
5. Pass 2: Find first hard action ✓
6. Combine results with proper formatting ✓
7. Update state file if needed ✓
8. Return unified response to Claude ✓

### Feedback Aggregation Alignment
Must implement the exact logic from feedback-aggregation.md:
- Hard action + soft feedback → Combined message with blocking
- Only soft feedback → Block with all feedback
- Only hard action → Block with hard message only
- Nothing triggered → Allow operation

### Hook Events Alignment
Perfect compliance with Claude Code's hook system:
- JSON input format matches Claude Code specification
- Exit code conventions match expected behavior
- Timeout handling (60s default)
- Tool matcher patterns for PreToolUse/PostToolUse

## Six-Phase Implementation Strategy

### Phase 1: Foundation (Hook Event Processing)
**Critical Path**: Establish stdin/stdout communication with Claude Code
- Hook event JSON parsing for all 6 event types
- Basic policy loading (structure only)
- Exit code response framework
- Error handling for malformed input

**Design Dependency**: Must match hook-events.md specifications exactly

### Phase 2: Condition Engine
**Critical Path**: Implement all condition evaluation logic
- Regex matchers with proper flag handling
- Logical operators (And, Or, Not) with short-circuit evaluation
- File system conditions (exists, modified, etc.)
- Time-based conditions with timezone support

**Design Dependency**: Must support all condition types in policy-schema.md

### Phase 3: State Management
**Critical Path**: Enable stateful policy evaluation
- Session state file creation and management
- Automatic tool usage tracking
- State query engine for complex conditions
- Cleanup on session end

**Design Dependency**: Append-only state model from architecture.md

### Phase 4: Two-Pass Evaluation
**Critical Path**: Core policy engine logic
- Pass 1: Complete soft feedback collection
- Pass 2: First hard action detection
- Result aggregation with proper formatting
- Policy ordering enforcement

**Design Dependency**: Exact implementation of feedback-aggregation.md

### Phase 5: Action Execution
**Critical Path**: Make policies actionable
- Command execution with process spawning
- Template variable substitution
- Timeout and background execution
- State modification actions

**Design Dependency**: Command execution patterns from design docs

### Phase 6: Integration
**Critical Path**: Production readiness
- End-to-end integration
- Performance optimization (sub-100ms target)
- Comprehensive error handling
- Debug and audit features

## Technical Architecture Decisions

### Core Abstractions
```rust
// Evaluation pipeline
struct PolicyEngine {
    loader: PolicyLoader,           // Load policies from files
    evaluator: PolicyEvaluator,     // Two-pass evaluation logic
    executor: ActionExecutor,       // Execute actions
    state_manager: StateManager,    // Session state tracking
}

// Context passing
struct EvaluationContext {
    hook_event: HookEvent,         // Incoming event
    session_id: String,            // Session identifier
    policies: Vec<Policy>,         // Loaded policies
    state: SessionState,           // Current session state
}
```

### Pattern Applications
- **Builder**: Complex PolicyEvaluator construction
- **Strategy**: Pluggable condition evaluators
- **Chain of Responsibility**: Two-pass evaluation sequence
- **Template**: Action execution with variable substitution

### Performance Considerations
- Lazy regex compilation with caching
- Minimal state file I/O (read once, append only)
- Early termination in Pass 2 (but never in Pass 1)
- Pre-allocated buffers for stdin reading
- Sub-100ms response time target

## Critical Success Factors

### 1. Correctness
- Perfect adherence to two-pass evaluation model
- Exact feedback formatting as specified
- Proper policy ordering (project → user)
- Complete condition evaluation coverage

### 2. Performance
- Sub-100ms response time for typical operations
- Efficient regex compilation and caching
- Minimal memory allocation during evaluation
- Fast state file access

### 3. Reliability
- Graceful degradation on errors (allow on failure)
- No panics in production code
- Proper timeout handling
- Clear error messages to Claude

### 4. Maintainability
- Clean separation of concerns
- Comprehensive test coverage
- Clear abstractions and interfaces
- Extensible design for future features

## Risk Analysis

### High-Risk Areas
1. **Two-Pass Logic**: Complex evaluation model with specific ordering requirements
2. **State Management**: File-based state with concurrency considerations
3. **Command Execution**: Process spawning with security implications
4. **Performance**: Sub-100ms constraint with complex evaluation

### Mitigation Strategies
1. Extensive unit testing for two-pass logic
2. Append-only state design to avoid corruption
3. Proper command escaping and sandboxing
4. Performance benchmarking throughout development

## Implementation Validation

### Unit Testing Focus
- Each condition type with edge cases
- Two-pass evaluation with various policy combinations
- Action execution with different exit codes
- State management with concurrent access

### Integration Testing Focus
- Real policy files with known outcomes
- Complete stdin/stdout communication
- Performance under load
- Error injection scenarios

### Key Test Scenarios
1. Empty policy file → Allow all operations
2. Complex nested conditions → Correct boolean evaluation
3. Multiple soft feedback → Proper aggregation
4. Hard action priority → First hard action wins
5. Command timeout → Graceful handling
6. State queries → Accurate historical lookups

## Conclusion

Plan 002 is the most critical implementation phase, transforming the solid foundation of Plan 001 into a working policy enforcement engine. Success requires perfect alignment with design specifications, particularly the two-pass evaluation model and feedback aggregation logic. The phased approach ensures incremental validation while maintaining engineering excellence throughout the implementation.

The key to success lies in meticulous attention to the design documents, comprehensive testing at each phase, and never compromising on the core architectural principles that make Cupcake both powerful and reliable.