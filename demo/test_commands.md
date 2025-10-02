# Quick Demo Commands

## Test Claude Code Settings
Copy `.claude/settings.json` to your project's `.claude/` directory, then try these commands:

### Should be BLOCKED:
- `Write a file called test.txt` 
- `rm *.log`
- `rm -rf /`

### Should ASK for confirmation:
- `sudo ls`
- `sudo docker ps`

### Should be ALLOWED:
- `Write a file called test.md`
- `ls -la`
- `git status`

## Manual Testing:
```bash
# Test file blocking
echo '{"hook_event_name": "PreToolUse", "tool_name": "Write", "tool_input": {"file_path": "/tmp/test.txt", "content": "test"}, "session_id": "test", "transcript_path": "/tmp/test.txt", "cwd": "/tmp"}' | cargo run -- eval --policy-dir demo

# Test bash blocking  
echo '{"hook_event_name": "PreToolUse", "tool_name": "Bash", "tool_input": {"command": "rm *.txt"}, "session_id": "test", "transcript_path": "/tmp/test.txt", "cwd": "/tmp"}' | cargo run -- eval --policy-dir demo

# Test sudo ask
echo '{"hook_event_name": "PreToolUse", "tool_name": "Bash", "tool_input": {"command": "sudo ls"}, "session_id": "test", "transcript_path": "/tmp/test.txt", "cwd": "/tmp"}' | cargo run -- eval --policy-dir demo
```

## Check audit log:
```bash
tail -f /tmp/cupcake_audit.log
```