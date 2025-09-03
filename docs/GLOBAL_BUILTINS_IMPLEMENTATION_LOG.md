# Global Builtins Implementation Log

## Implementation Overview
**Date Started**: 2025-09-03  
**Objective**: Implement machine-wide security builtins that protect the host system from malicious operations  
**Developer**: Claude (with human guidance)  

## Context and Requirements

### Business Requirements
- Protect sensitive system paths from ANY modification
- Block reading of credentials and secrets in repositories  
- Prevent direct execution of cupcake binary via Bash (while allowing hooks)
- Ensure these protections are non-overridable at project level

### Technical Context
The builtin system in Cupcake provides pre-configured security policies that users can enable. Currently, builtins are project-scoped. We're extending this to support global (machine-wide) builtins that:
1. Live in the global configuration directory
2. Use the `cupcake.global.policies.builtins.*` namespace
3. Take absolute precedence over project policies
4. Are enabled by default on global init

## Critical Design Decisions

### Decision 1: Builtin Namespace
**Choice**: Use `cupcake.global.policies.builtins.*` for global builtins  
**Rationale**: 
- Maintains clear separation from project builtins
- Leverages existing namespace transformation in compiler.rs
- Ensures no collision with project policies
**Impact**: Global builtins will be automatically routed and evaluated in Phase 1

### Decision 2: Policy Decisions Types
**Choices**:
- `system_protection`: Uses HALT (immediate termination)
- `sensitive_data_protection`: Uses DENY (blockable but logged)
- `cupcake_exec_protection`: Uses HALT (prevent bypass)
**Rationale**:
- System modification is critical - must halt immediately
- Credential reading is serious but might have legitimate cases
- Cupcake execution could bypass all policies - must halt
**Impact**: Different severity levels provide appropriate response

### Decision 3: Deployment Strategy
**Choice**: Bundle policies as string literals in Rust code
**Rationale**:
- Ensures policies are always available
- Prevents tampering with external files
- Simplifies distribution (single binary)
**Impact**: Policies become part of the compiled binary

## Implementation Phases

### Phase 1: Configuration Structures
**Status**: Not Started  
**Objective**: Add configuration support for new builtins

#### Tasks:
1. Add `SystemProtectionConfig` struct to builtins.rs
2. Add `SensitiveDataConfig` struct to builtins.rs
3. Add `CupcakeExecProtectionConfig` struct to builtins.rs
4. Update `BuiltinsConfig` to include new fields
5. Update `enabled_builtins()` method
6. Update `generate_signals()` if needed

#### Verification:
- [ ] Code compiles without errors
- [ ] Can parse YAML with new builtin configs
- [ ] enabled_builtins() returns correct list

---

### Phase 2: Builtin Policies
**Status**: Not Started  
**Objective**: Create and validate the three builtin policies

#### Tasks:
1. Finalize `system_protection.rego`
2. Finalize `sensitive_data_protection.rego`  
3. Create `cupcake_exec_protection.rego`
4. Validate all policies compile with OPA
5. Test policies with sample inputs

#### Verification:
- [ ] All policies pass OPA compilation
- [ ] Policies use correct global namespace
- [ ] Decision structures are valid

---

### Phase 3: CLI Global Init
**Status**: Not Started  
**Objective**: Implement `cupcake init --global` command

#### Tasks:
1. Update init_command to handle --global flag
2. Create global directory structure
3. Deploy builtin policies to global/policies/builtins/
4. Create default global guidebook.yml
5. Set appropriate file permissions

#### Verification:
- [ ] Command creates ~/.config/cupcake/ (or platform equivalent)
- [ ] All files deployed correctly
- [ ] Permissions prevent modification

---

### Phase 4: Policy Bundling
**Status**: Not Started  
**Objective**: Bundle policies with the binary

#### Tasks:
1. Create `builtin_policies.rs` module
2. Embed policy content as const strings
3. Add deployment function to write policies
4. Update init command to use bundled policies

#### Verification:
- [ ] Policies accessible from binary
- [ ] Deployment writes correct content
- [ ] No external file dependencies

---

