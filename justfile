# Cupcake development commands
# Run 'just --list' to see all available commands

# Default recipe - run the basic test suite
default: test

# Run the lean test suite (no TUI tests)
test:
    cargo test --all --locked

# Run the full test suite including TUI tests
test-tui:
    cargo test --all --locked --features tui

# Run clippy linting
lint:
    cargo clippy --all-targets --all-features -- -D warnings

# Run formatting check
fmt-check:
    cargo fmt --all -- --check

# Apply formatting
fmt:
    cargo fmt --all

# Run all checks (lint + format + test)
check: lint fmt-check test

# Run all checks including TUI tests
check-all: lint fmt-check test-tui

# Build the binary without TUI (lean)
build:
    cargo build --release

# Build the binary with TUI support
build-tui:
    cargo build --release --features tui

# Clean build artifacts
clean:
    cargo clean

# Install the binary locally (without TUI)
install:
    cargo install --path .

# Install the binary locally (with TUI)
install-tui:
    cargo install --path . --features tui

# Run a specific test file (e.g., just test-file hook_contract_integration_test)
test-file file:
    cargo test --test {{file}}

# Run a specific TUI test file (e.g., just test-tui-file tui_compilation_test)
test-tui-file file:
    cargo test --features tui --test {{file}}

# Show binary size comparison (lean vs TUI)
size-compare:
    @echo "Building lean binary..."
    @cargo build --release --quiet
    @echo "Lean binary size: $(ls -lh target/release/cupcake | awk '{print $5}')"
    @echo "Building TUI binary..."  
    @cargo build --release --features tui --quiet
    @echo "TUI binary size:  $(ls -lh target/release/cupcake | awk '{print $5}')"

# Run the cupcake binary with init command (requires TUI build)
demo-init: build-tui
    ./target/release/cupcake init --help

# Run the cupcake binary help
demo-help: build
    ./target/release/cupcake --help

# Development workflow - check everything
dev: clean check-all
    @echo "✅ All checks passed! Ready to commit."

# Run security audit on dependencies
audit:
    cargo audit

# Analyze unsafe code usage
geiger:
    cargo geiger


pkgbloat:
    cargo bloat --release --bin cupcake --crates

# Same binary but with the TUI feature enabled
tuibloat:
    cargo bloat --release --bin cupcake --features tui --crates