# TUI Extraction Process - Synthesis & Next Steps

## Current State Analysis

### Existing ExtractedRule Structure
The current implementation has a decent foundation:
```rust
ExtractedRule {
    description: String,        // The rule text
    hook_description: String,   // What the hook does
    severity: Severity,         // High/Medium/Low
    category: String,          // testing/security/code-style etc
    when: String,              // pre-commit/file-change/tool-call
    block_on_violation: bool,  // Whether to block
    policy_decision: PolicyDecision,
}
```

### What's Missing
To support intelligent extraction as described, we need:
1. **Original rule text** - Verbatim from the source file
2. **Hook event mapping** - PreToolUse/PostToolUse/UserPromptSubmit etc
3. **Tool matcher pattern** - What tools trigger this (e.g., "Bash", "Write|Edit")
4. **Condition details** - Specific patterns/checks to evaluate
5. **Action specifics** - Command to run, feedback message, etc
6. **Discovered utilities** - Found lint/test commands
7. **Custom script needs** - When standard conditions aren't enough

## The Meta-Program Structure

### Phase 1: File Processing Loop
```
For each selected rule file:
  1. Spawn async extraction task
  2. Read file content
  3. Call LLM with extraction prompt
  4. Parse natural language rules
  5. Analyze codebase context
  6. Generate ExtractedRule structures
  7. Send progress updates
```

### Phase 2: Intelligent Extraction
The LLM extraction needs to:
1. **Parse** - Find all rules (explicit and implicit)
2. **Contextualize** - Look at codebase to understand intent
3. **Map** - Determine appropriate hooks and tools
4. **Discover** - Find existing utilities (npm scripts, make targets)
5. **Structure** - Create comprehensive rule definitions

## Prompt Engineering Needs

### The Master Extraction Prompt
Following the meta-framework:

```
MISSION: Extract enforceable policies from natural language developer rules

ROLE: You are a policy extraction specialist who understands both natural 
language requirements and Claude Code's hook system.

CONTEXT: 
- Rule file content: {{RULE_FILE}}
- Available hooks: PreToolUse, PostToolUse, UserPromptSubmit, etc.
- Codebase context: {{RELEVANT_FILES}}

WORKFLOW:
1. Parse all rules (explicit "must/should" and implicit patterns)
2. For each rule, determine:
   - What the developer wants to enforce
   - When it should be enforced (which hook)
   - How to detect violations (conditions)
   - What action to take (block/feedback/autofix)
3. Analyze codebase for existing tools (lint, test, format)
4. Map rules to enforcement mechanisms
5. Verify each rule is enforceable

OUTPUT SCHEMA: [Detailed JSON structure for ExtractedRule]

VERIFICATION: Ensure each rule maintains original intent while being technically enforceable
```

## Data Structure Enhancement

### Enhanced ExtractedRule
```rust
struct ExtractedRule {
    // Original
    original_text: String,      // Verbatim rule from source
    
    // Core rule info
    description: String,        // Clear one-line summary
    category: String,          // testing/security/style/workflow
    severity: Severity,        // High/Medium/Low
    
    // Hook integration
    hook_event: HookEventType, // PreToolUse/PostToolUse/etc
    tool_matcher: String,      // "Bash", "Write|Edit", "*"
    
    // Enforcement details
    conditions: Vec<Condition>, // Patterns to match, commands to run
    action_type: ActionType,   // block/feedback/inject/autofix
    action_details: ActionDetails, // Commands, messages, etc
    
    // Metadata
    discovered_utilities: Vec<String>, // Found commands
    requires_custom_script: bool,
    policy_decision: PolicyDecision,
}
```

## UI/UX Requirements

### Extraction Progress Screen
- Show each file being processed
- Live extraction count per file
- Visual indicators for rule categories
- Error handling for failed extractions

### Review & Edit Screen
- Clear presentation of:
  - Original rule (quoted)
  - How Cupcake will enforce it
  - Technical details (collapsible)
- Inline editing with:
  - Dropdowns for hooks/actions
  - Text fields for patterns/messages
  - Toggle for enable/disable

## Implementation Path

### Step 1: Enhance Data Structures
- Update ExtractedRule to support all needed fields
- Create proper enums for HookEventType, ActionType
- Design Condition and ActionDetails types

### Step 2: Create Extraction Prompt
- Build prompt using meta-framework
- Include examples of good extractions
- Add verification steps

### Step 3: Wire Up LLM Integration
- Replace stub generation with actual LLM calls
- Add codebase analysis utilities
- Implement progress tracking

### Step 4: Build Review UI
- Enhanced display of extracted rules
- Editing capabilities
- Validation logic

## Key Success Metrics

1. **Extraction Quality** - Rules accurately captured with correct enforcement
2. **User Understanding** - Clear presentation of what will happen
3. **Edit Efficiency** - Minimal corrections needed
4. **Performance** - Async extraction completes reasonably fast

## Critical Insight

The extraction process is not just "parse and convert" - it's an intelligent analysis that:
- Understands developer intent
- Leverages existing tooling
- Creates deterministic guarantees
- Maintains human control

This is what makes Cupcake valuable - it bridges the gap between "what developers write" and "what can be enforced" while keeping the human in the loop for critical decisions.