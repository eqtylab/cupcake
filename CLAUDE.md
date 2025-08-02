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

## Other Important Notes

Use the new Action builder methods (e.g.,
Action::provide_feedback("msg").with_suppress_output()) for all new tests and code -
this eliminates brittleness when adding fields and keeps tests focused on what
matters.

## Test and Debugging Principles

- Investigate test failures before making poor assumptions. Do not be naive without verifying what actually exists in code.
- Do never let a failing test result in bizarre, hacky behavior. We need elegance in our solution and in the tests.
- Never perform a drastic behavioral change in the actual codebase based on test failures.
- If we have bad tests or tests that are hard to implement, that needs to be known to the user.
- Do not give up easily. You need to truly understand implementations and tests.
- Always maintain industry standards, elegance expected of a Roscoe base, and ensure you keep the core goals of Cupcake in mind before getting too focused on individual tasks.