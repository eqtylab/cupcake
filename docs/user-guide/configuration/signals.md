# Cupcake Signals System

**Status:** v0.2.0 Design Specification  
**Date:** August 2025  

## Overview

Signals enable you to gather additional context that you need within the rule enforcement layer of Cupcake. You use signals when an agent's action in isolation does not provide full context on its own. They bridge the gap between what the agent attempts to do and what the environment actually looks like.

Signals have dual-use capability:
- **Context Gathering**: Fetch real-time state about git branches, test results, deployment status, database connections, etc.
- **Evaluation Delegation**: Integrate existing industry guardrails (NVIDIA NeMo, Invariant) as signal evaluators

## Core Design Principles

### 1. **JSON-First Data Model**
Signals can output any valid JSON structure. The engine attempts to parse all signal outputs as JSON, falling back to plain strings for non-JSON output. This enables rich, structured data access in policies.

### 2. **Orchestration, Not Competition**
Cupcake doesn't compete with existing guardrails. Through signals, it orchestrates them. A signal can call NeMo Guardrails or Invariant, returning their evaluation results for use in Rego policies with simple checks like `input.signals.nemo_evaluation.passed == true`.

### 3. **User Responsibility**
Signal authors are responsible for outputting well-formed data. The engine provides the plumbing but doesn't validate or transform signal semantics. This gives maximum flexibility for integration patterns.

### 4. **Graceful Degradation**
Invalid JSON doesn't break policy evaluation - it's stored as a string and users can debug by examining the raw output.

### 5. **Performance First**
Signals execute concurrently and only when required by matched policies. O(1) routing ensures minimal overhead.

## How Signals Work

### 1. **Declaration in Policies**

Policies declare their signal dependencies in OPA metadata:

```rego
# METADATA
# scope: package
# title: Deployment Safety Guard
# custom:
#   routing:
#     required_events: ["PreToolUse"]
#     required_tools: ["Bash"]
#     required_signals: ["deployment_status", "test_results", "git_branch"]
package cupcake.policies.deployment.safety

import rego.v1

deny contains decision if {
    # Use structured signal data
    input.signals.test_results.passing == false
    input.signals.deployment_status.environment == "production"
    contains(input.tool_input.command, "kubectl apply")
    
    decision := {
        "reason": "Cannot deploy to production with failing tests",
        "severity": "HIGH",
        "rule_id": "DEPLOY-001"
    }
}
```

### 2. **Signal Definition**

Signals are defined through two mechanisms:

#### **Convention-Based Discovery (Recommended)**
Place executable scripts in `.cupcake/signals/`:

```bash
.cupcake/
├── signals/
│   ├── git_branch.sh          # → signal name: "git_branch"
│   ├── test_results.py        # → signal name: "test_results"  
│   └── deployment_status      # → signal name: "deployment_status"
```

#### **Explicit Guidebook Configuration**
Override or supplement auto-discovered signals in `.cupcake/guidebook.yml`:

```yaml
signals:
  # Override auto-discovered script with inline command
  git_branch:
    command: "git rev-parse --abbrev-ref HEAD"
    timeout_seconds: 3
  
  # Define new signal not in filesystem
  security_scan:
    command: "trivy fs --format json --quiet ."
    timeout_seconds: 30
```

### 3. **Signal Execution**

When a policy evaluation requires signals:

1. **Collection**: Engine identifies unique signals from all matched policies
2. **Concurrent Execution**: All required signals execute simultaneously
3. **JSON Parsing**: Each signal output is parsed as JSON with string fallback
4. **Input Enrichment**: Parsed data is injected into policy input as `input.signals.<name>`

## Signal Output Formats

### **String Signals**
Simple text outputs are stored as JSON strings:

```bash
#!/bin/bash
# git_branch.sh
git rev-parse --abbrev-ref HEAD
```

Output: `"main"` → Access as: `input.signals.git_branch == "main"`

### **Structured Signals**  
JSON outputs enable rich data access:

