# Plan 013: Clean Up RunCommand Pattern Matching Dead Code

Created: 2025-01-14T10:25:00Z
Depends: plan-009
Enables: none
Priority: MODERATE

## Goal

Remove the redundant pattern matching in `execute_pass_2` where both branches of RunCommand handling do exactly the same thing (continue).

## Success Criteria

- Dead code removed from execute_pass_2
- Clear documentation of RunCommand evaluation behavior
- No change in functionality
- Improved code readability
- Test coverage remains complete

## Context

Plan 007's implementation has redundant pattern matching in `src/engine/evaluation.rs:216-224` where both `OnFailureBehavior::Block` and `OnFailureBehavior::Continue` branches just continue. This creates confusion about the evaluation model and makes the code harder to understand.

## Code Quality Impact

- **Issue**: Both branches do `continue`, making the conditional meaningless
- **Location**: `src/engine/evaluation.rs:216-224`
- **Solution**: Simplify to single continue with clear comment
- **Benefit**: Cleaner code, clearer intent