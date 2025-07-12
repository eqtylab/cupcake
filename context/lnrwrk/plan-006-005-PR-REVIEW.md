# Pull Request: YAML Migration & Config File Flexibility

## Overview

This PR implements two major plans that transform Cupcake's policy format from TOML to YAML and adds flexible configuration loading capabilities.

### Original Problem

The initial TOML format, while successful as an MVP, presented scalability challenges:
- **Flat structure**: All policies in a single file became unwieldy at scale
- **Verbosity**: Repetitive hook_event and matcher fields for every policy
- **No modularity**: Teams couldn't manage their own policy domains without conflicts
- **Limited composability**: No way to organize policies by concern or team

The need was for a "Scalable and Composable Policy Format" that would support:
- Multiple teams contributing policies without merge conflicts
- Clear ownership boundaries
- Thousands of policies organized by domain
- Industry-standard directory conventions

## What Changed

### Plan 005: YAML Migration (Complete)

**Core Deliverables:**
1. **New Directory Structure**: `guardrails/` convention
   - `guardrails/cupcake.yaml` - Root configuration with settings and imports
   - `guardrails/policies/*.yaml` - Domain-specific policy fragments

2. **"Grouped Map" Format**: Eliminated repetition
   ```yaml
   PreToolUse:
     "Bash":
       - name: "Policy Name"
         conditions: [...]
         action: {...}
   ```

3. **Composition Engine**: Three-step process
   - Discover root config
   - Resolve imports with glob patterns
   - Deep merge fragments with name validation

4. **Complete Migration**: 
   - All tests converted to YAML
   - Documentation updated
   - TOML dependencies removed

### Plan 006: Config File Flexibility (Complete)

**Core Deliverables:**
1. **--config Parameter**: Industry-standard naming (was --policy-file)
2. **Flexible Loading**: Supports both:
   - Full RootConfig with imports
   - Bare PolicyFragment for testing
3. **Smart Detection**: Content-based format detection
4. **Testing Support**: Integration tests can use isolated policy files

### Bonus Enhancement: Inspect Command

**Added based on user suggestion:**
- `cupcake inspect` - Shows policies in compact table format
- Essential for debugging and understanding active policies
- Clean, scannable output perfect for development

## Code Quality Review

### Architecture & Design ✅

**Strengths:**
- Clean separation of concerns (loader, types, composition)
- Follows Rust idioms and error handling patterns
- Maintains backward compatibility where sensible
- Performance preserved (1.6ms load time)

**Key Design Decisions:**
1. **Content-based detection** over try-parse-fallback
2. **Deep merge** with name validation prevents conflicts
3. **Alphabetical loading** ensures determinism
4. **Two-format support** enables both production and testing use cases

### Test Coverage ✅

**Comprehensive Testing:**
- 120+ tests passing (up from 113)
- Unit tests for each component
- Integration tests for full workflows
- Error case coverage

**Quality Tests Added:**
- `test_load_from_config_file_policy_fragment`
- `test_load_from_config_file_root_config`
- `test_load_from_config_file_missing_file`
- `test_load_from_config_file_invalid_yaml`
- `test_deep_merge_fragment`
- `test_validate_and_flatten_duplicate_names`

### Error Handling ✅

**Production-Quality:**
- Missing configs error with clear messages
- Invalid YAML provides parse context
- Duplicate policy names caught at validation
- Graceful degradation in runtime enforcement

### Performance ✅

**Targets Met:**
- 1.6ms load time (well under 100ms target)
- Binary caching preserved for future optimization
- No performance regression from TOML

## Potential Issues & Mitigation

### 1. Breaking Change
**Issue**: TOML configs no longer supported
**Mitigation**: Acceptable in early development; clean break preferred

### 2. Import Path Resolution
**Issue**: Imports resolved relative to guardrails/ directory
**Mitigation**: Clear documentation and examples provided

### 3. Policy Name Uniqueness
**Issue**: Names must be unique across all fragments
**Mitigation**: Validation provides clear error messages

## Verification Checklist

- [x] Original problem solved (scalable, composable format)
- [x] All success criteria met
- [x] Tests are meaningful, not superficial
- [x] Error handling is robust
- [x] Performance targets maintained
- [x] Documentation updated
- [x] Code follows project conventions

## Summary

This PR successfully delivers a professional-grade policy management system that scales from small projects to enterprise deployments. The YAML migration provides the modularity and composability needed for team collaboration, while the config flexibility enables robust testing and development workflows.

The implementation is clean, well-tested, and maintains the performance characteristics that make Cupcake practical for real-world use.