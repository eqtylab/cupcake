You are Claude Code (claude.ai), a powerful AI coding assistant.
Your own software is modular and extensible, allowing users to build custom tools and integrations.
You are building the below software, Cupcake, which is a tool that integrates with Claude Code Hooks.

**Cupcake** is a tool that directly integrates with Claude Code Hooks.
It is critical that we stay aligned to that integration and Cloud Code's own implementation at all times.
Everything you do that is hook related you should be thinking "does this keep us very well aligned" to the utmost perfection. Does allow Cupcake to be amazing at what it's at and enhance the cloud code experience.

## Debug Logging

Cupcake automatically logs all hook executions to
`/tmp/cupcake-debug.log` for troubleshooting. Check this file when
debugging hook integration issues.

## Test Flakiness Fix

Integration tests using `cargo run` cause resource
contention → Use pre-built binary pattern with
get_cupcake_binary() to eliminate SIGKILL failures

## TUI Feature Flag

TUI is now optional (--features tui) - saves ~900KB
binary size, shows helpful recompile hints when
disabled, documented in README + help output

## Critical Commands

Use below commands for critical operations in cupcake codebase:

### Lean build (default, no TUI)

```bash
just test # fast test suite
just build # 4.3MB binary
just install # install lean version
```

### Full build (with TUI)

```bash
just test-tui # complete test suite
just build-tui # 5.2MB binary
just install-tui # install with interactive wizard
```

### Development

```bash
just check-all # lint + format + full tests
just dev # clean + check everything
```

`just --list` for full command list.

## Critical Integration: Claude Code Hooks

Remember the motto, "everything Cupcake does is directly intended to enhance a very strong integration with Cloud Code Hooks."

We keep documentation of cloud code within this repository, specifically within the `context` folder.

You can see Claude Code's own documentation for hooks at `context/claude-code-docs/hooks.md`.
