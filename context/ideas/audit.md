1.  **A First-Class Audit Trail:** The `audit` command is currently a placeholder, but the `engine/audit.rs` and `command_executor` modules lay the groundwork for a killer feature. A rich, queryable audit log is essential for enterprise adoption, security reviews, and debugging agent behavior.
    - **Vision:** The `cupcake audit` command should become a powerful query tool. Imagine commands like:
      - `cupcake audit --session <id>`: Show all policy decisions for a specific session.
      - `cupcake audit --policy "Block dangerous commands"`: Show every time a specific policy was triggered.
      - `cupcake audit --decision block`: Show all blocked actions across all sessions.
      - `cupcake audit --since 24h`: Review all agent activity from the last day.
    - This turns Cupcake from just an enforcer into an observability platform for AI agents.

---

Audit Capabilities Analysis

When Developed

- July 15, 2025 - Part of Plan 008 Part 3 (Shell Escape Hatch implementation)
- Added as a post-review security enhancement for enterprise-grade auditing

Current Implementation

What it does:

1. Logs all command executions (from run_command actions and check conditions)
2. Two sink types:


    - StdoutSink - Prints JSON to stdout
    - FileSink - Writes to ~/.cupcake/audit/exec-YYYYMMDD.jsonl with daily

rotation 3. Audit record includes: - timestamp - command type (array/string/shell) - full command details - exit code - stdout/stderr (if available) - execution duration - correlation_id (session_id)

Actual Library Support

Very Limited:

- Only used in CommandExecutor for command execution
- NOT integrated with:
  - Policy evaluation decisions
  - State management operations
  - Action execution (other than run_command)
  - Hook lifecycle events
  - User prompt submissions
  - File operations (Read/Write/Edit)

Critical Limitations

1. Incomplete Coverage - Only audits command execution, missing 90% of
   security-relevant events
2. No Audit Command - The cupcake audit CLI command exists but appears to be a
   stub
3. No Retention Policy - Daily files accumulate forever
4. No Search/Query - Just raw JSONL files
5. Async Trait Issue - Makes custom audit sinks difficult to implement
6. Configuration Limited - Only on/off toggle, no sink selection or filtering

Assessment

The audit system is minimally viable but far from production-ready. It was added
late in development (Part 3 of Plan 008) as a checkbox feature rather than a
comprehensive security solution. For a security-focused tool like Cupcake, the
audit capabilities are surprisingly weak - it only captures a small fraction of
security-relevant events and provides no tools for analysis or compliance.
