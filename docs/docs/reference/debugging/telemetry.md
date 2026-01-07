---
layout: "@/layouts/mdx-layout.astro"
title: "Telemetry"
heading: "Telemetry & Tracing"
description: "Understanding Cupcake's telemetry spans and data structures"
---

Cupcake captures detailed telemetry for every event processed, enabling debugging, auditing, and observability integration. This page documents the span hierarchy, data structures, and output formats.

## Overview

Cupcake uses a **hierarchical span model** inspired by OpenTelemetry. Each event evaluation creates a trace with parent-child relationships:

```
cupcake.ingest (root)
├── cupcake.enrich (preprocessing)
├── cupcake.evaluate.global (global policies)
└── cupcake.evaluate.project (project policies)
```

All spans share a common `trace_id` and maintain parent-child relationships via `span_id` and `parent_span_id`.

## Enabling Telemetry

### Debug Files (CLI Flag)

Enable debug file output with the `--debug-files` flag:

```bash
cupcake eval --debug-files
```

Files are written to `.cupcake/debug/` with the format:
```
{timestamp}_{trace_id}.txt
```

### Telemetry Configuration (rulebook.yml)

Configure telemetry output in your `rulebook.yml`:

```yaml
telemetry:
  enabled: true
  format: json  # or "text"
  destination: .cupcake/telemetry
```

## Span Hierarchy

### 1. IngestSpan (Root)

The root span captures the raw event exactly as received from stdin, before any processing.

| Field | Type | Description |
|-------|------|-------------|
| `span_id` | string | Unique 16-char hex identifier |
| `parent_span_id` | string | Empty (root span) |
| `trace_id` | string | UUID v7 trace identifier |
| `start_time_unix_nano` | u64 | Start time in nanoseconds since Unix epoch |
| `end_time_unix_nano` | u64 | End time (set at finalization) |
| `raw_event` | object | The event exactly as received |
| `timestamp` | string | RFC3339 formatted timestamp |
| `harness` | string | Agent type: `ClaudeCode`, `Cursor`, `Factory`, `OpenCode` |

**Example:**
```json
{
  "span_id": "abc123def4567890",
  "parent_span_id": "",
  "trace_id": "019b0a92-e8b9-7781-889f-4e47252d167a",
  "start_time_unix_nano": 1734567890123456789,
  "end_time_unix_nano": 1734567890234567890,
  "raw_event": {
    "hook_event_name": "PreToolUse",
    "tool_name": "Bash",
    "tool_input": {"command": "rm -rf /tmp/cache"}
  },
  "timestamp": "2024-12-15T10:30:00.123Z",
  "harness": "ClaudeCode"
}
```

### 2. EnrichSpan (Preprocessing)

Child span capturing preprocessing/enrichment results. Records transformations applied to the input.

| Field | Type | Description |
|-------|------|-------------|
| `span_id` | string | Unique 16-char hex identifier |
| `parent_span_id` | string | IngestSpan's span_id |
| `start_time_unix_nano` | u64 | When preprocessing started |
| `end_time_unix_nano` | u64 | When preprocessing completed |
| `enriched_event` | object | Event after preprocessing |
| `preprocessing_operations` | string[] | Operations applied |
| `duration_us` | u64 | Duration in microseconds |

**Preprocessing Operations:**
- `whitespace_normalization` - Collapsed whitespace in commands
- `content_unification` - Unified Write/Edit content fields
- `symlink_resolution` - Resolved symlinks to canonical paths
- `opencode_field_mapping` - Mapped OpenCode fields to standard format

**Example:**
```json
{
  "span_id": "def456abc7890123",
  "parent_span_id": "abc123def4567890",
  "start_time_unix_nano": 1734567890123456789,
  "end_time_unix_nano": 1734567890123606789,
  "enriched_event": {
    "hook_event_name": "PreToolUse",
    "tool_name": "Bash",
    "tool_input": {"command": "rm -rf /tmp/cache"},
    "resolved_file_path": "/tmp/cache",
    "is_symlink": false
  },
  "preprocessing_operations": ["whitespace_normalization", "symlink_resolution"],
  "duration_us": 150
}
```

### 3. EvaluateSpan (Policy Evaluation)

Child span for each evaluation phase. Multiple evaluate spans may exist per trace.

