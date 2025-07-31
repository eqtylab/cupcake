# TUI Intelligent Extraction Design

## Overview

The intelligent extraction process is the heart of Cupcake's value proposition - converting natural language rules into enforceable policies through a two-stage process with human review and editing capabilities.

## TUI Flow

### Screen 1: Rule File Discovery & Selection
- **Deterministic file scan** finds all rule files (CLAUDE.md, .cursorrules, etc.)
- All files selected by default with checkboxes
- Preview panel for reviewing rule content
- Users can deselect outdated or irrelevant rule files

### Screen 2: Intelligent Rule Extraction
- **Asynchronous extraction** runs on each selected file
- Converts natural language rules into structured "rule requirement scoping"
- NOT creating YAML policies yet - creating intermediate representation
- Enables human review and manual editing before final conversion

## The Extraction Process

### What Gets Extracted

For each natural language rule, the extraction creates a structured representation containing:

1. **The Rule** - As the user defined it in natural language
2. **Enforcement Mechanism** - How Cupcake will guarantee this rule
3. **Hook Integration** - Which Claude Code hooks will be used (PreToolUse, PostToolUse, etc.)
4. **Trigger Conditions** - When the rule evaluation fires
5. **Action on Violation** - What happens if the rule is broken (block, feedback, auto-fix)
6. **Action on Success** - Any chained activities when rule passes

### Examples

**Example 1: Auto-linting**
- Rule: "After any new file you must lint the file"
- Hook: PostToolUse on Write tool
- Action: Automatically lint files after creation
- On Failure: Provide feedback if lint fails and can't auto-fix

**Example 2: Test-before-commit**
- Rule: "Tests must pass before committing"
- Hook: PreToolUse on bash (detecting git commit commands)
- Action: Block tool use on test failure, allow if tests pass

## Meta-Programming Intelligence

The extraction isn't just pattern matching - it's a meta-programming task where Claude Code:

1. **Analyzes the codebase** to understand what rules mean in context
2. **Discovers existing utilities** (lint commands, test scripts, formatters)
3. **Identifies custom evaluation needs** (some rules may need custom scripts)
4. **Determines appropriate actions** (block vs feedback vs auto-fix vs notify)

This is why we leverage Claude Code itself - it can explore the repository to find:
- Package.json scripts
- Makefile targets
- CI/CD configurations
- Existing tooling that can be leveraged

## Output Structure

The extraction produces an intermediate representation (possibly JSON) that:
- Is human-readable for review
- Supports field-by-field manual editing
- Contains all necessary information for YAML policy generation
- Maintains traceability to original natural language rule

## Human Review & Editing

### Two-fold Purpose:
1. **Review** - Users understand exactly how their rules will be enforced
2. **Edit** - Users can correct misinterpretations or adjust enforcement

### Editing Approach (Initial):
- Manual field editing for each extracted rule
- Clear UI for modifying:
  - Rule description
  - Hook selection
  - Trigger conditions
  - Actions
- Future: Potentially add intelligent editing via prompts

## Technical Considerations

### Async Architecture
- Each rule file processed independently
- Progress tracking for user feedback
- Concurrent extraction for performance

### Data Structure Needs
- Research existing JSON structure in codebase
- May need enhancement to capture all required fields
- Must support both simple and complex rule patterns

### Integration Points
- Must understand Claude Code hook capabilities
- Need to map rules to appropriate hook events
- Consider custom script generation for complex evaluations

## Success Criteria

The extraction is successful when:
1. Users clearly understand how each rule will be enforced
2. The enforcement mechanism matches user intent
3. Complex rules are broken down appropriately
4. The output can be reliably converted to YAML policies

## Next Steps

After successful extraction and user review:
1. Feed approved rules to YAML generation process
2. Run validation tests on generated policies
3. Allow final review before activation

## Key Insight

This isn't just "send file to LLM and convert" - it's an intelligent process that:
- Understands codebase context
- Leverages existing tooling
- Creates deterministic guarantees from probabilistic understanding
- Maintains human oversight at critical decision points