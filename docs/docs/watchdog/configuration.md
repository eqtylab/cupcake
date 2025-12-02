# Configuration

Watchdog uses a self-contained directory structure for configuration, keeping your `rulebook.yml` clean.

## Directory Structure

```
.cupcake/
├── rulebook.yml              # Just: watchdog: true
└── watchdog/
    ├── config.json           # Backend, model, timeout settings
    ├── system.txt            # Custom system prompt (optional)
    └── user.txt              # User message template (optional)
```

## Quick Start

1. Enable Watchdog in `rulebook.yml`:

```yaml
watchdog: true
```

2. Set your API key:

```bash
export OPENROUTER_API_KEY="sk-or-..."
```

That's it. Watchdog uses sensible defaults for everything else.

## Configuration Files

### rulebook.yml

The rulebook **only** controls whether Watchdog is enabled or disabled. All other settings come from `.cupcake/watchdog/config.json`.

```yaml
# Enable Watchdog (uses defaults or config.json settings)
watchdog: true

# Or explicitly disable
watchdog: false
```

> **Note**: The `rulebook.yml` file only reads the `enabled` state. Any other settings (model, timeout, etc.) placed in `rulebook.yml` will be ignored. Use `.cupcake/watchdog/config.json` for all configuration.

### config.json

Create `.cupcake/watchdog/config.json` to customize settings:

```json
{
  "backend": "openrouter",
  "model": "google/gemini-2.5-flash",
  "timeout_seconds": 10,
  "on_error": "allow",
  "api_key_env": "OPENROUTER_API_KEY"
}
```

All fields are optional - unspecified fields use defaults.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `backend` | string | `"openrouter"` | LLM backend to use |
| `model` | string | `"google/gemini-2.5-flash"` | Model ID |
| `timeout_seconds` | integer | `10` | API call timeout |
| `on_error` | string | `"allow"` | `"allow"` (fail-open) or `"deny"` (fail-closed) |
| `api_key_env` | string | `"OPENROUTER_API_KEY"` | Environment variable for API key |

### system.txt

Custom system prompt for the LLM evaluator. If not provided, a built-in security-focused prompt is used.

```
You are a security reviewer for an AI coding agent...
```

### user.txt

Template for the user message sent to the LLM. Use `{{event}}` as a placeholder for the event JSON.

```
Evaluate this tool call:
{{event}}

Focus on security implications.
```

Default is just `{{event}}` (raw event JSON).

## Configuration Precedence

Watchdog loads configuration with this precedence:

1. **Project**: `.cupcake/watchdog/` (highest priority)
2. **Global**: Platform-specific config directory
3. **Defaults**: Built-in values

### Global Configuration Paths

The global configuration path varies by platform:

- **Linux**: `~/.config/cupcake/watchdog/`
- **macOS**: `~/Library/Application Support/cupcake/watchdog/`
- **Windows**: `%APPDATA%\cupcake\watchdog\`

This allows organization-wide defaults that projects can override.

## CLI Options

The `cupcake watchdog` command supports additional flags:

```bash
# Test configuration without making API calls
cupcake watchdog --dry-run < event.json

# Override the model for this run
cupcake watchdog --model "anthropic/claude-3-haiku" < event.json

# Read from file instead of stdin
cupcake watchdog --input event.json
```

## Examples

### Minimal Setup

```yaml
# .cupcake/rulebook.yml
watchdog: true
```

```bash
export OPENROUTER_API_KEY="sk-or-..."
```

### Custom Model and Timeout

```json
// .cupcake/watchdog/config.json
{
  "model": "anthropic/claude-3-haiku",
  "timeout_seconds": 15
}
```

### Fail-Closed (Strict Mode)

```json
// .cupcake/watchdog/config.json
{
  "on_error": "deny"
}
```

With `on_error: deny`, if the LLM is unavailable, Watchdog blocks the action rather than allowing it.

### Custom Prompts

```
// .cupcake/watchdog/system.txt
You are reviewing tool calls for a financial application.
Be extra cautious about:
- Database modifications
- File access outside /app
- Network requests to external services
```

### Organization-Wide Defaults

Create a global config at your platform's config directory (see [Global Configuration Paths](#global-configuration-paths)):

```json
// Linux: ~/.config/cupcake/watchdog/config.json
// macOS: ~/Library/Application Support/cupcake/watchdog/config.json
{
  "model": "google/gemini-2.5-flash",
  "on_error": "deny"
}
```

Projects inherit these settings unless they provide their own `.cupcake/watchdog/config.json`.

## Verifying Configuration

Use `--dry-run` to verify your configuration without making API calls:

```bash
echo '{"hook_event_name":"PreToolUse","tool_name":"Bash"}' | cupcake watchdog --dry-run
```

This logs the resolved configuration and what would be sent to the LLM.

## Disabling Watchdog

```yaml
# .cupcake/rulebook.yml
watchdog: false
```

Or simply remove the `watchdog` key entirely.
