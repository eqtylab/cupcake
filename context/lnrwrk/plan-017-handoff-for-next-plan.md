# Plan 017 Handoff Document

Created: 2025-01-21T20:00:00Z
Author: Previous Claude Instance
For: Next Plan Implementation

## Executive Summary

Plan 017 successfully delivered a complete interactive TUI wizard for Cupcake initialization with all 6 screens fully implemented. However, the TUI currently uses **mock data** and **stub implementations** for the actual rule extraction and policy generation. This handoff document details exactly what needs to be replaced with real implementations.

## Current State Analysis

### What Works (Complete)
1. **File Discovery** - Real implementation that finds actual rule files
2. **File Preview** - Shows actual file contents
3. **UI/UX** - All screens, navigation, and interactions are polished
4. **Claude Settings** - Updates .claude/settings.local.json correctly
5. **YAML Structure** - Creates correct directory structure (guardrails/)
6. **Tests** - 25+ passing tests covering all functionality

### What's Mock/Stub (Needs Implementation)
1. **Rule Extraction** - Currently generates fake ExtractedRule objects
2. **LLM Integration** - No actual AI processing of rule files
3. **Policy Generation** - Creates placeholder YAML instead of real policies
4. **State Management** - No real tracking of extraction progress
5. **Error Handling** - Basic stubs for extraction failures

## Code Areas Requiring Changes

### 1. Rule Extraction System (Priority 1)

**Location**: `src/cli/tui/init/app.rs`
```rust
// Line 607: start_extraction function
fn start_extraction(&self, selected_files: &HashSet<PathBuf>, state: &mut ExtractionState) -> Result<()> {
    // TODO: Actually spawn extraction tasks
    // Currently just creates mock ExtractionTask objects
}
```

**What's Needed**:
- Create a real extraction engine module (`src/engine/extractor.rs`)
- Implement LLM integration for processing rule files
- Use the meta-prompt design from `context/design_phase/meta-prompt.md`
- Support for different file types (CLAUDE.md, .cursor/rules, YAML, etc.)
- Progress tracking and parallel processing
- Error recovery and retry logic

### 2. Mock Rules Population (Priority 1)

**Location**: `src/cli/tui/init/app.rs`
```rust
// Line 794: populate_mock_rules function
fn populate_mock_rules(state: &mut ReviewState) {
    // Currently creates hardcoded ExtractedRule objects
    // Needs to use actual extracted rules from step 1
}
```

**What's Needed**:
- Remove this entire function
- Replace with actual rules from extraction engine
- Maintain the same ReviewState structure
- Preserve the grouping by source file

### 3. YAML Policy Generation (Priority 1)

**Location**: `src/cli/tui/init/yaml_writer.rs`
```rust
// Lines 8-89: generate_stub_files function
pub fn generate_stub_files(output_dir: &Path, rule_count: usize) -> Result<()> {
    // Currently writes hardcoded stub YAML files
    // Needs to generate real policies from extracted rules
}
```

**What's Needed**:
- Create a policy compiler module (`src/engine/compiler.rs`)
- Convert ExtractedRule objects to Cupcake policy YAML
- Group policies by hook event type
- Generate proper condition and action blocks
- Follow the schema in `context/design_phase/policy-schema.md`

### 4. Compilation Process Integration (Priority 2)

**Location**: `src/cli/tui/init/app.rs`
```rust
// Line 639: start_compilation function
fn start_compilation(&self, state: &mut CompilationState) -> Result<()> {
    // Currently simulates compilation with timeouts
    // Needs to actually compile policies and update settings
}
```

**What's Needed**:
- Real policy compilation using the compiler from step 3
- Actual file I/O operations with proper error handling
- Real-time progress updates
- Integration with `cupcake validate` command
- Proper hook installation verification

### 5. Rule Count and Statistics (Priority 2)

**Location**: `src/cli/tui/init/app.rs`
```rust
// Line 217: Hardcoded statistics
total_rules: 52, // TODO: Get from actual data
critical_count: 18,
warning_count: 23,
info_count: 11,
```

**What's Needed**:
- Track actual statistics during extraction
- Pass real counts through state transitions
- Update success screen with accurate data

## Integration Points

### 1. LLM Service Integration
- Need to decide on LLM provider (Claude API, local model, etc.)
- Create abstraction layer for swappable LLM backends
- Handle API keys and authentication
- Implement retry logic and rate limiting

### 2. Existing Engine Integration
- Reuse types from `src/engine/types.rs`
- Leverage existing policy validation logic
- Use the evaluation engine for testing generated policies
- Maintain compatibility with `cupcake run` command

### 3. State Persistence
- Save extraction progress to `.cupcake/state/`
- Allow resuming interrupted extractions
- Cache extracted rules for re-use
- Track which files have been processed

## Suggested Implementation Approach

### Phase 1: Extraction Engine (Week 1)
1. Create `src/engine/extractor/` module structure
2. Implement file readers for each agent type
3. Build LLM integration with meta-prompt
4. Add progress tracking and event emission
5. Create comprehensive tests

### Phase 2: Policy Compilation (Week 2)
1. Create `src/engine/compiler/` module
2. Implement ExtractedRule -> Policy conversion
3. Build YAML generation with proper schema
4. Add validation pass
5. Test with real extracted rules

### Phase 3: Integration (Week 3)
1. Remove all mock data generation
2. Wire up real extraction to TUI
3. Connect compiler to compilation screen
4. Add error handling and recovery
5. End-to-end testing

### Phase 4: Polish (Week 4)
1. Performance optimization
2. Better error messages
3. Resume capability
4. Documentation
5. Production hardening

## Key Design Decisions Needed

1. **LLM Provider**: Which LLM API to use? Local vs cloud?
2. **Extraction Strategy**: One prompt per file or batch processing?
3. **Caching**: How to cache extracted rules for speed?
4. **Parallelism**: How many files to process concurrently?
5. **Error Recovery**: How to handle partial failures?

## Testing Considerations

1. **Mock LLM for Tests**: Need abstraction to test without real API calls
2. **Golden Files**: Create test fixtures with known extraction results
3. **Performance Tests**: Ensure sub-100ms requirement is met
4. **Integration Tests**: Full flow from discovery to YAML generation
5. **Error Cases**: Test all failure modes thoroughly

## Migration Path

The current implementation is designed to make migration straightforward:
1. The UI doesn't need to change at all
2. Only the data generation functions need replacement
3. All state structures can remain the same
4. Tests can be updated incrementally

## Success Criteria for Next Plan

1. Real rules extracted from actual files
2. Valid Cupcake policies generated
3. All mock data removed
4. Full integration with existing engine
5. Production-ready error handling
6. Performance within requirements
7. Comprehensive test coverage

## Appendix: Mock Data Reference

For reference, here's what the mock data currently generates:
- 52 total rules (18 critical, 23 warning, 11 info)
- Rules from 4 different source files
- Mix of pre-commit, tool-use, and file-change hooks
- Various condition types and actions

The real implementation should produce similar variety but from actual file content.