```python
#!/usr/bin/env python3
# test_results.py
import json
import subprocess

result = subprocess.run(['pytest', '--json-report'], capture_output=True)
if result.returncode == 0:
    print(json.dumps({
        "passing": True,
        "coverage": 94.2,
        "duration": 12.5,
        "failed_tests": []
    }))
else:
    print(json.dumps({
        "passing": False,
        "coverage": 87.1,
        "duration": 8.3,
        "failed_tests": ["test_auth.py::test_login", "test_db.py::test_connection"]
    }))
```

Output: `{"passing": false, "coverage": 87.1, ...}` 

Access as:
- `input.signals.test_results.passing == false`
- `input.signals.test_results.coverage < 90` 
- `count(input.signals.test_results.failed_tests) > 0`

### **Complex Structures**
Signals can return arrays, nested objects, or any JSON structure:

```bash
#!/bin/bash
# security_scan.sh
trivy fs --format json --quiet . | jq '{
  "critical_vulnerabilities": [.Results[]?.Vulnerabilities[]? | select(.Severity == "CRITICAL")],
  "high_vulnerabilities": [.Results[]?.Vulnerabilities[]? | select(.Severity == "HIGH")],
  "scan_time": now | todate
}'
```

Access as:
- `count(input.signals.security_scan.critical_vulnerabilities) > 0`
- `input.signals.security_scan.scan_time`

## Error Handling

### **Invalid JSON Output**
Non-JSON output is stored as a string:

```bash
#!/bin/bash
# broken_signal.sh
echo "Error: database connection failed"
```

Results in: `input.signals.broken_signal == "Error: database connection failed"`

### **Signal Execution Failures**
- Command timeouts are logged but don't fail evaluation
- Non-zero exit codes are logged but don't fail evaluation  
- Missing signals are logged but don't fail evaluation
- Policies receive empty signal data and should handle gracefully

### **Debug Information**
Enable debug logging to troubleshoot signal issues:

```bash
RUST_LOG=debug cupcake eval --policy-dir .cupcake/policies
```

Debug output shows:
- Which signals are being executed
- Raw signal outputs before JSON parsing
- JSON parsing success/failure
- Signal execution timing

## Best Practices

### **Signal Design**
1. **Output valid JSON** when you need structured access
2. **Keep signals fast** - they run on every policy evaluation
3. **Handle errors gracefully** - return meaningful JSON even on failure
4. **Use timeouts** - default 5s is usually sufficient
5. **Test signal outputs** independently before using in policies

### **Policy Usage**
1. **Check signal existence** before accessing nested fields:
   ```rego
   input.signals.test_results.passing == false  # Assumes signal exists
   
   # Better: 
   input.signals.test_results
   input.signals.test_results.passing == false
   ```

2. **Handle missing signals**:
   ```rego
   # Provide defaults for missing signals
   branch := object.get(input.signals, "git_branch", "unknown")
   ```

3. **Use appropriate data types**:
   ```rego
   # String comparison
   input.signals.environment == "production"
   
   # Numeric comparison  
   input.signals.test_coverage > 80
   
   # Boolean logic
   input.signals.tests_passing == true
   ```

## Performance Characteristics

- **Signal Discovery**: Fast filesystem scan and YAML parsing
- **Signal Execution**: Depends on signal complexity, executed concurrently
- **JSON Parsing**: Minimal overhead per signal output
- **Input Enrichment**: Efficient serialization into policy input

**Optimization**: Only signals required by matched policies are executed. Unused signals have zero performance impact.

## Migration Guide

### From String-Based Signals (v0.1.x)
Old policies expecting string access continue to work:

```rego
# This still works if git_branch outputs a JSON string
input.signals.git_branch == "main"
```

New structured access is now possible:

```rego
# This now works if test_status outputs JSON
input.signals.test_status.passing == false
```

### Signal Output Updates
Update your signals to output JSON for structured access:

```bash
# Old: plain string
echo "main"

# New: JSON string (enables both string and object access)
echo '"main"'

# New: structured object
echo '{"branch": "main", "commit": "abc123", "dirty": false}'
```

## Examples

Example signal implementations can be created in your project's `.cupcake/signals/` directory:
- Git branch detection: `git_branch.sh` returning current branch name
- Test status: `test_status.sh` returning test execution results
- Any custom shell script that outputs data for policy evaluation