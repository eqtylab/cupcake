Read full/entire files to avoid confusion.

Aim to keep files under 500loc.

## Architecture Notes (Last Updated: 2025-08-04)

### Key Abstractions
- `AgentEvent` enum provides agent-agnostic interface (currently wraps ClaudeCodeEvent)
- `EngineRunner` creates contexts internally for single-source-of-truth
- `SanitizedEnvironment` filters env vars through hardcoded allow-list
- Response builders in `response/claude_code/` ensure 100% spec compliance

### Security Principles
- No shell execution by default (array mode uses direct process spawning)
- Template variables blocked in command paths
- Environment variables filtered (CLAUDE_PROJECT_DIR and CLAUDE_SESSION_ID preserved)

### Testing
- Tests organized by feature in `tests/features/`
- Use EventFactory for creating test events
- Contract tests verify Claude Code JSON compatibility
