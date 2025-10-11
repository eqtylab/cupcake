export const meta = {
title: 'Hooks',
description: 'Built-in hooks and how to use them effectively in Cursor.'
};

# Hooks

Hooks let you observe, control, and extend the agent loop using custom scripts. Hooks are spawned processes that communicate over stdio using JSON in both directions. They run before or after defined stages of the agent loop and can observe, block, or modify behavior.

<Frame>
  <video
    src="/docs-static/images/agent/hooks.mp4"
    autoPlay
    loop
    muted
  />
</Frame>

With hooks, you can:

- Run formatters after edits
- Add analytics for events
- Scan for PII or secrets
- Gate risky operations (e.g., SQL writes)

## Quickstart

Create a `hooks.json` file in your home directory at `~/.cursor/hooks.json`

```json
{
  "version": 1,
  "hooks": {
    "afterFileEdit": [{ "command": "./hooks/format.sh" }]
  }
}
```

Create your hook script at `~/.cursor/hooks/format.sh`:

```bash
#!/bin/bash
# Read input, do something, exit 0
cat > /dev/null
exit 0
```

Make it executable:

```bash
chmod +x ~/.cursor/hooks/format.sh
```

Restart Cursor. Your hook now runs after every file edit.

## Examples

<CodeGroup>

```json title="hooks.json"
{
  "version": 1,
  "hooks": {
    "beforeShellExecution": [
      {
        "command": "./hooks/audit.sh"
      },
      {
        "command": "./hooks/block-git.sh"
      }
    ],
    "beforeMCPExecution": [
      {
        "command": "./hooks/audit.sh"
      }
    ],
    "beforeReadFile": [
      {
        "command": "./hooks/redact-secrets.sh"
      }
    ],
    "afterFileEdit": [
      {
        "command": "./hooks/audit.sh"
      }
    ],
    "beforeSubmitPrompt": [
      {
        "command": "./hooks/audit.sh"
      }
    ],
    "stop": [
      {
        "command": "./hooks/audit.sh"
      }
    ]
  }
}
```

```sh title="audit.sh"
#!/bin/bash

# audit.sh - Hook script that writes all JSON input to /tmp/agent-audit.log
# This script is designed to be called by Cursor's hooks system for auditing purposes

# Read JSON input from stdin
json_input=$(cat)

# Create timestamp for the log entry
timestamp=$(date '+%Y-%m-%d %H:%M:%S')

# Create the log directory if it doesn't exist
mkdir -p "$(dirname /tmp/agent-audit.log)"

# Write the timestamped JSON entry to the audit log
echo "[$timestamp] $json_input" >> /tmp/agent-audit.log

# Exit successfully
exit 0
```

```sh title="block-git.sh"
#!/bin/bash

# Hook to block git commands and redirect to gh tool usage
# This hook implements the beforeShellExecution hook from the Cursor Hooks Spec

# Initialize debug logging
echo "Hook execution started" >> /tmp/hooks.log

# Read JSON input from stdin
input=$(cat)
echo "Received input: $input" >> /tmp/hooks.log

# Parse the command from the JSON input
command=$(echo "$input" | jq -r '.command // empty')
echo "Parsed command: '$command'" >> /tmp/hooks.log

# Check if the command contains 'git' or 'gh'
if [[ "$command" =~ git[[:space:]] ]] || [[ "$command" == "git" ]]; then
    echo "Git command detected - blocking: '$command'" >> /tmp/hooks.log
    # Block the git command and provide guidance to use gh tool instead
    cat << EOF
{
  "continue": true,
  "permission": "deny",
  "userMessage": "Git command blocked. Please use the GitHub CLI (gh) tool instead.",
  "agentMessage": "The git command '$command' has been blocked by a hook. Instead of using raw git commands, please use the 'gh' tool which provides better integration with GitHub and follows best practices. For example:\n- Instead of 'git clone', use 'gh repo clone'\n- Instead of 'git push', use 'gh repo sync' or the appropriate gh command\n- For other git operations, check if there's an equivalent gh command or use the GitHub web interface\n\nThis helps maintain consistency and leverages GitHub's enhanced tooling."
}
EOF
elif [[ "$command" =~ gh[[:space:]] ]] || [[ "$command" == "gh" ]]; then
    echo "GitHub CLI command detected - asking for permission: '$command'" >> /tmp/hooks.log
    # Ask for permission for gh commands
    cat << EOF
{
  "continue": true,
  "permission": "ask",
  "userMessage": "GitHub CLI command requires permission: $command",
  "agentMessage": "The command '$command' uses the GitHub CLI (gh) which can interact with your GitHub repositories and account. Please review and approve this command if you want to proceed."
}
EOF
else
    echo "Non-git/non-gh command detected - allowing: '$command'" >> /tmp/hooks.log
    # Allow non-git/non-gh commands
    cat << EOF
{
  "continue": true,
  "permission": "allow"
}
EOF
fi
```