### Phase 5: Integration Testing
**Status**: Not Started  
**Objective**: Comprehensive test coverage

#### Tasks:
1. Test system path blocking
2. Test credential file blocking
3. Test cupcake binary blocking
4. Test global precedence over project
5. Test with builtins disabled

#### Verification:
- [ ] All tests pass
- [ ] Coverage > 80%
- [ ] No race conditions

---

### Phase 6: Manual Testing
**Status**: Not Started  
**Objective**: Real-world validation with Claude Code

#### Test Cases:
1. Try to read ~/.ssh/id_rsa
2. Try to edit /etc/hosts
3. Try to search for *.env files
4. Try to run `cupcake trust init` via Bash
5. Verify normal operations still work

#### Verification:
- [ ] All malicious operations blocked
- [ ] Legitimate operations allowed
- [ ] Clear error messages

---

## Risk Assessment

### Risk 1: Breaking Legitimate Operations
**Mitigation**: Use DENY instead of HALT where appropriate, allow whitelisting

### Risk 2: Performance Impact
**Mitigation**: Policies only evaluated for matching events, O(1) routing

### Risk 3: Platform Compatibility
**Mitigation**: Test on macOS, Linux, Windows; use platform-specific paths

### Risk 4: User Confusion
**Mitigation**: Clear error messages, documentation, allow disabling if needed

## Code Quality Standards

### For this implementation:
1. **Type Safety**: All configurations strongly typed
2. **Error Handling**: Graceful degradation if global config missing
3. **Testing**: Both unit and integration tests required
4. **Documentation**: Inline comments for complex logic
5. **Security**: Policies cannot be bypassed or modified

## Implementation Notes

### Entry Points:
- `builtins.rs`: Configuration structures
- `init_command` in main.rs: Global initialization
- `global_config.rs`: Path discovery (already implemented)

### Dependencies:
- No new external dependencies required
- Uses existing OPA compilation pipeline
- Leverages existing builtin filtering system

## Progress Tracking

### Phase Status:
- [x] Phase 1: Configuration Structures - COMPLETE
- [x] Phase 2: Builtin Policies - COMPLETE
- [x] Phase 3: CLI Global Init - COMPLETE
- [x] Phase 4: Policy Bundling - COMPLETE  
- [ ] Phase 5: Integration Testing - IN PROGRESS
- [ ] Phase 6: Manual Testing

### Current Status (2025-09-03):

**Completed:**
✅ Configuration structures added to builtins.rs for three new global builtins
✅ Three global builtin policies created and tested for OPA compilation
✅ CLI init --global command updated to deploy builtins
✅ Policies successfully bundled in binary via include_str!
✅ Global configuration deployed successfully to ~/Library/Application Support/cupcake
✅ Integration tests working (4/5 passing)
✅ Resolved OPA WASM compilation issue

**Key Findings:**
1. Global and project policies compile to separate WASM modules with different entrypoints
2. Global uses `cupcake.global.system.evaluate`, project uses `cupcake.system.evaluate`
3. The engine evaluates them in two phases - global first with early termination
4. Policies need routing metadata to be discovered and evaluated
5. Builtin filtering works correctly but test policies shouldn't go in builtins/ directory

### Architecture Insights:
- Global and project policies are completely isolated in separate namespaces
- No conflict between their evaluate entrypoints
- Compiler automatically transforms package names for global namespace
- Two separate WASM runtimes handle global vs project evaluation

### Known Limitations:
- **Builtin signal injection for global policies**: The `test_global_builtin_signals` test fails because builtin signal generation (like `__builtin_system_protection_paths`) is not yet wired up for global policy evaluation. This means global builtins can't use dynamic configuration from guidebook.yml yet. The static policies work fine, but dynamic configuration through signals needs additional implementation.

### Questions for Review:
1. Should we allow users to disable global builtins?
2. Should we provide a --force flag to override blocks?
3. How should we handle updates to bundled policies?

---

## Post-Implementation Notes
(To be filled after completion)

### What Worked Well:

### What Could Be Improved:

### Lessons Learned:

### Future Enhancements: