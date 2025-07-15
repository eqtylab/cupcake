# Plan 015: Security Test Organization

Created: 2025-07-15T18:00:00Z
Depends: plan-008 (all parts)
Enables: production-ready security validation
Priority: HIGH

## Goal

Reorganize and expand security testing into dedicated, isolated test suites to validate security claims and ensure system effectiveness. Focus on critical security gaps identified in Plan 008 test audit.

## Success Criteria

1. ✅ Create dedicated `tests/security/` directory structure
2. ✅ Implement `array_security_test.rs` with expanded malicious input testing
3. ✅ Implement `string_security_test.rs` with injection prevention validation
4. ✅ Implement `shell_security_test.rs` with sandboxing and governance tests
5. ✅ Implement `template_security_test.rs` with advanced injection patterns
6. ✅ Implement `cross_mode_security_test.rs` for mode boundary testing
7. ✅ Create `tests/performance/` directory for performance security tests
8. ✅ All security tests pass and validate security claims

## Context

Plan 008 test audit revealed critical security testing gaps:
- Shell sandboxing tests missing (UID dropping, timeout enforcement)
- Advanced template injection patterns not tested
- Cross-mode security boundaries not validated
- Performance security requirements not verified

Current security tests are scattered across functional tests, making security validation difficult to assess and maintain.

## Technical Scope

### Security Test Structure

```
tests/
├── security/
│   ├── array_security_test.rs     # Array mode security validation
│   ├── string_security_test.rs    # String mode injection prevention
│   ├── shell_security_test.rs     # Shell sandboxing and governance
│   ├── template_security_test.rs  # Template injection patterns
│   └── cross_mode_security_test.rs # Mode boundary security
├── performance/
│   ├── array_performance_test.rs  # Array mode performance
│   ├── string_performance_test.rs # String mode performance
│   ├── shell_performance_test.rs  # Shell mode performance
│   └── benchmark_test.rs          # Sub-100ms requirement
└── [existing functional tests remain unchanged]
```

### Critical Security Tests to Implement

#### Array Security Tests
- **Malicious input isolation** - Expand existing test with more attack vectors
- **Command path template injection** - Prevent templates in command[0]
- **Argument sanitization** - Shell metacharacters as literal arguments
- **Process spawning security** - Direct tokio::process::Command usage

#### String Security Tests
- **Command substitution blocking** - `$(...)` and backticks prevention
- **Shell operator injection** - Malicious pipe/redirect attempts
- **Quote escape security** - Quote boundary testing
- **Parser boundary testing** - Complex parsing edge cases

#### Shell Security Tests (Critical Gap)
- **UID dropping validation** - Test sandbox_uid setting enforcement
- **Timeout enforcement** - Test timeout_ms prevents runaway processes
- **Governance controls** - Test allow_shell setting enforcement
- **Resource limit enforcement** - Test memory/CPU constraints

#### Template Security Tests (Critical Gap)
- **Advanced injection patterns** - Complex template attack vectors
- **Context boundary validation** - Templates only in safe contexts
- **Variable substitution security** - Malicious template content handling
- **Cross-context contamination** - Template isolation between modes

#### Cross-Mode Security Tests
- **Mode escalation attacks** - Array mode cannot become shell mode
- **Configuration bypass** - allow_shell=false cannot be circumvented
- **Context switching attacks** - Mixed mode policy security
- **Audit trail completeness** - All modes log security events

### Performance Security Tests
- **Sub-100ms requirement** - Validate performance claims
- **Resource exhaustion protection** - DoS prevention testing
- **Memory/CPU limit enforcement** - Resource constraint validation

## Implementation Strategy

1. **Create directory structure** - `tests/security/` and `tests/performance/`
2. **Move existing security tests** - Extract from functional tests
3. **Implement critical gap tests** - Focus on shell sandboxing first
4. **Add performance validation** - Sub-100ms requirement testing
5. **Document security test procedures** - Clear security validation process

## Risk Mitigation

- **Stay focused** - Only implement tests that validate specific security claims
- **Avoid deep holes** - Don't over-engineer test infrastructure
- **Maintain existing tests** - Don't break current functional testing
- **Critical first** - Prioritize shell sandboxing and template injection tests

## Success Metrics

- All security tests pass
- Critical security gaps filled
- Performance requirements validated
- Clear security test organization
- Maintainable test structure