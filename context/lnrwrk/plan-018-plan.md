# Plan for Plan 018

Created: 2025-01-22T12:00:00Z

## Approach

Replace all mock implementations in the TUI wizard with real functionality, working from user-facing features down to core extraction engine.

## Phases

### Phase 1: Core TUI Enhancements (Week 1-2)

**1.1 Enhanced Navigation & UX**
- Graceful Exit: Implement Ctrl+C and Esc handling throughout all screens
- Quality Navigation: Fix Tab/Arrow key consistency across all screens  
- Help System: Add F1/? for contextual help on each screen
- Confirmation Dialogs: Add "Are you sure?" for destructive actions

**1.2 Real File Discovery & Preview**
- Multi-Agent Support: Enhance discovery.rs to find all agent types (Claude, Cursor, Windsurf, etc.)
- Smart Preview: Show syntax-highlighted file contents with rule extraction hints
- Existing Rules Detection: Scan for existing `guardrails/` directory and prompt user for action:
  - Overwrite existing rules
  - Merge with existing rules  
  - Backup and replace
  - Cancel operation

**1.3 Custom Prompt Integration**
- Modal Implementation: Complete the custom prompt submission modal in discovery screen
- Prompt Validation: Ensure prompts are reasonable length and content
- Context Preservation: Pass custom prompts through to extraction engine

### Phase 2: Real Rule Extraction Engine (Week 3-4)

**2.1 LLM Integration Module**

Create `src/engine/extractor/`:
- `mod.rs` - Main extraction coordination
- `llm_client.rs` - Claude API integration with retry logic
- `rule_parser.rs` - Parse LLM responses into ExtractedRule objects
- `file_processor.rs` - Handle different file types (CLAUDE.md, .cursor/rules, etc.)

**2.2 Replace Mock Implementations**
- Real start_extraction(): Replace mock task creation with actual LLM calls
- Real populate_mock_rules(): Remove function entirely, use extracted rules
- Progress Tracking: Real-time updates during extraction with error handling
- Statistics: Actual rule counts by severity (Critical/Warning/Info)

**2.3 Extraction Features**
- Parallel Processing: Extract from multiple files concurrently
- Error Recovery: Handle API failures, rate limits, network issues
- Rule Categorization: Automatically categorize rules by type (git, security, code-style, etc.)
- Context-Aware Extraction: Use file paths and content to improve rule extraction

### Phase 3: Policy Generation Engine (Week 5-6)

**3.1 Policy Compiler Module**

Create `src/engine/compiler/`:
- `mod.rs` - Main compilation coordination
- `rule_converter.rs` - Convert ExtractedRule to Cupcake YAML policies
- `yaml_generator.rs` - Replace yaml_writer.rs with real implementation
- `policy_validator.rs` - Validate generated policies against schema

**3.2 Real Policy Generation**
- YAML Structure: Generate proper hook event organization (PreToolUse, PostToolUse, etc.)
- Condition Mapping: Map rule descriptions to proper condition types (pattern, check, state_exists)
- Action Generation: Create appropriate actions (provide_feedback, block_with_feedback, run_command)
- File Organization: Group policies by category into separate YAML files

**3.3 Integration & Validation**
- Schema Compliance: Ensure all generated YAML follows policy-schema.md
- Validation Pass: Run `cupcake validate` on generated policies
- Claude Settings: Real hook installation and configuration
- Testing: Verify generated policies work with `cupcake run`

### Phase 4: Meta-Prompt System (Week 7)

**4.1 Prompt Engineering**
- Rule Extraction Prompt: Design prompt for converting natural language to structured rules
- Context Injection: Include file paths, project structure, and custom instructions
- Few-Shot Examples: Include examples of good rule extractions
- Error Handling: Handle malformed LLM responses gracefully

**4.2 Quality Assurance**
- Response Validation: Verify LLM responses contain valid rule structures
- Confidence Scoring: Rate extraction confidence and flag uncertain rules
- Interactive Refinement: Allow users to refine extraction prompts
- Fallback Strategies: Handle cases where extraction fails completely

## Technical Decisions

**LLM Provider**: Use Claude API for rule extraction with fallback to local models
**Extraction Strategy**: One prompt per file with context injection
**Caching**: Cache extracted rules in `.cupcake/cache/` for re-use
**Parallelism**: Process up to 3 files concurrently to respect API limits
**Error Recovery**: Exponential backoff with manual retry options

## Dependencies

- Claude API access for rule extraction
- Update to latest ratatui/crossterm for better key handling
- Integration with existing policy evaluation engine
- Comprehensive testing with tests/manual-test environment

## Success Criteria

1. Complete UX: Smooth navigation, graceful exits, helpful error messages
2. Real Extraction: Actual rules extracted from files using LLM
3. Valid Policies: Generated YAML policies work with cupcake run
4. Existing Rules: Proper handling of existing guardrails configurations  
5. Performance: Sub-30 second extraction for typical projects
6. Reliability: Robust error handling and recovery mechanisms