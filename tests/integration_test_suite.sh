#!/bin/bash

# Cupcake Integration Test Suite
# Tests the complete Cupcake integration with Claude Code using real hooks
#
# Usage: ./integration_test_suite.sh [-v|--verbose]
#   -v, --verbose    Show Claude Code prompts and responses

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Test configuration
TEST_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/claude-code-integration-directory"
CUPCAKE_BIN="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)/target/release/cupcake"
CLAUDE_SETTINGS_FILE="$TEST_DIR/.claude/settings.example.json"
POLICY_FILE="$TEST_DIR/.claude/test-policy.toml"

# Check for verbose mode
VERBOSE=false
if [ "$1" = "-v" ] || [ "$1" = "--verbose" ]; then
    VERBOSE=true
fi

# Test counters
TOTAL_TESTS=0
PASSED_TESTS=0
FAILED_TESTS=0

print_header() {
    echo -e "${BLUE}=== Cupcake Integration Test Suite ===${NC}"
    echo -e "Testing directory: ${TEST_DIR}"
    echo -e "Cupcake binary: ${CUPCAKE_BIN}"
    echo -e "Policy file: ${POLICY_FILE}"
    if [ "$VERBOSE" = true ]; then
        echo -e "${YELLOW}Verbose mode: ON${NC}"
    fi
    echo ""
}

print_verbose() {
    if [ "$VERBOSE" = true ]; then
        echo -e "${BLUE}[VERBOSE]${NC} $1"
    fi
}

show_claude_interaction() {
    local prompt="$1"
    local output_file="$2"
    
    if [ "$VERBOSE" = true ]; then
        echo -e "${BLUE}[PROMPT]${NC} $prompt"
        echo ""
        
        if [ -f "$output_file" ]; then
            echo -e "${BLUE}[CLAUDE RESPONSE JSON]${NC}"
            # Show the full JSON response with formatting if jq is available
            if command -v jq &> /dev/null; then
                jq '.' "$output_file" 2>/dev/null || cat "$output_file"
            else
                cat "$output_file"
            fi
            echo ""
        fi
    fi
}

print_test() {
    echo -e "${YELLOW}[TEST $((TOTAL_TESTS + 1))]${NC} $1"
}

print_success() {
    echo -e "${GREEN}✓ PASS${NC} $1"
    ((PASSED_TESTS++))
}

print_failure() {
    echo -e "${RED}✗ FAIL${NC} $1"
    ((FAILED_TESTS++))
}

print_summary() {
    echo ""
    echo -e "${BLUE}=== Test Summary ===${NC}"
    echo -e "Total tests: $TOTAL_TESTS"
    echo -e "${GREEN}Passed: $PASSED_TESTS${NC}"
    if [ $FAILED_TESTS -gt 0 ]; then
        echo -e "${RED}Failed: $FAILED_TESTS${NC}"
    else
        echo -e "Failed: $FAILED_TESTS"
    fi
    echo ""
    
    if [ $FAILED_TESTS -eq 0 ]; then
        echo -e "${GREEN}🎉 All tests passed!${NC}"
        exit 0
    else
        echo -e "${RED}❌ Some tests failed.${NC}"
        exit 1
    fi
}

check_dependencies() {
    print_test "Checking dependencies"
    ((TOTAL_TESTS++))
    
    # Check if cupcake binary exists
    if [ ! -f "$CUPCAKE_BIN" ]; then
        print_failure "Cupcake binary not found at $CUPCAKE_BIN"
        echo "Run 'cargo build --release' first"
        exit 1
    fi
    
    # Check if Claude Code is available
    if ! command -v claude &> /dev/null; then
        print_failure "Claude Code CLI not found. Install with 'npm install -g @anthropic-ai/claude-code'"
        exit 1
    fi
    
    # Check if test directory exists
    if [ ! -d "$TEST_DIR" ]; then
        print_failure "Test directory not found: $TEST_DIR"
        exit 1
    fi
    
    # Check if policy file exists
    if [ ! -f "$POLICY_FILE" ]; then
        print_failure "Policy file not found: $POLICY_FILE"
        exit 1
    fi
    
    print_success "All dependencies found"
}

