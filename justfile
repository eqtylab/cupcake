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

# Build Python bindings (requires Python dev headers)
build-python:
    cd cupcake-py && maturin build --release

# ==================== ENVIRONMENT ====================

# Setup Python virtual environment with dependencies
venv:
    #!/usr/bin/env bash
    if [ ! -d ".venv" ]; then
        python3 -m venv .venv
        source .venv/bin/activate && pip install --upgrade pip
        source .venv/bin/activate && pip install maturin pytest pytest-asyncio
        echo "✅ Virtual environment created at .venv/"
    else
        echo "Virtual environment already exists"
    fi

# ==================== TEST COMMANDS ====================

# Run ALL tests (Rust + Python + TypeScript if available)
test-all: test test-python test-typescript

# Run Rust tests with deterministic-tests feature (REQUIRED)
# NOTE: Tests use EngineConfig to disable global config discovery, ensuring isolation
test *ARGS='':
    #!/usr/bin/env bash
    set -euo pipefail

    echo "Running Rust tests with deterministic-tests feature..."
    if cargo test --workspace --features cupcake-core/deterministic-tests {{ARGS}}; then
        echo "$(date '+%Y-%m-%d %H:%M:%S') | PASS | cargo test --workspace {{ARGS}}" >> test-results.log
        echo "✅ All Rust tests passed"
    else
        echo "$(date '+%Y-%m-%d %H:%M:%S') | FAIL | cargo test --workspace {{ARGS}}" >> test-results.log
        echo "❌ Some Rust tests failed"
        exit 1
    fi

# Run only unit tests (fast)
test-unit:
    cargo test --workspace --lib --features cupcake-core/deterministic-tests

# Run only integration tests
test-integration:
    cargo test --workspace --test '*' --features cupcake-core/deterministic-tests

# Run specific test by name
test-one TEST_NAME:
    cargo test --workspace --features cupcake-core/deterministic-tests {{TEST_NAME}}

# Run tests for core only
test-core:
    cargo test -p cupcake-core --features deterministic-tests

# Run tests for CLI only  
test-cli:
    cargo test -p cupcake-cli

# Run Python tests (auto-builds if needed)
test-python: venv
    #!/usr/bin/env bash
    set -euo pipefail

    echo "Running Python tests..."
    # Build Python module if needed
    if ! source .venv/bin/activate && python -c "import cupcake" 2>/dev/null; then
        echo "Building Python module first..."
        source .venv/bin/activate && cd cupcake-py && maturin develop
    fi

    # Run tests
    source .venv/bin/activate && python -m pytest cupcake-py/tests/ -v

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

# Develop Python bindings locally (uses venv)
develop-python: venv
    source .venv/bin/activate && cd cupcake-py && maturin develop

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
    rm -rf cupcake-py/target
    rm -rf cupcake-py/build
    rm -rf cupcake-py/*.egg-info
    rm -rf cupcake-py/dist
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
    cargo install cargo-watch cargo-edit maturin
    
    # Python tools
    pip install pytest pytest-asyncio black mypy ruff
    
    echo "✅ Development dependencies installed"

# Watch for changes and rebuild
watch:
    cargo watch -x "build --workspace"

# Watch and run tests on change
watch-test:
    cargo watch -x "test --workspace --features cupcake-core/deterministic-tests"

# ==================== ASSET GENERATION (VHS) ====================

# Check if VHS is installed
check-vhs:
    #!/usr/bin/env bash
    if ! command -v vhs &> /dev/null; then
        echo "❌ VHS not installed"
        echo ""
        echo "Install with:"
        echo "  brew install vhs        # macOS"
        echo "  scoop install vhs       # Windows"
        echo "  go install github.com/charmbracelet/vhs@latest"
        echo ""
        echo "VHS also requires ttyd and ffmpeg"
        exit 1
    fi
    echo "✅ VHS is available: $(vhs --version)"

# Generate all documentation assets from tape files
assets: check-vhs build-cli
    #!/usr/bin/env bash
    set -euo pipefail
    echo "Generating documentation assets..."
    echo ""
    
    # Ensure cupcake is in PATH
    export PATH="$PWD/target/release:$PATH"
    
    # VHS requires running from assets/ directory for paths to work
    cd assets
    
    # Find and process all tape files (excluding common/)
    for tape in tapes/cli/*.tape tapes/getting-started/*.tape; do
        if [[ -f "$tape" ]]; then
            echo "📼 Processing: $tape"
            vhs "$tape"
            echo ""
        fi
    done
    
    echo "✅ Assets generated in assets/output/"
    echo ""
    ls -la output/gifs/ 2>/dev/null || true
    ls -la output/screenshots/ 2>/dev/null || true

# Generate a specific tape file
asset TAPE: check-vhs build-cli
    #!/usr/bin/env bash
    set -euo pipefail
    export PATH="$PWD/target/release:$PATH"
    
    # Handle both with and without .tape extension
    tape_file="{{TAPE}}"
    if [[ ! "$tape_file" == *.tape ]]; then
        tape_file="${tape_file}.tape"
    fi
    
    # VHS requires running from assets/ directory for paths to work
    cd assets
    
    # Check common locations
    if [[ -f "tapes/cli/$tape_file" ]]; then
        vhs "tapes/cli/$tape_file"
    elif [[ -f "tapes/getting-started/$tape_file" ]]; then
        vhs "tapes/getting-started/$tape_file"
    elif [[ -f "$tape_file" ]]; then
        vhs "$tape_file"
    else
        echo "❌ Tape file not found: $tape_file"
        echo "Looked in:"
        echo "  - tapes/cli/$tape_file"
        echo "  - tapes/getting-started/$tape_file"
        echo "  - $tape_file"
        exit 1
    fi

# List available tape files
list-tapes:
    @echo "Available tape files:"
    @echo ""
    @find assets/tapes -name "*.tape" -not -path "*/common/*" | sort | while read f; do echo "  $(basename $f .tape)"; done

# Preview a tape file without generating output (validates syntax)
preview-tape TAPE: check-vhs
    #!/usr/bin/env bash
    tape_file="{{TAPE}}"
    if [[ ! "$tape_file" == *.tape ]]; then
        tape_file="${tape_file}.tape"
    fi
    
    cd assets
    
    if [[ -f "tapes/cli/$tape_file" ]]; then
        vhs "tapes/cli/$tape_file" --dry-run
    elif [[ -f "tapes/getting-started/$tape_file" ]]; then
        vhs "tapes/getting-started/$tape_file" --dry-run
    else
        echo "❌ Tape file not found: $tape_file"
        exit 1
    fi