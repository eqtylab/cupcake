# Plan 017 Handoff

Created: 2025-01-21T19:30:00Z
From: Plan 017 (Interactive TUI Init Wizard)
To: Next Plan (Real Data Integration)

## Overview

Plan 017 has delivered a fully functional TUI wizard with 6 polished screens and complete user flows. However, it currently uses mock data throughout. This handoff document identifies exactly what needs to be implemented to make the wizard work with real data.

## Current State

### What's Complete
- ✅ All 6 UI screens (Discovery, Extraction, Review, Compilation, Success)
- ✅ State machine and event handling
- ✅ Real file discovery (finds actual CLAUDE.md, .cursor/rules, etc.)
- ✅ Real file preview (shows actual file contents)
- ✅ File I/O for YAML generation (writes real files)
- ✅ Claude Code settings update (writes .claude/settings.local.json)
- ✅ All user interactions (navigation, search, edit, etc.)

### What's Mocked
- ❌ Rule extraction (returns hardcoded rules)
- ❌ LLM integration (no actual AI calls)
- ❌ Policy compilation (generates stub YAML)
- ❌ Rule statistics (hardcoded counts)

## Code Locations Requiring Changes

### 1. Rule Extraction Engine
**File**: `src/cli/tui/init/app.rs`
**Function**: `start_extraction` (line 607)
**Current**: Creates mock ExtractionTask objects with fake progress
**Needed**: Real extraction engine that:
- Takes selected files and custom instructions
- Calls LLM API to extract rules
- Returns actual ExtractedRule objects
- Updates progress in real-time

### 2. Mock Rules Population
**File**: `src/cli/tui/init/app.rs`
**Function**: `populate_mock_rules` (line 794)
**Current**: Creates 9 hardcoded ExtractedRule objects
**Needed**: Replace with actual rules from extraction engine

### 3. YAML Policy Compiler
**File**: `src/cli/tui/init/yaml_writer.rs`
**Function**: `generate_stub_files` (line 8)
**Current**: Writes hardcoded stub YAML files
**Needed**: Real policy compiler that:
- Takes Vec<ExtractedRule> from review state
- Groups rules by hook event type
- Generates valid Cupcake YAML policies
- Handles condition patterns and actions

### 4. Compilation Process
**File**: `src/cli/tui/init/app.rs`
**Function**: `start_compilation` (line 638)
**Current**: Simulates compilation with sleep timers
**Needed**: Real compilation that:
- Calls yaml_writer with actual rules
- Validates generated policies
- Updates progress based on actual work

### 5. Statistics Calculation
**File**: `src/cli/tui/init/app.rs`
**Line**: 217-221
**Current**: Hardcoded counts (52 total, 18 critical, etc.)
**Needed**: Calculate from actual selected rules in ReviewState

## Key Data Structures

### ExtractedRule (src/cli/tui/init/state.rs)
```rust
pub struct ExtractedRule {
    pub id: usize,
    pub source_file: PathBuf,      // Which file it came from
    pub description: String,        // Rule description
    pub severity: Severity,         // Critical/Warning/Info
    pub category: String,           // testing/code-style/etc
    pub when: String,               // Hook event timing
    pub block_on_violation: bool,   // Hard block or soft feedback
}
```

### Cupcake Policy Format (expected output)
```yaml
PreToolUse:
  "Bash":
    - name: "Extracted Rule Name"
      description: "From ExtractedRule.description"
      conditions:
        - type: "pattern"
          field: "tool_input.command"
          regex: "generated_from_rule"
      action:
        type: "provide_feedback"  # or "block_with_feedback"
        message: "Generated message"
```

## Integration Points

### 1. LLM Service
Need to create an LLM service module that:
- Configures API credentials (OpenAI/Anthropic)
- Handles rate limiting and retries
- Provides structured extraction prompts
- Parses LLM responses into ExtractedRule objects

### 2. Extraction Prompts
Design extraction prompts that:
- Include custom instructions if provided
- Extract actionable rules from markdown/YAML
- Identify severity levels
- Determine appropriate hook events
- Generate regex patterns for conditions

### 3. Policy Templates
Create templates for different rule types:
- Git/commit rules → PreCommit hook
- File operation rules → PreToolUse/PostToolUse
- Code style rules → appropriate tool patterns
- Security rules → blocking actions

## Implementation Plan

### Week 1: LLM Integration
- Create LLM service module
- Add API configuration
- Design extraction prompts
- Test with sample files

### Week 2: Extraction Engine
- Implement real extraction in start_extraction
- Parse LLM responses to ExtractedRule
- Add progress tracking
- Handle extraction errors

### Week 3: Policy Compilation
- Replace yaml_writer stub generation
- Map ExtractedRule → YAML policies
- Group by hook events
- Generate valid regex patterns

### Week 4: Integration & Testing
- Connect all components
- Calculate real statistics
- Add error handling
- End-to-end testing

## Key Decisions Needed

1. **LLM Provider**: OpenAI GPT-4 or Anthropic Claude?
2. **Extraction Strategy**: One prompt per file or batch?
3. **Pattern Generation**: LLM-generated regex or template-based?
4. **Error Recovery**: What if extraction fails for some files?
5. **Caching**: Cache extracted rules to avoid re-extraction?

## Testing Considerations

1. Create test rule files with known expected extractions
2. Mock LLM responses for unit tests
3. Test regex pattern validity
4. Verify generated YAML against schema
5. End-to-end test with real files

## Success Criteria

The next plan is complete when:
1. Running `cupcake init` extracts real rules from discovered files
2. Extracted rules appear in the review screen with accurate data
3. Generated YAML files contain valid policies from selected rules
4. Rule counts in success screen match actual selections
5. Generated policies work with `cupcake run` command

## Notes

- The UI is complete and should not need changes
- Focus only on replacing mock data with real implementations
- Preserve the existing user experience
- Keep extraction times reasonable (<5s per file)
- Consider adding a --mock flag for testing without LLM