```sh title="redact-secrets.sh"
#!/bin/bash

# Secrets hide in code
# Like shadows in the moonlight
# This hook finds them all

# redact-secrets.sh - Hook script that checks for GitHub API keys in file content
# This script implements a file content validation hook from the Cursor Hooks Spec

# Initialize debug logging
echo "Redact-secrets hook execution started" >> /tmp/hooks.log

# Read JSON input from stdin
input=$(cat)
echo "Received input: $input" >> /tmp/hooks.log

# Parse the file path and content from the JSON input
file_path=$(echo "$input" | jq -r '.file_path // empty')
content=$(echo "$input" | jq -r '.content // empty')
attachments_count=$(echo "$input" | jq -r '.attachments | length // 0')
echo "Parsed file path: '$file_path'" >> /tmp/hooks.log
echo "Attachments count: $attachments_count" >> /tmp/hooks.log
echo "Content length: ${#content} characters" >> /tmp/hooks.log

# Check if the content contains a GitHub API key pattern
# Pattern explanation: GitHub personal access tokens (ghp_), GitHub app tokens (ghs_), or test keys (gh_api_) followed by alphanumeric characters
if echo "$content" | grep -qE 'gh[ps]_[A-Za-z0-9]{36}|gh_api_[A-Za-z0-9]+'; then
    echo "GitHub API key detected in file: '$file_path'" >> /tmp/hooks.log
    # Deny permission if GitHub API key is detected
    cat << EOF
{
  "permission": "deny"
}
EOF
    exit 3
else
    echo "No GitHub API key detected in file: '$file_path' - allowing" >> /tmp/hooks.log
    # Allow permission if no GitHub API key is detected
    cat << EOF
{
  "permission": "allow"
}
EOF
fi
```

</CodeGroup>

## Configuration

Define hooks in a `hooks.json` file. Configuration can exist at multiple levels; higher-priority sources override lower ones:

```sh
~/.cursor/
├── hooks.json
└── hooks/
    ├── audit.sh
    ├── block-git.sh
    └── redact-secrets.sh
```

- Home Directory (User-specific):
  - `~/.cursor/hooks.json`
- Global (Enterprise-managed):
  - macOS: `/Library/Application Support/Cursor/hooks.json`
  - Linux/WSL: `/etc/cursor/hooks.json`
  - Windows: `C:\\ProgramData\\Cursor\\hooks.json`

The `hooks` object maps hook names to arrays of hook definitions. Each definition currently supports a `command` property that can be a shell string, an absolute path, or a path relative to the `hooks.json` file.

### Configuration file

```json
{
  "version": 1,
  "hooks": {
    "beforeShellExecution": [{ "command": "./script.sh" }],
    "afterFileEdit": [{ "command": "./format.sh" }]
  }
}
```

## Reference

### Common schema

#### Input (all hooks)

```json
{
  "conversation_id": "string",
  "generation_id": "string",
  "hook_event_name": "string",
  "workspace_roots": ["<path>"]
}
```

### Hook events

#### beforeShellExecution / beforeMCPExecution

Called before any shell command or MCP tool is executed. Return a permission decision.

```json
// beforeShellExecution input
{
  "command": "<full terminal command>",
  "cwd": "<current working directory>"
}

// beforeMCPExecution input
{
  "tool_name": "<tool name>",
  "tool_input": "<json params>"
}
// Plus either:
{ "url": "<server url>" }
// Or:
{ "command": "<command string>" }

// Output
{
  "permission": "allow" | "deny" | "ask",
  "userMessage": "<message shown in client>",
  "agentMessage": "<message sent to agent>"
}
```

#### afterFileEdit

Fires after a file is edited; useful for formatters or accounting of agent-written code.

```json
// Input
{
  "file_path": "<absolute path>",
  "edits": [{ "old_string": "<search>", "new_string": "<replace>" }]
}
```

#### beforeReadFile

Enable redaction or access control before the agent reads a file. Includes any prompt attachments for auditing rules inclusion.

```json
// Input
{
  "file_path": "<absolute path>",
  "content": "<file contents>",
  "attachments": [
    {
      "type": "rule",
      "file_path": "<absolute path>"
    }
  ]
}

// Output
{
  "permission": "allow" | "deny"
}
```

#### beforeSubmitPrompt

Called right after user hits send but before backend request. Can prevent submission.

```json
// Input
{
  "prompt": "<user prompt text>",
  "attachments": [
    {
      "type": "file" | "rule",
      "file_path": "<absolute path>"
    }
  ]
}

// Output
{
  "continue": true | false
}
```

#### stop

Called when the agent loop ends.

```json
// Input
{ "status": "completed" | "aborted" | "error" }
```

## Troubleshooting

**I'm on SSH, how do I use hooks?**

Remote SSH is not yet supported

**How to confirm hooks are active**

There is a Hooks tab in Cursor Settings to debug configured and executed hooks, as well as a Hooks output channel to see errors.

**If hooks are not working**

- Restart Cursor to ensure the hooks service is running.
- Ensure hook script paths are relative to `hooks.json` when using relative paths.
