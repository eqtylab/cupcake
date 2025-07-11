# Cupcake Integration Tests

This directory contains integration tests for Cupcake's Claude Code hooks integration.

## Test Structure

- `integration_test_suite.sh` - Comprehensive test suite for Cupcake functionality
- `claude-code-integration-directory/` - Test environment with policies and Claude Code configuration
- `run_command_integration_test.rs` - Rust unit tests for the run command

## Running Tests

### Prerequisites

1. **Build Cupcake**: 
   ```bash
   cargo build --release
   ```

2. **Install Claude Code CLI** (for full integration tests):
   ```bash
   npm install -g @anthropic-ai/claude-code
   ```

3. **Set API Key** (optional, for Claude Code integration test):
   ```bash
   export ANTHROPIC_API_KEY=your_key_here
   ```

### Running the Integration Test Suite

```bash
# Run all tests
./tests/integration_test_suite.sh

# The test will:
# - Validate policy files
# - Test cupcake run command directly
# - Test soft feedback policies (should not block)
# - Test hard block policies (should block with exit code 2)
# - Test state tracking
# - Test full Claude Code integration (if API key provided)
```

### Test Environment

The `claude-code-integration-directory/` contains:

- `.claude/test-policy.toml` - Test policies for various scenarios
- `.claude/settings.example.json` - Claude Code hooks configuration
- Sample files for testing (README.md, example.py, etc.)

### Test Policies

The test policies include:

1. **Soft Feedback**: Echo commands get informational feedback but are allowed
2. **Hard Blocks**: Creating .txt files is blocked
3. **PostToolUse Feedback**: Write operations get feedback after execution
4. **State Tracking**: File reads and tool usage are tracked

### Manual Testing

You can also test manually by:

1. **Copy settings file**:
   ```bash
   cd tests/claude-code-integration-directory
   cp .claude/settings.example.json .claude/settings.local.json
   ```

2. **Run Claude Code in the test directory**:
   ```bash
   claude -p "echo 'hello world'"
   claude -p "write a file called test.txt with content 'hello'"
   ```

3. **Clean up**:
   ```bash
   rm .claude/settings.local.json
   ```

## Expected Behavior

- **Echo commands**: Should work but show feedback in transcript
- **File creation**: Should be blocked with feedback to Claude
- **Read operations**: Should work and be tracked in `.cupcake/state/`

## Debugging

Use the `--debug` flag with cupcake for verbose output:

```bash
echo '{"hook_event_name":"PreToolUse",...}' | cupcake run --event PreToolUse --debug
```

Or run Claude Code with `--verbose` to see hook execution:

```bash
claude --verbose -p "your command"
```