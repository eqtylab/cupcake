# Backends

Watchdog uses a backend abstraction to communicate with LLMs. Currently, OpenRouter is the supported backend.

## OpenRouter

[OpenRouter](https://openrouter.ai) provides a unified API for hundreds of AI models. It handles routing, fallbacks, and billing across multiple providers.

### Setup

1. Create an account at [openrouter.ai](https://openrouter.ai)
2. Generate an API key
3. Set the environment variable:

```bash
export OPENROUTER_API_KEY="sk-or-..."
```

4. Enable Watchdog in `rulebook.yml`:

```yaml
watchdog: true
```

That's all you need. Watchdog uses `google/gemini-2.5-flash` by default.

### Model Selection

To use a different model, create `.cupcake/watchdog/config.json`:

```json
{
  "model": "anthropic/claude-3-haiku"
}
```

OpenRouter supports hundreds of models. See [OpenRouter's model list](https://openrouter.ai/models) for available options.

Popular choices for Watchdog:
- `google/gemini-2.5-flash` (default) - Fast, cost-effective
- `anthropic/claude-3-haiku` - Balanced speed and quality
- `openai/gpt-4o-mini` - Good reasoning capabilities

### Custom System Prompt

By default, Watchdog uses a security-focused system prompt. Override it by creating `.cupcake/watchdog/system.txt`:

```
You are a code review assistant. Evaluate tool calls for:
- Security concerns
- Best practice violations
- Project-specific rules

Respond with JSON: {"allow": bool, "confidence": 0-1, "reasoning": "...", "concerns": [], "suggestions": []}
```

### Custom User Template

The user message sent to the LLM can be customized via `.cupcake/watchdog/user.txt`:

```
Evaluate this tool call for our Python Django project:
{{event}}

Pay special attention to database operations and file access.
```

The `{{event}}` placeholder is replaced with the event JSON.

## Future Backends

The backend abstraction is designed for extensibility. Planned backends include:

- **Claude Code SDK**: Local evaluation using the Claude instance already running on your machine
- **Ollama**: Self-hosted local models for air-gapped environments

These are not yet implemented. If you need a specific backend, please open an issue on GitHub.
