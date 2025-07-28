InjectContext Design Choice Documentation

Well-Documented Design: The use_stdout: true default is intentionally chosen and well-documented:

Design Philosophy:

- Stdout method: Simple, direct, low overhead - good for adoption
- JSON method: Structured, composable, extensible - good for sophistication

Evolution Strategy:

- Phase 1: Basic stdout context injection (current default)
- Phase 2: Dynamic context with templates (partially implemented)
- Phase 3: Intelligent context analysis (future)
