# Plan 019: Cupcake System Discovery - Claude Code July 20 Integration Analysis

Created: 2025-01-25T10:45:00Z
Type: Discovery Todo List

## Objective

Conduct a comprehensive analysis of the Cupcake codebase to understand:
1. Current hook integration implementation
2. How Cupcake generates and manages hook configurations
3. Opportunities to leverage July 20 Claude Code updates
4. Technical approach for implementing new capabilities

## Discovery Todos

### 1. Hook Registration & Configuration Analysis
**Goal**: Understand how Cupcake currently registers with Claude Code hooks

- [ ] Examine `src/cli/commands/sync.rs` - How does sync command work?
- [ ] Find where hook configurations are generated (JSON structure)
- [ ] Identify which hook events Cupcake currently uses
- [ ] Check if Cupcake uses matchers and how they're implemented
- [ ] Look for `.claude/settings.json` manipulation code
- [ ] Understand the "empty string" vs wildcard matcher usage

### 2. Policy Execution Model Analysis
**Goal**: Understand how policies map to hook responses

- [ ] Analyze `src/engine/response.rs` - Current response types
- [ ] Check how `PolicyDecision` enum maps to exit codes
- [ ] Find JSON output generation (if any exists)
- [ ] Examine `block_with_feedback` action implementation
- [ ] Understand current exit code usage (0, 2, others)
- [ ] Look for any existing JSON response structures

### 3. UserPromptSubmit Integration Check
**Goal**: Determine if Cupcake uses this hook and how

- [ ] Search for "UserPromptSubmit" in codebase
- [ ] Check if policies can target prompt events
- [ ] Look for any context injection mechanisms
- [ ] Analyze if state manager tracks prompts
- [ ] Find any prompt validation logic

### 4. State Management & Context Awareness
**Goal**: Understand stateful capabilities for context injection

- [ ] Examine `src/state/` module structure
- [ ] Understand what events are tracked in state
- [ ] Check how state influences policy decisions
- [ ] Look for session-aware logic that could feed context
- [ ] Analyze state query capabilities for dynamic decisions

### 5. Action System Capabilities
**Goal**: Map current actions to new Claude Code capabilities

- [ ] Review all action types in `src/config/actions.rs`
- [ ] Check if there's an "inject_context" action type
- [ ] Understand `run_command` action implementation
- [ ] Look for "approve" or permission-related actions
- [ ] Find feedback aggregation logic (two-pass system)

### 6. Command Execution & Environment
**Goal**: Understand how Cupcake executes commands

- [ ] Examine `src/engine/command_executor/`
- [ ] Check environment variable handling
- [ ] Look for timeout configuration support
- [ ] Find working directory management
- [ ] Check for any `$CLAUDE_PROJECT_DIR` usage

### 7. YAML Policy Schema Analysis
**Goal**: Understand policy format and extensibility

- [ ] Review policy schema in `src/config/types.rs`
- [ ] Check if schema supports new decision types
- [ ] Look for hookSpecificOutput support
- [ ] Analyze condition types for new possibilities
- [ ] Find policy validation logic

### 8. Runtime Hook Handler Analysis
**Goal**: Understand the `cupcake run` command deeply

- [ ] Trace full execution path of `run` command
- [ ] Understand stdin JSON parsing
- [ ] Check stdout/stderr handling
- [ ] Find exit code decision logic
- [ ] Look for JSON output capabilities
- [ ] Analyze error handling and graceful degradation

### 9. MCP Tool Awareness
**Goal**: Check if Cupcake handles MCP tools

- [ ] Search for "mcp__" pattern references
- [ ] Check if matcher patterns handle MCP naming
- [ ] Look for any MCP-specific logic
- [ ] Understand tool name parsing

### 10. Current Integration Points
**Goal**: Map all current Claude Code touchpoints

- [ ] Find all references to Claude Code hooks
- [ ] Check for hook event constants/enums
- [ ] Look for Claude Code specific paths
- [ ] Understand settings.json structure assumptions
- [ ] Find any hardcoded hook behaviors

### 11. Security Model Implementation
**Goal**: Understand Cupcake's security approach

- [ ] Check for configuration validation
- [ ] Look for path traversal protection
- [ ] Find command injection prevention
- [ ] Understand permission model
- [ ] Check for any security-related policies

### 12. Test Coverage Analysis
**Goal**: Understand how hooks are tested

- [ ] Find hook-related tests
- [ ] Check for UserPromptSubmit tests
- [ ] Look for JSON output tests
- [ ] Find integration tests with Claude Code
- [ ] Check for exit code behavior tests

## Analysis Questions to Answer

### Architecture Questions
1. Does Cupcake generate different hook configurations per event type?
2. How does Cupcake decide between exit codes vs JSON responses?
3. Is the architecture ready for context injection features?
4. Can policies target specific MCP tools without changes?

### Implementation Questions
1. Where would context injection logic live?
2. How to add `permissionDecision` support?
3. Can we support both old and new JSON formats?
4. Where to implement `ask` permission handling?

### Integration Questions
1. Does Cupcake need to know about `$CLAUDE_PROJECT_DIR`?
2. Should Cupcake support per-command timeouts?
3. How to handle the security snapshot model?
4. Where to add PreCompact hook support?

## Final Deliverable

### Discovery Report Structure
1. **Current State Analysis**
   - How Cupcake currently integrates with Claude Code
   - Which hooks and features are used
   - Architecture strengths and limitations

2. **Gap Analysis**
   - July 20 features not yet leveraged
   - Architectural changes needed
   - Policy schema extensions required

3. **Integration Opportunities**
   - UserPromptSubmit for proactive guidance
   - Context injection for dynamic policies  
   - Ask permission for user education
   - MCP tool policy support

4. **Technical Recommendations**
   - Minimal changes for maximum value
   - Backward compatibility approach
   - Priority implementation order

5. **Implementation Roadmap**
   - Quick wins (low effort, high value)
   - Core enhancements (medium effort)
   - Advanced features (higher effort)

## Success Criteria

The discovery is complete when I can:
1. Explain exactly how Cupcake integrates with Claude Code today
2. Identify all gaps between current implementation and July 20 features
3. Propose specific code changes to leverage new capabilities
4. Provide a prioritized implementation plan
5. Understand any architectural constraints or required refactoring