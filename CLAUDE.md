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
