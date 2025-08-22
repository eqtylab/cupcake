# Invariant Integration

**Status:** Design Specification  
**Integration Type:** Evaluation Signal  
**Vendor:** Invariant Labs

## Overview

Invariant is a comprehensive AI safety platform that provides monitoring, evaluation, and protection for AI systems. By integrating Invariant as a signal, Cupcake can leverage its advanced safety metrics and real-time monitoring capabilities.

## What Invariant Provides

- **Safety Scoring**: Real-time safety assessment of AI outputs
- **Policy Compliance**: Verify adherence to organizational policies
- **Hallucination Detection**: Identify factually incorrect statements
- **PII Detection**: Find and flag personally identifiable information
- **Toxicity Analysis**: Detect harmful or offensive content
- **Security Scanning**: Identify potential security vulnerabilities in generated code

## Integration Architecture

```
Claude Code Action → Cupcake → Invariant Signal → Invariant API
                        ↓                              ↓
                   Rego Policy ← JSON Result ←---------┘
```

## Signal Implementation

### Basic Integration Script

```bash
#!/bin/bash
# signals/invariant_evaluation.sh

# Extract relevant fields from the Cupcake event
EVENT_DATA="$1"
TOOL_NAME=$(echo "$EVENT_DATA" | jq -r '.tool_name')
TOOL_INPUT=$(echo "$EVENT_DATA" | jq -r '.tool_input')

# Prepare Invariant API request
INVARIANT_REQUEST=$(jq -n \
  --arg tool "$TOOL_NAME" \
  --argjson input "$TOOL_INPUT" \
  '{
    input: {
      type: "code_generation",
      content: $input.command,
      context: {
        tool: $tool,
        source: "claude-code"
      }
    },
    checks: [
      "safety_score",
      "pii_detection",
      "code_security",
      "hallucination_check"
    ],
    threshold: {
      safety_score: 0.8,
      confidence: 0.7
    }
  }')

# Call Invariant API
RESPONSE=$(curl -s -X POST \
  -H "Authorization: Bearer ${INVARIANT_API_KEY}" \
  -H "Content-Type: application/json" \
  -d "$INVARIANT_REQUEST" \
  "${INVARIANT_ENDPOINT}/v1/evaluate")

# Transform to Cupcake-friendly format
echo "$RESPONSE" | jq '{
  passed: .overall_passed,
  safety_score: .metrics.safety_score,
  confidence: .metrics.confidence,
  violations: [.checks[] | select(.passed == false) | {
    type: .check_type,
    severity: .severity,
    message: .message
  }],
  pii_detected: .checks.pii_detection.found_pii,
  security_issues: .checks.code_security.issues,
  guardrail: "invariant",
  evaluation_id: .evaluation_id
}'
```

## Policy Examples

### Basic Safety Check

```rego
package cupcake.policies.safety.invariant

import rego.v1

# METADATA
# custom:
#   routing:
#     required_events: ["PreToolUse", "PostToolUse"]
#     required_signals: ["invariant_evaluation"]

# Deny if Invariant safety score is too low
deny contains decision if {
    input.signals.invariant_evaluation.safety_score < 0.8
    
    decision := {
        "reason": sprintf("Safety score too low: %.2f (minimum: 0.8)", 
                         [input.signals.invariant_evaluation.safety_score]),
        "severity": "HIGH",
        "rule_id": "INVARIANT-SAFETY-001"
    }
}
```

### PII Protection

```rego
# Block if PII is detected
halt contains decision if {
    input.signals.invariant_evaluation.pii_detected == true
    
    decision := {
        "reason": "Personally Identifiable Information detected - immediate halt required",
        "severity": "CRITICAL",
        "rule_id": "INVARIANT-PII-001"
    }
}
```

### Code Security Checks

```rego
# Ask for confirmation on security issues
ask contains decision if {
    count(input.signals.invariant_evaluation.security_issues) > 0
    
    issues_text := concat(", ", [issue.type | 
        issue := input.signals.invariant_evaluation.security_issues[_]])
    
    decision := {
        "reason": sprintf("Security concerns detected: %s. Continue anyway?", 
                         [issues_text]),
        "severity": "MEDIUM",
        "rule_id": "INVARIANT-SEC-001"
    }
}
```

## Configuration Options

### Signal Configuration in guidebook.yml

```yaml
signals:
  invariant_evaluation:
    command: "./signals/invariant_evaluation.sh"
    timeout_seconds: 3
    env:
      INVARIANT_API_KEY: "${INVARIANT_API_KEY}"
      INVARIANT_ENDPOINT: "https://api.invariantlabs.ai"
      INVARIANT_MODE: "strict"  # strict, balanced, permissive
```

## Advanced Features

### Multi-Stage Evaluation

```rego
# Use different Invariant thresholds for different environments
deny contains decision if {
    input.signals.environment == "production"
    input.signals.invariant_evaluation.safety_score < 0.95  # Stricter in prod
    
    decision := {
        "reason": "Production requires 95% safety score",
        "severity": "HIGH",
        "rule_id": "INVARIANT-PROD-001"
    }
}

deny contains decision if {
    input.signals.environment == "development"
    input.signals.invariant_evaluation.safety_score < 0.7  # More lenient in dev
    
    decision := {
        "reason": "Development safety threshold not met",
        "severity": "MEDIUM",
        "rule_id": "INVARIANT-DEV-001"
    }
}
```

## Performance Considerations

- **Latency**: Typically 50-200ms for standard evaluations
- **Caching**: Invariant supports response caching via evaluation_id
- **Batch Mode**: Can evaluate multiple inputs in a single API call
- **Async Mode**: Supports webhook callbacks for long-running evaluations

## Error Handling

```rego
# Fail-safe when Invariant is unavailable
deny contains decision if {
    input.signals.invariant_evaluation.error
    contains(input.tool_input.command, "production")
    
    decision := {
        "reason": "Safety evaluation unavailable for production operations",
        "severity": "HIGH",
        "rule_id": "INVARIANT-FAILSAFE-001"
    }
}
```

## Monitoring and Analytics

Invariant provides:
- Dashboard for evaluation metrics
- Trend analysis of safety scores
- Violation patterns and insights
- Integration with observability platforms

## Limitations

- Requires Invariant API subscription
- Rate limits based on pricing tier
- Some checks require additional configuration
- Limited offline capability

## Resources

- [Invariant Documentation](https://docs.invariantlabs.ai)
- [API Reference](https://api.invariantlabs.ai/docs)
- [Integration Examples](https://github.com/invariantlabs/examples)