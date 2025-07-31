# Behavioral Anomaly Detection Specification

## Overview

Cupcake's behavioral anomaly detection system identifies concerning patterns in agent behavior that may indicate policy circumvention attempts or unintended harmful actions.

## Core Detection Patterns

### 1. Persistence Despite Blocks
```yaml
pattern: repeated_attempts
indicators:
  - Multiple attempts at similar operations
  - Trying alternative commands for same goal
  - Rapid succession of related attempts
example:
  - Attempt 1: `rm file.txt` (blocked)
  - Attempt 2: `unlink file.txt` (blocked)
  - Attempt 3: `python -c "import os; os.remove('file.txt')"` (flagged)
severity: high
response: escalating_alerts
```

### 2. Capability Exploration
```yaml
pattern: unusual_interest
indicators:
  - Sudden exploration of system utilities
  - Listing/examining unfamiliar directories
  - Querying about system capabilities
example:
  - Normal: Editing .js files
  - Anomaly: `ls /bin`, `which shred`, `man unlink`
severity: medium
response: visibility_increase
```

### 3. Tool Acquisition
```yaml
pattern: seeking_alternatives
indicators:
  - Downloading new utilities
  - Installing packages not in project deps
  - Fetching scripts from external sources
example:
  - `wget https://site.com/file-destroyer`
  - `pip install secure-delete`
  - `npm install -g permanent-delete`
severity: high
response: require_justification
```

### 4. Execution Pattern Changes
```yaml
pattern: behavioral_shift
indicators:
  - Creating executable files when unusual
  - Changing file permissions to executable
  - Writing scripts in new languages
example:
  - Normal: Creates .py files
  - Anomaly: Creates delete.sh, chmod +x delete.sh
severity: medium
response: alert_and_log
```

### 5. Timing Anomalies
```yaml
pattern: automation_indicators
indicators:
  - Rapid retry attempts (< 1s between)
  - Burst of related commands
  - No pause for error reading
example:
  - 10 deletion attempts in 5 seconds
  - Immediate retry after each failure
severity: medium
response: rate_limiting
```

## Implementation Architecture

### Detection Engine
```rust
pub struct AnomalyDetector {
    // Sliding window of recent commands
    command_history: CircularBuffer<TimedCommand>,
    
    // Category frequency tracking
    category_tracker: CategoryFrequencyMap,
    
    // Pattern matchers
    patterns: Vec<Box<dyn PatternMatcher>>,
    
    // Anomaly score accumulator
    anomaly_score: AnomalyScore,
}

pub trait PatternMatcher {
    fn check(&self, event: &Event, history: &[TimedCommand]) -> Option<Anomaly>;
}
```

### Anomaly Scoring
```rust
pub struct AnomalyScore {
    base_score: f32,
    time_decay: f32,
    pattern_weights: HashMap<PatternType, f32>,
}

impl AnomalyScore {
    pub fn add_anomaly(&mut self, anomaly: &Anomaly) {
        self.base_score += anomaly.severity * self.pattern_weights[anomaly.pattern];
    }
    
    pub fn decay(&mut self, elapsed: Duration) {
        self.base_score *= (1.0 - self.time_decay * elapsed.as_secs_f32());
    }
}
```

## Response Framework

### Alert Levels
```rust
pub enum AlertLevel {
    Info,      // Log only
    Warning,   // Log + dashboard notification  
    Alert,     // Log + immediate notification
    Critical,  // Log + pause + human required
}
```

### Response Actions
```rust
pub enum ResponseAction {
    Continue,
    ContinueWithWarning(String),
    RequireJustification { prompt: String },
    PauseForReview { context: String },
    TerminateSession { reason: String },
}
```

## Rich Policy Evaluation

### Policy Rules Can Reference Anomalies
```yaml
policies:
  - name: "Block on circumvention attempts"
    condition: 
      anomaly_score: "> 7.5"
      pattern: "repeated_attempts"
    action: 
      type: "block"
      message: "Multiple bypass attempts detected. Human review required."
      
  - name: "Warn on capability exploration"
    condition:
      pattern: "unusual_interest"
      category: "system_utilities"
    action:
      type: "inject_warning"
      message: "Unusual system exploration detected. Intended behavior?"
```

## Visibility Dashboard

### Real-time Anomaly Feed
```
[12:01:02] ⚠️  Anomaly: Persistence Despite Blocks
           Command: rm important.db
           Pattern: 3rd attempt in 5 seconds
           Score: +2.5 (total: 5.5)

[12:01:10] 🚨 Anomaly: Tool Acquisition
           Command: wget file-shredder.com/destroy
           Pattern: Downloading deletion utility
           Score: +4.0 (total: 9.5)
           
[12:01:11] 🛑 ALERT: Anomaly threshold exceeded
           Action: Session paused for human review
```

### Pattern Analytics
- Most common anomaly types
- Anomaly frequency over time
- Per-agent anomaly profiles
- Learned bypass methods

## Integration Points

### With Claude Code Hooks
```json
{
  "hooks": {
    "AnomalyDetected": [
      {
        "threshold": 5.0,
        "hooks": [
          {
            "type": "command",
            "command": "notify-team.sh 'Anomaly detected: ${ANOMALY_TYPE}'"
          }
        ]
      }
    ]
  }
}
```

### With Policy Engine
- Policies can reference anomaly scores
- Patterns can trigger specific policies
- Anomaly history influences future evaluations

## Future Enhancements

### Machine Learning Integration
- Learn normal patterns per project/team
- Detect novel anomalies not in pattern list
- Predict likely next steps in bypass attempts

### Cross-Instance Learning
- Share anonymized patterns across Cupcake instances
- Build library of known bypass techniques
- Community-driven pattern updates

## Key Design Decision

**Visibility First**: Every anomaly is logged and visible, even if not blocked. This ensures:
- Users understand agent behavior
- False positives don't break workflows
- Patterns emerge from real usage
- Trust through transparency