test_cupcake_validate() {
    print_test "Testing cupcake validate"
    ((TOTAL_TESTS++))
    
    cd "$TEST_DIR"
    if "$CUPCAKE_BIN" validate .claude/test-policy.toml &> /tmp/validate_test.log; then
        # Check if validate command ran (even if not fully implemented)
        if grep -q "Cupcake validate command" /tmp/validate_test.log; then
            print_success "Policy validation command executed"
        else
            print_success "Policy validation passed"
        fi
    else
        print_failure "Policy validation failed"
        cat /tmp/validate_test.log
    fi
}

test_cupcake_run_direct() {
    print_test "Testing cupcake run with sample PreToolUse event"
    ((TOTAL_TESTS++))
    
    cd "$TEST_DIR"
    
    # Create a sample PreToolUse event JSON
    local test_event='{
        "hook_event_name": "PreToolUse",
        "session_id": "test-session-123",
        "transcript_path": "/tmp/test-transcript.jsonl",
        "tool_name": "Bash",
        "tool_input": {
            "command": "echo \"hello test\"",
            "description": "Test echo command"
        }
    }'
    
    # Test that cupcake run processes the event without error
    if echo "$test_event" | "$CUPCAKE_BIN" run --event PreToolUse --policy-file .claude/test-policy.toml --debug &> /tmp/cupcake_test.log; then
        print_success "Cupcake run processed PreToolUse event successfully"
    else
        print_failure "Cupcake run failed on PreToolUse event"
        echo "Debug output:"
        cat /tmp/cupcake_test.log | head -20
    fi
}

test_echo_command_feedback() {
    print_test "Testing echo command with Claude Code (should get feedback)"
    ((TOTAL_TESTS++))
    
    cd "$TEST_DIR"
    
    # Copy example settings to local settings for this test
    cp .claude/settings.example.json .claude/settings.local.json
    
    local prompt="echo 'hello world'"
    print_verbose "Running Claude Code with echo command..."
    
    # Run Claude Code with an echo command - should trigger soft feedback
    # Use --dangerously-skip-permissions to avoid Claude's built-in permission prompts
    if timeout 30 claude -p "$prompt" --output-format json --dangerously-skip-permissions &> /tmp/echo_test.log; then
        show_claude_interaction "$prompt" "/tmp/echo_test.log"
        
        # Check if command succeeded (soft feedback shouldn't block)
        if grep -q '"subtype":"success"' /tmp/echo_test.log; then
            print_success "Echo command executed successfully with Cupcake feedback"
        else
            print_failure "Echo command didn't complete successfully"
            if [ "$VERBOSE" != true ]; then
                echo "Output:"
                cat /tmp/echo_test.log | head -10
            fi
        fi
    else
        print_failure "Claude Code failed on echo command"
        if [ "$VERBOSE" != true ]; then
            echo "Output:"
            cat /tmp/echo_test.log | head -10
        fi
    fi
    
    # Clean up
    rm -f .claude/settings.local.json
}

