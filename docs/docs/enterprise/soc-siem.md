# SOC & SIEM

Every policy evaluation produces structured decision data—what tool was called, what the agent intended, what Cupcake decided, and why. This is enriched telemetry ready for your security operations tooling.

## Configuration

```yaml
# .cupcake/rulebook.yml
telemetry:
  enabled: true
  format: json
  destination: /var/log/cupcake/
```

| Field | Default | Description |
|-------|---------|-------------|
| `enabled` | `false` | Enable telemetry export |
| `format` | `json` | Output format (`json` or `text`) |
| `destination` | `.cupcake/telemetry/` | Output directory |

## Output

Each evaluation writes a JSON file:

```
/var/log/cupcake/2025-12-07_14-23-45_abc123.json
```

## Schema

```json
{
  "trace_id": "abc123",
  "timestamp": "2025-12-07T14:23:45Z",
  "event_received": {
    "hook_event_name": "PreToolUse",
    "tool_name": "Bash",
    "tool_input": { "command": "rm -rf /tmp/*" }
  },
  "routed": true,
  "matched_policies": ["cupcake.policies.security"],
  "final_decision": {
    "type": "Deny",
    "reason": "Dangerous rm command blocked"
  },
  "response_to_agent": { ... }
}
```

## Integration

Point your SIEM collector (Filebeat, Fluentd, Splunk UF) at the destination directory. Standard JSON—no custom parsing required.
