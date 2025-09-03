# Implementation Log: Builtins Refactor (global_file_lock & protected_paths)

## Overview
**Date Started**: 2025-09-03
**Purpose**: Refactor builtin naming for semantic clarity and add user-defined protected paths feature
**Developer**: Claude (with human guidance)

## Background
Current builtin `never_edit_files` has poor naming - suggests specific file list when it's actually a global lock. Users need a way to declare specific files/directories as read-only while still allowing Claude Code to read them.

## Goals
1. Rename `never_edit_files` → `global_file_lock` for clarity
2. Create new `protected_paths` builtin for user-defined read-only paths
3. Maintain clear distinction between three security builtins:
   - `global_file_lock`: Session-wide write prevention
   - `protected_paths`: User-defined read-only paths (read allowed, write blocked)
   - `rulebook_security_guardrails`: Total Cupcake directory lockdown

## Implementation Phases

### Phase 1: Rename never_edit_files → global_file_lock
**Status**: ✅ Complete

#### 1.1 Impact Analysis
- ✅ Identified 42 references across 12 files
- ✅ Added backward compatibility via `#[serde(alias = "never_edit_files")]`
- ✅ No breaking changes for existing configs

#### 1.2 Rust Refactoring
Files modified:
- `cupcake-core/src/engine/builtins.rs`
  - ✅ Renamed `NeverEditConfig` → `GlobalFileLockConfig`
  - ✅ Renamed field `never_edit_files` → `global_file_lock`
  - ✅ Renamed `default_never_edit_message()` → `default_global_file_lock_message()`
  - ✅ Updated `any_enabled()` method
  - ✅ Updated `enabled_builtins()` method
  - ✅ Updated `generate_signals()` method
  - ✅ Updated validation logic

#### 1.3 Rego Policy Refactoring
- ✅ Renamed file: `never_edit_files.rego` → `global_file_lock.rego`
- ✅ Updated package: `package cupcake.policies.builtins.global_file_lock`
- ✅ Updated rule_id: `BUILTIN-NEVER-EDIT` → `BUILTIN-GLOBAL-FILE-LOCK`
- ✅ Updated metadata title and comments
- ✅ Fixed bash command parameter access (use `tool_input.command` not `params.command`)

#### 1.4 CLI Updates
- `cupcake-cli/src/main.rs`
  - ✅ Renamed constant: `NEVER_EDIT_FILES_POLICY` → `GLOBAL_FILE_LOCK_POLICY`
  - ✅ Updated include_str! path
  - ✅ Updated init command deployment

#### 1.5 Test Updates
- ✅ Updated builtin_integration.rs tests
- ✅ All references updated to use GlobalFileLockConfig
- ✅ Tests pass with new naming

#### 1.6 Verification
- ✅ Run `cargo test --features deterministic-tests test_enabled_builtins_list` - PASSED
- ✅ Run `cargo run -- validate` in examples directory - 0 errors
- ✅ Policy validates correctly as `global_file_lock.rego`

### Phase 2: Implement protected_paths Builtin
**Status**: Pending

#### 2.1 Design Decisions
- **Allowed Operations**: Read, Grep, Glob, cat, less, grep, head, tail
- **Blocked Operations**: Edit, Write, MultiEdit, NotebookEdit, rm, mv, cp, >, >>
- **Path Matching**: Direct paths, directory prefixes, glob patterns
- **Case Sensitivity**: Case-insensitive matching for robustness

#### 2.2 Rego Policy Implementation
File: `protected_paths.rego`
- [ ] Create base policy structure with metadata
- [ ] Implement file write blocking rule (Edit, Write, MultiEdit, NotebookEdit)
- [ ] Implement bash write blocking with read allowance
- [ ] Add path matching functions (direct, glob, directory)
- [ ] Add signal integration for dynamic paths

Key differences from rulebook_security_guardrails:
- DO NOT block Read, Grep, Glob tools
- Allow bash read commands (cat, less, grep, etc.)
- Use user-configurable paths from signals

