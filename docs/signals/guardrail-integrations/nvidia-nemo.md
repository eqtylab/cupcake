# NVIDIA NeMo Guardrails Integration

**Status:** Design Specification  
**Integration Type:** Evaluation Signal  
**Vendor:** NVIDIA

## Overview

NVIDIA NeMo Guardrails is a toolkit for adding programmable guardrails to LLM-based conversational systems. By integrating NeMo as a signal, Cupcake can leverage its sophisticated safety checks while maintaining policy flexibility.

## What NeMo Guardrails Provides

- **Input/Output Filtering**: Detect and prevent harmful, biased, or inappropriate content
- **Dialogue Flow Control**: Ensure conversations follow intended patterns
- **Fact Checking**: Validate factual accuracy of responses
- **Jailbreak Detection**: Identify attempts to bypass safety measures
- **Topic Control**: Keep conversations within approved boundaries

## Integration Architecture

```
Claude Code Action → Cupcake → NeMo Signal → NeMo Guardrails API
                        ↓                            ↓
                   Rego Policy ← JSON Result ←-------┘
```

## Signal Implementation

### Basic Integration Script

```bash
#!/bin/bash
# signals/nemo_evaluation.sh

# Parse the Claude Code event
EVENT_TYPE="${CUPCAKE_EVENT_TYPE}"
TOOL_NAME="${CUPCAKE_TOOL_NAME}"
CONTENT="${CUPCAKE_TOOL_INPUT}"

# Prepare NeMo request
NEMO_REQUEST=$(jq -n \
  --arg event "$EVENT_TYPE" \
  --arg tool "$TOOL_NAME" \
  --argjson content "$CONTENT" \
  '{
    messages: [{
      role: "user",
      content: $content.command
    }],
    config: {
      rails: ["input_filtering", "jailbreak_detection"],
      sensitivity: "high"
    }
  }')

# Call NeMo Guardrails (example using Python client)
RESULT=$(python -c "
import json
import sys
from nemoguardrails import LLMRails

config = {
    'models': [{
        'type': 'main',
        'engine': 'openai',
        'model': 'gpt-3.5-turbo'
    }],
    'rails': {
        'input': {
            'flows': ['check_blocked_terms', 'check_jailbreak']
        }
    }
}

rails = LLMRails(config)
request = json.loads('$NEMO_REQUEST')

try:
    result = rails.generate(**request)
    output = {
        'passed': not result.get('blocked', False),
        'confidence': result.get('confidence', 0.0),
        'violations': result.get('violations', []),
        'explanation': result.get('explanation', ''),
        'guardrail': 'nemo',
        'version': '0.5.0'
    }
except Exception as e:
    output = {
        'passed': False,
        'error': str(e),
        'guardrail': 'nemo'
    }

print(json.dumps(output))
")

echo "$RESULT"
```

## Policy Examples

### Simple Pass/Fail Check

```rego
package cupcake.policies.safety.nemo

import rego.v1

# METADATA
# custom:
#   routing:
#     required_events: ["PreToolUse"]
#     required_signals: ["nemo_evaluation"]

deny contains decision if {
    input.signals.nemo_evaluation.passed == false
    
    decision := {
        "reason": sprintf("NeMo Guardrails blocked: %s", 
                         [input.signals.nemo_evaluation.explanation]),
        "severity": "HIGH",
        "rule_id": "NEMO-BLOCK-001"
    }
}
```

### Confidence-Based Decisions

```rego
ask contains decision if {
    # Low confidence detections require user confirmation
    input.signals.nemo_evaluation.confidence < 0.7
    input.signals.nemo_evaluation.passed == false
    
    decision := {
        "reason": "Potential safety concern detected. Please confirm this action.",
        "severity": "MEDIUM",
        "rule_id": "NEMO-CONFIRM-001"
    }
}
```

## Configuration Options

### Signal Configuration in guidebook.yml

```yaml
signals:
  nemo_evaluation:
    command: "./signals/nemo_evaluation.sh"
    timeout_seconds: 5  # NeMo can be slow for complex checks
    env:
      NEMO_API_KEY: "${NEMO_API_KEY}"
      NEMO_ENDPOINT: "https://api.nemo-guardrails.nvidia.com"
      NEMO_SENSITIVITY: "high"  # low, medium, high
```

## Performance Considerations

- **Latency**: NeMo evaluation can add 100-500ms per request
- **Caching**: Consider caching identical inputs for 5-10 minutes
- **Batch Processing**: NeMo supports batch evaluation for multiple inputs
- **Fallback**: Have a simpler guardrail as fallback if NeMo is unavailable

## Error Handling

When NeMo is unavailable or returns an error, policies should fail-safe:

```rego
deny contains decision if {
    # Fail-safe: deny if NeMo errored
    input.signals.nemo_evaluation.error
    
    decision := {
        "reason": "Safety evaluation unavailable - denying by default",
        "severity": "HIGH",
        "rule_id": "NEMO-ERROR-001"
    }
}
```

## Limitations

- Requires NeMo Guardrails installation and configuration
- May have rate limits on cloud API
- Adds latency to policy evaluation
- English-primary with limited multilingual support

## Resources

- [NeMo Guardrails Documentation](https://github.com/NVIDIA/NeMo-Guardrails)
- [Configuration Guide](https://docs.nvidia.com/nemo/guardrails/index.html)
- [Python Client Library](https://pypi.org/project/nemoguardrails/)