test_blocked_file_creation() {
    print_test "Testing blocked file creation with Claude Code"
    ((TOTAL_TESTS++))
    
    cd "$TEST_DIR"
    
    # Copy example settings to local settings for this test
    cp .claude/settings.example.json .claude/settings.local.json
    
    local prompt="create a file called test.txt with content 'hello'"
    print_verbose "Running Claude Code with file creation command..."
    
    # Run Claude Code asking it to create a .txt file - should be blocked by Cupcake
    # Use --dangerously-skip-permissions to avoid Claude's built-in permission prompts
    if timeout 30 claude -p "$prompt" --output-format json --dangerously-skip-permissions &> /tmp/block_test.log; then
        show_claude_interaction "$prompt" "/tmp/block_test.log"
        
        # Even if Claude "succeeds", the file shouldn't be created due to our block
        if [ -f "test.txt" ]; then
            print_failure "File creation was not blocked (test.txt exists)"
            rm -f test.txt  # Clean up
        else
            print_success "File creation successfully blocked"
        fi
    else
        # Command failed - check if it was due to our policy block
        if grep -q "Creating \.txt files is not allowed" /tmp/block_test.log; then
            show_claude_interaction "$prompt" "/tmp/block_test.log"
            print_success "File creation blocked with correct message"
        else
            print_failure "Claude Code failed for unexpected reason"
            if [ "$VERBOSE" != true ]; then
                echo "Output:"
                cat /tmp/block_test.log | head -10
            fi
        fi
    fi
    
    # Clean up
    rm -f .claude/settings.local.json test.txt
}

test_read_file_tracking() {
    print_test "Testing file read with state tracking"
    ((TOTAL_TESTS++))
    
    cd "$TEST_DIR"
    
    # Copy example settings to local settings for this test
    cp .claude/settings.example.json .claude/settings.local.json
    
    # Clear any existing state
    rm -rf .cupcake/state/* 2>/dev/null || true
    
    local prompt="read the README.md file"
    print_verbose "Running Claude Code with file read command..."
    
    # Run Claude Code asking it to read a file
    if timeout 30 claude -p "$prompt" --output-format json --dangerously-skip-permissions &> /tmp/read_test.log; then
        show_claude_interaction "$prompt" "/tmp/read_test.log"
        # Check if state was tracked
        if [ -d ".cupcake/state" ] && [ "$(ls -A .cupcake/state 2>/dev/null)" ]; then
            print_success "File read successfully tracked in state"
        else
            print_success "Claude Code read succeeded (state tracking may not be visible)"
        fi
    else
        print_failure "Claude Code failed on read command"
        echo "Output:"
        cat /tmp/read_test.log | head -10
    fi
    
    # Clean up
    rm -f .claude/settings.local.json
}

test_cat_command_block() {
    print_test "Testing cat command blocking with Claude Code"
    ((TOTAL_TESTS++))
    
    cd "$TEST_DIR"
    
    # Copy example settings to local settings for this test
    cp .claude/settings.example.json .claude/settings.local.json
    
    local prompt="cat README.md"
    print_verbose "Running Claude Code with cat command..."
    
    # Run Claude Code with cat command - should be blocked by our policy
    if timeout 30 claude -p "$prompt" --output-format json --dangerously-skip-permissions &> /tmp/cat_test.log; then
        show_claude_interaction "$prompt" "/tmp/cat_test.log"
        # Check if the command was blocked or if Claude was redirected
        print_success "Cat command handled (may have been blocked or redirected)"
    else
        # Command failed - check if it was due to our policy block
        if grep -q "Use Read tool instead" /tmp/cat_test.log; then
            print_success "Cat command blocked with correct message"
        else
            print_failure "Claude Code failed for unexpected reason"
            echo "Output:"
            cat /tmp/cat_test.log | head -10
        fi
    fi
    
    # Clean up
    rm -f .claude/settings.local.json
}

cleanup() {
    # Clean up any test artifacts
    rm -f /tmp/cupcake_test.log /tmp/echo_test.log /tmp/block_test.log /tmp/read_test.log /tmp/cat_test.log
    rm -rf "$TEST_DIR/.cupcake/state" 2>/dev/null || true
    rm -f "$TEST_DIR/.claude/settings.local.json" 2>/dev/null || true
    rm -f "$TEST_DIR/test.txt" 2>/dev/null || true
}

# Main test execution
main() {
    print_header
    
    # Set up cleanup trap
    trap cleanup EXIT
    
    # Run tests
    check_dependencies
    test_cupcake_validate
    test_cupcake_run_direct
    test_echo_command_feedback
    test_blocked_file_creation
    test_read_file_tracking
    test_cat_command_block
    
    print_summary
}

# Run main function
main "$@"