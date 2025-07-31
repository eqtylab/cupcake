# TUI Extraction Process - Action Items & Considerations

## 1. Prompt Engineering Requirements

### Primary Extraction Prompt
We need a sophisticated prompt following the meta-framework that:
- **Mission**: Extract enforceable policies from natural language rules
- **Role**: Policy extraction specialist with codebase analysis capabilities
- **Decomposition**: 
  1. Parse natural language rules
  2. Analyze codebase for context
  3. Identify enforcement mechanisms
  4. Map to hooks and actions
  5. Verify feasibility
- **Output Schema**: Structured JSON for rule requirement scoping
- **Verification Hooks**: Ensure each rule has complete enforcement details

### Key Prompt Capabilities
- Must instruct LLM to look for implicit rules, not just explicit ones
- Should identify existing tooling in the codebase (lint, test, format commands)
- Need to categorize rules by enforcement type (block vs guide vs automate)
- Must maintain original rule intent while adding enforcement details

## 2. Data Structure Requirements

### Rule Requirement Scoping Structure
Need to define/enhance structure containing:
```
- rule_text: Original natural language
- rule_summary: Clear one-line description  
- hook_event: Which Claude Code hook
- tool_matcher: What tools trigger this
- conditions: When to evaluate
- enforcement_action: block/feedback/inject/auto-fix
- action_details: Specific commands or messages
- requires_custom_script: boolean
- discovered_utilities: Array of found commands
```

### Research Needed
- Check existing extraction.rs for current data structures
- Determine if JSON or custom struct is used
- Identify gaps in current implementation

## 3. UI/UX Considerations

### Extraction Screen Components
- Progress indicators for each file being processed
- Live updates as rules are extracted
- Clear visual hierarchy showing:
  - Original rule (quoted)
  - Enforcement mechanism (highlighted)
  - Hook integration (technical details subdued)
  
### Editing Interface
- Inline editing with clear field labels
- Dropdown selects for hooks and actions
- Validation feedback on changes
- "Revert to original" option per rule

## 4. Meta-Programming Architecture

### Claude Code Integration
- Need async spawning of Claude Code-like analysis tasks
- Each task gets:
  - Rule file content
  - Codebase access for context
  - Extraction prompt
- Results aggregated for review

### Codebase Analysis Needs
- Scan for package.json scripts
- Check for Makefile targets
- Identify CI/CD configs
- Find test/lint/format commands

## 5. Implementation Priorities

### Phase 1: Foundation
1. Define the rule requirement scoping data structure
2. Create the extraction prompt using meta-framework
3. Wire up async task spawning in TUI

### Phase 2: Intelligence  
1. Implement codebase analysis utilities
2. Enhance extraction to find existing tools
3. Add custom script detection logic

### Phase 3: Polish
1. Build editing UI components
2. Add validation and preview
3. Create smooth transition to YAML generation

## 6. Testing Strategy

### Extraction Quality Tests
- Create test rule files with known patterns
- Verify extraction identifies all rules
- Check enforcement mechanisms make sense
- Ensure no loss of original intent

### Edge Cases to Handle
- Rules that can't be enforced deterministically
- Rules requiring human judgment
- Rules with complex conditions
- Cross-cutting concerns

## Key Success Factor

The extraction must feel **intelligent** - users should think "wow, it really understood what I meant and found the right way to enforce it" rather than "I need to fix all these misinterpretations."

This requires the prompt to be exceptionally well-crafted using the meta-framework principles.