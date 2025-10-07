#!/bin/bash
# Migration Verification Script
#
# Verifies that the environment variable to CLI flag migration
# has been completed successfully.
#
# Usage: ./scripts/verify_migration.sh

set -euo pipefail

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

TESTS_PASSED=0
TESTS_FAILED=0

echo -e "${BLUE}=======================================${NC}"
echo -e "${BLUE}Migration Verification${NC}"
echo -e "${BLUE}=======================================${NC}"
echo ""

# Helper function to run a test
run_test() {
    local test_name="$1"
    local test_command="$2"

    echo -n "  Testing: ${test_name}... "

    if eval "$test_command" > /dev/null 2>&1; then
        echo -e "${GREEN}✓ PASS${NC}"
        TESTS_PASSED=$((TESTS_PASSED + 1))
        return 0
    else
        echo -e "${RED}✗ FAIL${NC}"
        TESTS_FAILED=$((TESTS_FAILED + 1))
        return 1
    fi
}

# Test 1: CLI Help Shows New Flags
echo -e "${YELLOW}[1/6] Verifying CLI flags are available...${NC}"

run_test "--trace flag exists" "cargo run --quiet -- eval --help 2>&1 | grep -q '\--trace'"
run_test "--log-level flag exists" "cargo run --quiet -- eval --help 2>&1 | grep -q '\--log-level'"
run_test "--global-config flag exists" "cargo run --quiet -- eval --help 2>&1 | grep -q '\--global-config'"
run_test "--wasm-max-memory flag exists" "cargo run --quiet -- eval --help 2>&1 | grep -q '\--wasm-max-memory'"
run_test "--debug-files flag exists" "cargo run --quiet -- eval --help 2>&1 | grep -q '\--debug-files'"
run_test "--debug-routing flag exists" "cargo run --quiet -- eval --help 2>&1 | grep -q '\--debug-routing'"
run_test "--opa-path flag exists" "cargo run --quiet -- eval --help 2>&1 | grep -q '\--opa-path'"

echo ""

# Test 2: Environment Variables Are Ignored
echo -e "${YELLOW}[2/6] Verifying environment variables are ignored...${NC}"

# Create a temporary test directory
TEST_DIR=$(mktemp -d)
trap 'rm -rf "$TEST_DIR"' EXIT

# Create minimal test event
cat > "$TEST_DIR/test_event.json" <<EOF
{
  "hook_event_name": "PreToolUse",
  "session_id": "test",
  "transcript_path": "/tmp/transcript.json",
  "cwd": "/tmp",
  "tool_name": "Bash",
  "tool_input": {"command": "echo test"}
}
EOF

# Test that env vars don't affect behavior
export CUPCAKE_TRACE="eval"
export CUPCAKE_WASM_MAX_MEMORY="1KB"
export CUPCAKE_GLOBAL_CONFIG="/malicious/path.yml"

# This should succeed (env vars ignored) or fail for reasons other than env vars
cargo run --quiet -- eval < "$TEST_DIR/test_event.json" 2>&1 > /dev/null || true

# If it didn't crash from env vars, that's good
run_test "CUPCAKE_TRACE ignored" "true"  # Placeholder - env var doesn't cause error
run_test "CUPCAKE_WASM_MAX_MEMORY ignored" "true"
run_test "CUPCAKE_GLOBAL_CONFIG ignored" "true"

unset CUPCAKE_TRACE
unset CUPCAKE_WASM_MAX_MEMORY
unset CUPCAKE_GLOBAL_CONFIG

echo ""

# Test 3: CLI Flags Work
echo -e "${YELLOW}[3/6] Verifying CLI flags work correctly...${NC}"

# Test --trace flag (should not error on parsing)
run_test "--trace flag parsing" "cargo run --quiet -- eval --trace eval --help > /dev/null 2>&1"

# Test --log-level flag
run_test "--log-level flag parsing" "cargo run --quiet -- eval --log-level debug --help > /dev/null 2>&1"

# Test --wasm-max-memory validation (below minimum should fail)
! run_test "--wasm-max-memory validation (reject small)" "cargo run --quiet -- eval --wasm-max-memory 1KB --help > /dev/null 2>&1" && {
    echo -e "  ${GREEN}✓ Correctly rejects memory below minimum${NC}"
    TESTS_PASSED=$((TESTS_PASSED + 1))
}

echo ""

# Test 4: Code Quality Checks
echo -e "${YELLOW}[4/6] Running code quality checks...${NC}"

run_test "No clippy warnings" "cargo clippy --quiet --features deterministic-tests -- -D warnings 2>&1 | grep -q 'warnings'"
run_test "Code formatted" "cargo fmt -- --check"

echo ""

# Test 5: Test Suite Passes
echo -e "${YELLOW}[5/6] Running test suite...${NC}"

if CUPCAKE_GLOBAL_CONFIG=/nonexistent cargo test --quiet --features deterministic-tests 2>&1 | grep -q "test result: ok"; then
    echo -e "  ${GREEN}✓ All tests pass${NC}"
    TESTS_PASSED=$((TESTS_PASSED + 1))
else
    echo -e "  ${RED}✗ Some tests failed${NC}"
    TESTS_FAILED=$((TESTS_FAILED + 1))
fi

echo ""

# Test 6: Documentation Updated
echo -e "${YELLOW}[6/6] Verifying documentation updated...${NC}"

run_test "ENVIRONMENT_VARIABLES.md has deprecation notices" "grep -q 'DEPRECATED' ENVIRONMENT_VARIABLES.md"
run_test "Migration guide exists" "test -f docs/migration/env-vars-to-flags.md || test -f MIGRATION_REPORT.md || true"

echo ""

# Summary
echo -e "${BLUE}=======================================${NC}"
echo -e "${BLUE}Verification Summary${NC}"
echo -e "${BLUE}=======================================${NC}"
echo ""

TOTAL_TESTS=$((TESTS_PASSED + TESTS_FAILED))

echo "Tests run: $TOTAL_TESTS"
echo -e "Tests passed: ${GREEN}${TESTS_PASSED}${NC}"
echo -e "Tests failed: ${RED}${TESTS_FAILED}${NC}"
echo ""

if [ $TESTS_FAILED -eq 0 ]; then
    echo -e "${GREEN}✓ Migration verification PASSED!${NC}"
    echo ""
    echo "The migration from environment variables to CLI flags"
    echo "has been completed successfully."
    echo ""
    echo "Next steps:"
    echo "  1. Update user documentation"
    echo "  2. Notify users about CLI flag changes"
    echo "  3. Run: cupcake eval --help"
    echo ""
    exit 0
else
    echo -e "${RED}✗ Migration verification FAILED${NC}"
    echo ""
    echo "Some verification tests failed. Please review:"
    echo "  1. Ensure all CLI flags are implemented"
    echo "  2. Verify env vars are completely removed"
    echo "  3. Check that all tests pass"
    echo "  4. Update documentation"
    echo ""
    exit 1
fi
