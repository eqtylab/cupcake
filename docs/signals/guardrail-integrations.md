# Guardrail Integrations

**Status:** Design Specification  
**Date:** August 2025

## Overview

Cupcake's signal system provides a powerful integration point for existing industry guardrail systems. Rather than competing with established safety solutions, Cupcake acts as an orchestration layer that can leverage best-in-class guardrails through its signal mechanism.

## Integration Philosophy

### Orchestration, Not Competition

Cupcake recognizes that specialized guardrail systems like NVIDIA NeMo Guardrails and Invariant have deep expertise in specific safety domains. Instead of reimplementing their capabilities, Cupcake provides:

1. **Unified Interface**: All guardrails integrate through the same signal pattern
2. **Simple Policy Integration**: Guardrail results are accessible via standard Rego checks
3. **Flexible Composition**: Combine multiple guardrails with custom logic
4. **Performance Optimization**: Only call guardrails when relevant policies match

## How Guardrail Signals Work

### 1. Signal Implementation

A guardrail signal script acts as an adapter between Cupcake and the external guardrail:

```bash
#!/bin/bash
# nemo_evaluation.sh - Adapter for NeMo Guardrails

# Extract the action details from environment or arguments
ACTION_TYPE="$1"
ACTION_CONTENT="$2"

# Call NeMo Guardrails API/CLI
NEMO_RESULT=$(nemo-guardrails evaluate \
  --action "$ACTION_TYPE" \
  --content "$ACTION_CONTENT" \
  --format json)

# Transform to Cupcake-friendly format
echo "$NEMO_RESULT" | jq '{
  passed: .safe,
  confidence: .confidence_score,
  violations: .violations,
  guardrail: "nemo"
}'
```

### 2. Policy Integration

In your Rego policies, guardrail evaluations become simple boolean checks:

```rego
package cupcake.policies.safety.guardrails

import rego.v1

# METADATA
# custom:
#   routing:
#     required_events: ["PreToolUse"]
#     required_signals: ["nemo_evaluation", "invariant_evaluation"]

# Deny if NeMo Guardrails detect unsafe content
deny contains decision if {
    input.signals.nemo_evaluation.passed == false
    
    violations_text := json.marshal(input.signals.nemo_evaluation.violations)
    
    decision := {
        "reason": concat("", ["NeMo Guardrails detected safety violation: ", violations_text]),
        "severity": "HIGH",
        "rule_id": "GUARDRAIL-NEMO-001"
    }
}

# Require both guardrails to pass for sensitive operations
deny contains decision if {
    contains(input.tool_input.command, "production")
    not (input.signals.nemo_evaluation.passed == true and 
         input.signals.invariant_evaluation.passed == true)
    
    decision := {
        "reason": "Production operations require all guardrails to pass",
        "severity": "CRITICAL",
        "rule_id": "GUARDRAIL-MULTI-001"
    }
}
```

## Supported Integrations

### Production Ready
- (Coming Soon)

### In Development
- [NVIDIA NeMo Guardrails](./guardrail-integrations/nvidia-nemo.md)
- [Invariant](./guardrail-integrations/invariant.md)

### Planned
- Anthropic Constitutional AI
- OpenAI Moderation API
- Azure Content Safety
- Google Perspective API
- Custom ML Models via REST APIs

## Benefits of This Approach

1. **Best of Both Worlds**: Leverage specialized guardrails while maintaining Cupcake's flexible policy engine
2. **Vendor Agnostic**: Switch or combine guardrails without changing policy structure
3. **Progressive Enhancement**: Start with basic policies, add guardrails as needed
4. **Unified Logging**: All guardrail decisions flow through Cupcake's audit system
5. **Cost Optimization**: Only invoke expensive guardrails when relevant policies require them

## Implementation Considerations

### Performance
- Guardrail signals may have higher latency than simple context signals
- Consider caching guardrail responses for identical inputs
- Use `timeout_seconds` in signal configuration to prevent hanging

### Error Handling
- Guardrail failures should fail-safe (deny by default)
- Log guardrail errors separately from policy violations
- Consider fallback guardrails for critical paths

### Testing
- Mock guardrail responses for unit tests
- Maintain test fixtures for common guardrail outputs
- Monitor guardrail API quotas and rate limits