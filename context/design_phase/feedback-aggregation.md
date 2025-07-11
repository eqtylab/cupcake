# Feedback Aggregation in Cupcake

## Overview

This document clarifies the critical design decision about how Cupcake aggregates feedback from multiple policies when both "soft" feedback and "hard" blocks are triggered.

## The Rule: Always Provide Complete Feedback

**When an operation triggers both soft feedback and a hard block, Cupcake provides ALL feedback in an organized response.**

This ensures Claude receives:
1. The critical blocking reason (so it knows what MUST be fixed)
2. All other policy feedback (so it can fix everything in one pass)

## Feedback Format

### When Only Soft Feedback Exists

```
Policy feedback found:
• Use <Button> instead of <button>
• Use <Link> instead of <a>
• Import components from @/ui

Please address these issues before proceeding.
```

### When Hard Block Exists (With Additional Feedback)

```
Operation blocked: Tests must pass before committing

Additional policy feedback:
• Use <Button> instead of <button>
• Use <Link> instead of <a>
• Import components from @/ui

Fix the blocking issue and address the additional feedback.
```

### When Only Hard Block Exists

```
Operation blocked: Cannot edit production config files without approval

Contact your team lead for access.
```

## Implementation Logic

```rust
struct PolicyResult {
    hard_action: Option<HardAction>,
    soft_feedback: Vec<String>,
}

fn aggregate_results(result: PolicyResult) -> Response {
    match (result.hard_action, !result.soft_feedback.is_empty()) {
        // Hard block + soft feedback
        (Some(hard), true) => {
            let mut message = format!("Operation blocked: {}\n", hard.message);
            message.push_str("\nAdditional policy feedback:\n");
            for feedback in result.soft_feedback {
                message.push_str(&format!("• {}\n", feedback));
            }
            Response::Block(message)
        },
        
        // Only soft feedback
        (None, true) => {
            let mut message = String::from("Policy feedback found:\n");
            for feedback in result.soft_feedback {
                message.push_str(&format!("• {}\n", feedback));
            }
            message.push_str("\nPlease address these issues before proceeding.");
            Response::Block(message)
        },
        
        // Only hard block
        (Some(hard), false) => {
            Response::Block(format!("Operation blocked: {}", hard.message))
        },
        
        // Nothing triggered
        (None, false) => Response::Allow,
    }
}
```

## Benefits of Complete Feedback

### 1. Efficiency
Claude can fix all issues in one attempt rather than discovering them one at a time:
- Without aggregation: Fix security issue → resubmit → fix style issue → resubmit → fix import issue
- With aggregation: See all issues → fix everything → resubmit once

### 2. Context
Claude understands the full scope of what needs to be addressed:
- Knows which issues are blocking (must fix)
- Knows which issues are suggestions (should fix)
- Can prioritize accordingly

### 3. User Experience
Developers get a complete picture of policy compliance:
- No surprise additional violations after fixing the first one
- Clear distinction between critical and stylistic issues
- Faster iteration cycles

## Examples

### Example 1: Security + Style

```toml
# Triggers on console.log (hard block)
[[policy]]
name = "No console.log in production"
action = { type = "block_with_feedback", feedback_message = "Remove console statements" }

# Triggers on <button> (soft feedback)
[[policy]]
name = "Use design system"
action = { type = "provide_feedback", message = "Use <Button> component" }
```

**Result when both trigger:**
```
Operation blocked: Remove console statements

Additional policy feedback:
• Use <Button> component
```

### Example 2: Multiple Soft Violations

```toml
[[policy]]
name = "Component naming"
action = { type = "provide_feedback", message = "Component should be PascalCase" }

[[policy]]
name = "Import ordering"
action = { type = "provide_feedback", message = "Group imports by type" }

[[policy]]
name = "File size"
action = { type = "provide_feedback", message = "File exceeds 500 lines" }
```

**Result when all trigger (no hard block):**
```
Policy feedback found:
• Component should be PascalCase
• Group imports by type
• File exceeds 500 lines

Please address these issues before proceeding.
```

### Example 3: Critical Security Block

```toml
[[policy]]
name = "Prevent secret exposure"
conditions = [{ type = "file_content_regex", value = "api_key\\s*=\\s*[\"']\\w+" }]
action = { 
  type = "block_with_feedback", 
  feedback_message = "CRITICAL: Exposed API key detected! Remove immediately." 
}
```

**Result (even with other feedback):**
```
Operation blocked: CRITICAL: Exposed API key detected! Remove immediately.

Additional policy feedback:
• Use <Button> instead of <button>
• Add TypeScript types to function parameters
```

## Design Rationale

### Why Not Discard Soft Feedback?

1. **Wasted Evaluation**: We already spent cycles finding all violations
2. **Poor UX**: Users would fix one issue only to discover more
3. **Inefficient**: Multiple round trips to fix all issues

### Why Not Only Show Soft Feedback?

1. **Clarity**: Critical issues must be prominently displayed
2. **Priority**: Users need to know what MUST be fixed vs SHOULD be fixed
3. **Security**: Blocking reasons should never be buried in other feedback

### The Balanced Approach

Our approach clearly separates:
- **Primary blocking reason** (what must be fixed)
- **Additional feedback** (what should also be addressed)

This provides maximum information while maintaining clear priorities.

## Conclusion

The two-pass evaluation with complete feedback aggregation ensures:
1. **Comprehensive feedback** - All policy violations are reported
2. **Clear priorities** - Blocking issues are highlighted
3. **Efficient workflows** - Fix everything in one pass
4. **Better UX** - No surprise violations after fixing the first issue

This design decision is fundamental to Cupcake's goal of providing deterministic, helpful policy enforcement that enhances rather than hinders developer productivity.