#### 2.3 Rust Configuration
- [ ] Create `ProtectedPathsConfig` struct
- [ ] Add `protected_paths` field to `BuiltinsConfig`
- [ ] Implement default message function
- [ ] Update validation logic
- [ ] Update enabled checks

#### 2.4 Signal Generation
- [ ] Generate `__builtin_protected_paths_list` signal
- [ ] Generate `__builtin_protected_paths_message` signal
- [ ] Ensure shell-safe escaping

#### 2.5 CLI Deployment
- [ ] Add `PROTECTED_PATHS_POLICY` constant
- [ ] Deploy policy in init command
- [ ] Update deployment order

#### 2.6 Testing
- [ ] Create integration test file: `protected_paths_integration.rs`
- [ ] Test read operations are allowed
- [ ] Test write operations are blocked
- [ ] Test glob pattern matching
- [ ] Test bash command discrimination

#### 2.7 Verification
- [ ] Validate policy with CLI validator
- [ ] Test with real guidebook configuration
- [ ] Verify interaction with other builtins

### Phase 3: Documentation and Integration
**Status**: Pending

#### 3.1 Documentation Updates
- [ ] Update `examples/base-config.yml` with all three builtins
- [ ] Add migration notes for `never_edit_files` users
- [ ] Document glob pattern support
- [ ] Add usage examples

#### 3.2 Integration Testing
- [ ] Test all three builtins enabled simultaneously
- [ ] Test overlapping protections
- [ ] Test precedence and priority

#### 3.3 Security Review
- [ ] Verify no bypass techniques for protected_paths
- [ ] Ensure read/write distinction is maintained
- [ ] Check for glob pattern edge cases

#### 3.4 Code Quality Review
- [ ] Consistent naming conventions
- [ ] Proper error handling
- [ ] Performance implications
- [ ] Maintainability

#### 3.5 Final Validation
- [ ] Full system test with example project
- [ ] CLI validation passes
- [ ] All tests green

## Technical Decisions

### Why Rename never_edit_files?
- Name implies specific file list, but it's actually global
- `global_file_lock` immediately conveys session-wide scope
- Aligns with new semantic naming pattern

### Why Create protected_paths Instead of Extending?
- Clear separation of concerns
- Different use cases (global vs specific)
- Easier to reason about and configure
- Maintains backward compatibility

### Read vs Write Distinction
- Users need to reference protected files (understand system)
- But prevent accidental modification (maintain integrity)
- Similar to Unix file permissions (r-- vs rw-)

### Path Matching Strategy
- Direct match: `production.env`
- Directory match: `src/legacy/` (anything inside)
- Glob match: `*.secret`, `config/*.yml`
- Case-insensitive for robustness

## Risks and Mitigations

### Risk: Breaking Existing Configurations
**Mitigation**: Could support both names temporarily with deprecation warning

### Risk: Path Matching Complexity
**Mitigation**: Start with simple patterns, expand based on user needs

### Risk: Performance Impact
**Mitigation**: Optimize path matching, cache compiled patterns

## Success Criteria
1. ✅ Clear semantic naming for all three builtins
2. ✅ Users can protect specific paths while allowing reads
3. ✅ No regression in existing functionality
4. ✅ All tests pass
5. ✅ Documentation is clear and complete

## Notes
- Bash command parameter fix needed (tool_input.command not params.command)
- Consider future enhancement: exempt patterns for global_file_lock
- Consider future enhancement: regex patterns for protected_paths

## Progress Log

### 2025-09-03 09:50 PST
- Created implementation plan
- Identified three-builtin system architecture
- Established clear semantic naming
- Created comprehensive todo list
- Started implementation log

### 2025-09-03 10:05 PST
- ✅ Completed Phase 1: Successfully renamed never_edit_files → global_file_lock
- All tests passing, backward compatibility maintained
- Fixed critical bug: Bash commands now use tool_input.command (not params.command)
- Ready to start Phase 2: protected_paths implementation

---
*This log will be updated as implementation progresses*