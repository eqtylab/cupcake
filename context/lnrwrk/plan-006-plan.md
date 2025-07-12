# Plan for plan 006

Created: 2025-07-12T18:05:00Z
Updated: 2025-07-12T19:30:00Z

## Approach

Implement flexible configuration loading that supports both directory-based and file-based policy definitions, enabling better testing and development workflows while maintaining production robustness.

## Steps

### Phase 1: Core Implementation
1. Update CLI argument from `--policy-file` to `--config`
2. Modify `RunCommand::load_policies()` to check config parameter
3. Implement dual-format loading (RootConfig with imports vs bare PolicyFragment)
4. Fix path resolution for imports to be relative to config file location

### Phase 2: Test Infrastructure
1. Fix `test_run_command_with_policy_evaluation` using new `--config` parameter
2. Create integration tests for both:
   - Directory-based loading (guardrails/ structure)
   - Direct config file loading (single YAML file)
3. Ensure tests run in isolated environments (temp directories)
4. Remove any legacy TOML test artifacts

### Phase 3: Production Standards
1. Error handling: Missing config should fail with clear error (not empty policies)
2. Add proper logging for config resolution path
3. Document the behavior clearly in CLI help

## Technical Decisions

- **NO backward compatibility**: Clean break, update all references
- **Error handling**: Missing config is an error, not silent failure (industry standard)
- **Test philosophy**: Tests should fail if tool is used incorrectly
- **Performance**: Defer optimization - current ~1.6ms is well within 100ms target

## Future Work

- Performance optimization (binary cache) if/when needed
- Enhanced import resolution (URLs, package names) - see plan notes