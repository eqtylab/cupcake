# Cupcake Development Commands - Workspace Edition

# Default recipe - show available commands
default:
    @just --list

# ==================== BUILD COMMANDS ====================

# Build the entire workspace in release mode
build:
    cargo build --workspace --release

# Build debug mode for faster compilation during development
build-debug:
    cargo build --workspace

# Build only the core library
build-core:
    cargo build -p cupcake-core --release

# Build only the CLI
build-cli:
    cargo build -p cupcake-cli --release

# Install cupcake binary to cargo bin directory
install: build-cli
    cp target/release/cupcake ~/.cargo/bin/cupcake
    @echo "✅ Installed cupcake to ~/.cargo/bin/"

# ==================== TEST COMMANDS ====================

# Run ALL tests (Rust + TypeScript)
test-all: test test-typescript

# Run Rust tests
# NOTE: Tests use EngineConfig to disable global config discovery, ensuring isolation
test *ARGS='':
    #!/usr/bin/env bash
    set -euo pipefail

    echo "Running Rust tests..."
    if cargo test --workspace {{ARGS}}; then
        echo "$(date '+%Y-%m-%d %H:%M:%S') | PASS | cargo test --workspace {{ARGS}}" >> test-results.log
        echo "✅ All Rust tests passed"
    else
        echo "$(date '+%Y-%m-%d %H:%M:%S') | FAIL | cargo test --workspace {{ARGS}}" >> test-results.log
        echo "❌ Some Rust tests failed"
        exit 1
    fi

# Run only unit tests (fast)
test-unit:
    cargo test --workspace --lib

# Run only integration tests
test-integration:
    cargo test --workspace --test '*'

# Run specific test by name
test-one TEST_NAME:
    cargo test --workspace {{TEST_NAME}}

# Run tests for core only
test-core:
    cargo test -p cupcake-core

# Run tests for CLI only  
test-cli:
    cargo test -p cupcake-cli

# Run TypeScript tests (auto-builds if needed)
test-typescript:
    #!/usr/bin/env bash
    set -euo pipefail

    echo "Running TypeScript tests..."
    cd cupcake-ts

    # Install dependencies if node_modules doesn't exist
    if [ ! -d "node_modules" ]; then
        echo "Installing dependencies..."
        npm install
    fi

    # Build native module if .node file doesn't exist
    if ! ls index.node 2>/dev/null; then
        echo "Building native module..."
        npm run build
    fi

    # Run tests
    npm test

# Run benchmarks
bench:
    cargo bench -p cupcake-core

# ==================== DEVELOPMENT COMMANDS ====================

# Check code without building
check:
    cargo check --workspace

# Format all code
fmt:
    cargo fmt --all

# Run clippy linter
lint:
    cargo clippy --workspace --all-targets

# Fix common issues automatically
fix:
    cargo fix --workspace --allow-dirty
    cargo fmt --all

# ==================== PERFORMANCE TESTING ====================

# Run performance validation tests
perf-test: build
    cargo bench -p cupcake-core --bench engine_benchmark

# Memory leak test with valgrind (Linux/macOS)
test-memory:
    #!/usr/bin/env bash
    if command -v valgrind &> /dev/null; then
        echo "Running memory leak detection..."
        cargo build --workspace
        valgrind --leak-check=full --show-leak-kinds=all \
            target/debug/cupcake eval < examples/events/mcp_filesystem_read.json
    else
        echo "⚠️  valgrind not found - install it for memory testing"
    fi

# ==================== CLEAN COMMANDS ====================

# Clean all build artifacts
clean:
    cargo clean
    rm -rf **/__pycache__

# Clean and rebuild everything
rebuild: clean build

# ==================== UTILITY COMMANDS ====================

# View recent test results
test-log:
    tail -n 50 test-results.log

# Clear test log
test-clear:
    > test-results.log
    echo "Test log cleared"

# Show project statistics
stats:
    @echo "📊 Cupcake Project Statistics"
    @echo "=============================="
    @echo "Rust files: $(find . -name '*.rs' -not -path './target/*' | wc -l)"
    @echo "Test files: $(find . -name '*test*.rs' -not -path './target/*' | wc -l)"  
    @echo "Policy files: $(find . -name '*.rego' | wc -l)"
    @echo "Lines of Rust: $(find . -name '*.rs' -not -path './target/*' | xargs wc -l | tail -1)"

# Run the CLI with example input
run-example:
    echo '{"hookEventName": "PreToolUse", "tool_name": "Bash", "command": "echo test"}' | \
    cargo run -p cupcake-cli -- eval --policy-dir examples/policies

# Install development dependencies
install-dev:
    #!/usr/bin/env bash
    echo "Installing development dependencies..."

    # Rust tools
    rustup component add rustfmt clippy
    cargo install cargo-watch cargo-edit

    echo "✅ Development dependencies installed"

# Watch for changes and rebuild
watch:
    cargo watch -x "build --workspace"

# Watch and run tests on change
watch-test:
    cargo watch -x "test --workspace"

# ==================== URL CHECKING ====================

# Check URLs in markdown and HTML files (use --replacements '{"from": "to"}' for staging)
check-urls *ARGS='':
    SKIP_DOMAINS="registry.mycompany.com,www.npmjs.com" \
    SKIP_URLS="https://github.com/myorg/my-rulebook" \
    python3 scripts/check_urls.py . {{ARGS}}