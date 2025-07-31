# Debug Logging

Cupcake includes comprehensive debug logging to help diagnose hook execution issues, particularly when integrating with Claude Code or other AI tools.

## Overview

Debug logging is automatically enabled in the `run` command and logs all hook invocations, input data, and errors to `/tmp/cupcake-debug.log`. This logging is essential for troubleshooting cases where hooks are firing but configuration issues prevent proper execution.

## Log Location

All debug logs are written to:
```
/tmp/cupcake-debug.log
```

## What Gets Logged

### 1. Every Invocation
Each time Cupcake is invoked, it logs:
- Timestamp with millisecond precision
- Event type (PreToolUse, PostToolUse, etc.)
- Config file path
- Debug mode status

```
[2025-07-30 19:01:44.123] Cupcake invoked - Event: PreToolUse, Config: /path/to/.cupcake/config.yaml, Debug: false
```

### 2. STDIN Content
All input received from stdin is logged:
- Shows actual hook event data
- Displays `[EMPTY]` for empty input
- Helps debug JSON parsing issues

```
  STDIN received: {"tool": "Bash", "args": {"command": "rm file.txt"}}
```

### 3. Errors
Any errors during hook processing are logged:
- Configuration file errors
- Hook event parsing failures
- Execution errors

```
  ERROR reading hook event: Config file not found at /path/to/.cupcake/config.yaml
```

## Implementation Details

### Logging Code Pattern
```rust
// Log to /tmp/cupcake-debug.log
if let Ok(mut file) = std::fs::OpenOptions::new()
    .create(true)
    .append(true)
    .open("/tmp/cupcake-debug.log")
{
    use std::io::Write;
    let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
    writeln!(file, "[{}] Message here", timestamp).ok();
}
```

### Key Design Decisions

1. **Always Active**: Logging happens regardless of debug mode setting
2. **Graceful Failure**: Uses `.ok()` to ignore file write errors
3. **Append Mode**: Logs accumulate over time for trend analysis
4. **Structured Format**: Consistent indentation and formatting
5. **Timestamped**: Millisecond precision for timing analysis

## Debugging Common Issues

### Hook Not Firing
If you expect a hook to fire but nothing happens:
1. Check if there are any log entries at all
2. Verify the event type matches your configuration
3. Look for configuration file errors

### Hook Firing But Not Working
If logs show invocation but hook doesn't execute properly:
1. Check the STDIN content for malformed JSON
2. Look for parsing errors in the logs
3. Verify config file path is correct

### Configuration Issues
Common configuration problems show up as:
```
  ERROR reading hook event: Config file not found at /path/to/.cupcake/config.yaml
  ERROR reading hook event: Invalid YAML syntax in config file
```

## Example Debug Session

```bash
# Trigger a hook
echo '{"tool": "Bash", "args": {"command": "rm test.txt"}}' | cupcake run PreToolUse

# Check the logs
tail -f /tmp/cupcake-debug.log
```

Example output:
```
[2025-07-30 19:01:44.123] Cupcake invoked - Event: PreToolUse, Config: /home/user/.cupcake/config.yaml, Debug: false
  STDIN received: {"tool": "Bash", "args": {"command": "rm test.txt"}}
[2025-07-30 19:01:44.456] Policy evaluation: file_deletion_policy
[2025-07-30 19:01:44.789] Action: block with feedback
```

## Log Rotation

Currently, logs append indefinitely to `/tmp/cupcake-debug.log`. For long-running systems, consider:

```bash
# Rotate logs manually
mv /tmp/cupcake-debug.log /tmp/cupcake-debug.log.old
```

Or set up logrotate:
```
/tmp/cupcake-debug.log {
    daily
    rotate 7
    compress
    missingok
    notifempty
}
```

## Privacy Considerations

The debug log may contain sensitive information from hook events, including:
- File paths
- Command arguments
- Tool parameters

Ensure the log file has appropriate permissions and is cleaned up regularly in production environments.

## Troubleshooting Tips

1. **Empty logs**: Check file permissions on `/tmp/`
2. **No STDIN content**: Hook caller may not be sending data
3. **Truncated logs**: File system space issues
4. **Permission errors**: `/tmp/` directory permissions

The debug logging was implemented specifically to address Claude Code integration challenges where hooks were firing but configuration paths weren't being resolved correctly.