| Field | Type | Description |
|-------|------|-------------|
| `span_id` | string | Unique 16-char hex identifier |
| `parent_span_id` | string | IngestSpan's span_id |
| `start_time_unix_nano` | u64 | When evaluation started |
| `end_time_unix_nano` | u64 | When evaluation completed |
| `phase` | string | `"global"` or `"project"` |
| `routed` | bool | Whether routing found matching policies |
| `matched_policies` | string[] | Policy package names that matched |
| `wasm_decision_set` | object | Raw decisions from WASM evaluation |
| `final_decision` | object | Synthesized final decision |
| `exit_reason` | string | Reason for early exit (if any) |
| `signals_executed` | SignalExecution[] | Signals that were run |
| `duration_ms` | u64 | Duration in milliseconds |

**Example:**
```json
{
  "span_id": "789abc123def4560",
  "parent_span_id": "abc123def4567890",
  "start_time_unix_nano": 1734567890123606789,
  "end_time_unix_nano": 1734567890125606789,
  "phase": "project",
  "routed": true,
  "matched_policies": ["cupcake.policies.bash_security", "cupcake.policies.file_guard"],
  "wasm_decision_set": {
    "halts": [],
    "denials": [],
    "blocks": [{
      "rule_id": "bash_security/dangerous_rm",
      "reason": "Blocked rm -rf command",
      "severity": "high"
    }],
    "asks": [],
    "modifications": [],
    "add_context": []
  },
  "final_decision": {
    "Block": {
      "reason": "Blocked rm -rf command",
      "severity": "high",
      "rule_id": "bash_security/dangerous_rm"
    }
  },
  "signals_executed": [],
  "duration_ms": 2
}
```

### 4. SignalExecution

Records individual signal executions within an evaluation phase.

| Field | Type | Description |
|-------|------|-------------|
| `name` | string | Signal name from rulebook |
| `command` | string | Shell command that was executed |
| `result` | any | Parsed JSON or raw output |
| `duration_ms` | u64 | Execution time in milliseconds |
| `exit_code` | i32 | Process exit code (if available) |

**Example:**
```json
{
  "name": "git_status",
  "command": "git status --porcelain",
  "result": ["M  src/main.rs", "?? new_file.txt"],
  "duration_ms": 45,
  "exit_code": 0
}
```

## TelemetryContext (Full Structure)

The complete telemetry context aggregates all spans:

```json
{
  "ingest": { /* IngestSpan */ },
  "enrich": { /* EnrichSpan (optional) */ },
  "evaluations": [ /* EvaluateSpan[] */ ],
  "response_to_agent": { /* Final response sent */ },
  "errors": [ /* Error messages */ ],
  "total_duration_ms": 15
}
```

## Output Formats

### Human-Readable Text

Debug files use a human-readable format:

```
===== Cupcake Telemetry [2024-12-15 10:30:00] [019b0a92-e8b9-7781-889f-4e47252d167a] =====
Harness: ClaudeCode
Total Duration: 15ms

----- STAGE: Ingest (Raw Event) -----
{
  "hook_event_name": "PreToolUse",
  "tool_name": "Bash"
}

----- STAGE: Enrich (Preprocessed) -----
Operations: whitespace_normalization, symlink_resolution
Duration: 150μs
Enriched Event:
{ ... }

----- STAGE: Evaluate (Policy Evaluation) -----

[Phase 1: project]
  Routed: true
  Matched Policies: cupcake.policies.bash_security
  WASM Decision Set:
    Halts: 0
    Denials: 0
    Blocks: 1
    Asks: 0
      - [BLOCK] bash_security/dangerous_rm: Blocked rm -rf command (high)
  Final Decision: Block { ... }
  Duration: 2ms

----- Response to Agent -----
{ "decision": "block", "reason": "..." }

===== End Telemetry [15ms] =====
```

### JSON Format

JSON output contains the full `TelemetryContext` structure, suitable for log aggregation systems like Loki, Elasticsearch, or Splunk. The JSON format uses OpenTelemetry-compatible span fields (`span_id`, `parent_span_id`, `start_time_unix_nano`, `end_time_unix_nano`) for compatibility with observability tools.

## Timing Guarantees

All timestamps use actual wall-clock capture times:

1. **IngestSpan** - `start_time_unix_nano` captured at span creation, `end_time_unix_nano` set at `finalize()`
2. **EnrichSpan** - Both times calculated from actual preprocessing start + measured duration
3. **EvaluateSpan** - `start_time_unix_nano` captured at span creation, `end_time_unix_nano` set at `finalize()`

This preserves real timing gaps (context switches, I/O waits) rather than using calculated offsets.

## Drop Guard

`TelemetryContext` implements `Drop` to ensure telemetry is written even if:
- The process panics
- An error causes early return via `?`
- Any unexpected exit path

This guarantees telemetry capture for debugging failures.
