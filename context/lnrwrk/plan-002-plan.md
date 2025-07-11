# Plan for Plan 002: Runtime Evaluation and Action System

Created: 2025-07-11T21:00:00Z

## Approach

Build the runtime evaluation engine in carefully orchestrated phases that enable incremental validation and maintain engineering excellence. Each phase delivers a working subset of functionality that can be independently tested and verified against the design specifications.

## Phases

### Phase 1: Hook Event Processing and Basic Infrastructure

**Goal**: Establish the foundation for receiving and processing Claude Code hook events.

**Deliverables**:
1. **Hook event deserialization from stdin**
   - Implement stdin reading with proper JSON parsing
   - Handle all 6 hook event types with full payload deserialization
   - Create robust error handling for malformed input
   
2. **Policy loading infrastructure**
   - Load from `./cupcake.toml` and `~/.claude/cupcake.toml`
   - Implement proper ordering (project policies first)
   - Add caching preparation (structure only, not binary cache yet)
   
3. **Basic response framework**
   - Implement exit code conventions (0=allow, 2=block)
   - Create structured output formatting for stdout/stderr
   - Add debug output capabilities

**Verification**:
- Unit tests for JSON deserialization of all hook event types
- Integration test: `echo '{"hook_event_name":"PreToolUse",...}' | cargo run -- run`
- Verify proper error handling for invalid JSON

### Phase 2: Condition Evaluation Engine

**Goal**: Implement the complete condition matching logic for all condition types.

**Deliverables**:
1. **Basic matchers**
   - CommandRegex, FilepathRegex, FilepathGlob
   - FileContentRegex with multiline support
   - MessageContains for Notification events
   
2. **Logical operators**
   - Not, And, Or with proper nesting
   - Short-circuit evaluation for performance
   
3. **File system conditions**
   - FileExists, FileModifiedWithin
   - WorkingDirContains, EnvVarEquals
   
4. **Time-based conditions**
   - TimeWindow with timezone support
   - DayOfWeek validation

**Verification**:
- Comprehensive unit tests for each condition type
- Test complex nested conditions (And/Or/Not combinations)
- Performance benchmarks for regex compilation

### Phase 3: State Management System

**Goal**: Implement session state tracking and querying.

**Deliverables**:
1. **State file structure**
   - Create `.cupcake/state/<session_id>.json` format
   - Implement append-only event logging
   - Add automatic cleanup on session end
   
2. **Automatic tool tracking**
   - Record all Read, Write, Edit, Bash operations
   - Capture tool input/output for state queries
   - Track success/failure status
   
3. **State query engine**
   - StateExists, StateMissing, StateQuery conditions
   - Time-based queries (within_minutes, since)
   - Custom event queries

**Verification**:
- Test state file creation and appending
- Verify tool usage tracking accuracy
- Test complex state queries with time constraints

### Phase 4: Two-Pass Evaluation Logic

**Goal**: Implement the core two-pass policy evaluation model.

**Deliverables**:
1. **Pass 1: Feedback collection**
   - Iterate through all policies
   - Evaluate conditions and collect soft feedback
   - Never stop early, always complete full pass
   
2. **Pass 2: Hard action detection**
   - Re-iterate to find first hard action
   - Implement proper action type classification
   - Handle conditional actions correctly
   
3. **Result aggregation**
   - Combine all soft feedback with hard action
   - Format messages according to design spec
   - Preserve feedback ordering

**Verification**:
- Test multiple soft feedback aggregation
- Test hard action priority (first wins)
- Test mixed soft + hard scenarios
- Verify feedback formatting matches spec

### Phase 5: Action Execution System

**Goal**: Implement all action types with proper execution semantics.

**Deliverables**:
1. **Basic actions**
   - ProvideFeedback (soft)
   - BlockWithFeedback, Approve (hard)
   - Template variable substitution
   
2. **Command execution**
   - RunCommand with process spawning
   - Capture stdout/stderr properly
   - Handle timeout and background execution
   - Exit code interpretation (on_failure behavior)
   
3. **State modification**
   - UpdateState action implementation
   - Custom event recording
   - Proper JSON serialization of data
   
4. **Conditional actions**
   - Runtime condition evaluation
   - Branch selection logic
   - Nested action execution

**Verification**:
- Test each action type in isolation
- Test command execution with various exit codes
- Test template variable substitution
- Test conditional action branching

### Phase 6: Integration and Polish

**Goal**: Complete end-to-end integration and production readiness.

**Deliverables**:
1. **Full integration**
   - Wire all components together
   - End-to-end testing with real policies
   - Performance optimization (target: sub-100ms)
   
2. **Error handling**
   - Graceful degradation on errors
   - Clear error messages to Claude
   - Timeout handling (60s limit)
   
3. **Debug and audit features**
   - Debug mode with detailed output
   - Audit logging preparation
   - Performance metrics

**Verification**:
- End-to-end tests with complex policies
- Performance benchmarks (must be sub-100ms)
- Error injection testing
- Memory usage analysis

## Technical Decisions

### Architecture Patterns
- **Builder pattern** for complex object construction (PolicyEvaluator)
- **Strategy pattern** for condition evaluators
- **Chain of responsibility** for two-pass evaluation
- **Template pattern** for action executors

### Key Abstractions
```rust
// Core traits
trait ConditionEvaluator {
    fn evaluate(&self, context: &EvaluationContext) -> Result<bool>;
}

trait ActionExecutor {
    fn execute(&self, context: &mut ExecutionContext) -> Result<ActionResult>;
}

// Main components
struct PolicyEngine {
    loader: PolicyLoader,
    evaluator: PolicyEvaluator,
    executor: ActionExecutor,
    state_manager: StateManager,
}
```

### Performance Considerations
- Lazy regex compilation with caching
- Minimal state file I/O (read once, append only)
- Early termination in Pass 2
- Pre-allocated buffers for stdin reading

### Error Philosophy
- Never panic in production code
- All errors bubble up with context
- Graceful degradation (allow on error)
- Clear, actionable error messages

## Testing Strategy

### Unit Testing
- Each component tested in isolation
- Mock interfaces for dependencies
- Property-based testing for complex logic
- Edge case coverage

### Integration Testing
- Real policy files with known outcomes
- Simulated hook events via stdin
- State file verification
- Command execution with mock scripts

### Critical Test Scenarios
1. **Empty policy file** - Should allow all operations
2. **Malformed JSON input** - Should allow with error message
3. **Missing state directory** - Should create automatically
4. **Command timeout** - Should handle gracefully
5. **Circular conditions** - Should detect and error
6. **Large policy files** - Should maintain performance

## Success Metrics

1. **Functionality**: All success criteria from Plan 002 definition met
2. **Performance**: Sub-100ms response time for typical operations
3. **Reliability**: No panics, graceful error handling
4. **Maintainability**: Clean abstractions, comprehensive tests
5. **Alignment**: Perfect adherence to design specifications

## Risk Mitigation

1. **Regex Performance**: Pre-compile and cache all patterns
2. **State File Growth**: Implement rotation/cleanup strategies
3. **Command Injection**: Use proper escaping for command execution
4. **Deadlocks**: Careful ordering of file operations
5. **Memory Usage**: Stream large inputs, don't load fully

## Notes

- This plan emphasizes incremental delivery with validation at each phase
- Each phase builds on the previous, minimizing rework
- Testing is integrated throughout, not left to the end
- Performance is considered from the beginning, not optimized later
- The design respects all constraints from the architecture documents