# Cupcake Development Commands

# Run tests with timestamped logging
test *ARGS='':
    #!/usr/bin/env bash
    set -euo pipefail
    
    # Run cargo test and capture outcome
    if cargo test {{ARGS}}; then
        echo "$(date '+%Y-%m-%d %H:%M:%S') | PASS | cargo test {{ARGS}}" >> test-results.log
    else
        echo "$(date '+%Y-%m-%d %H:%M:%S') | FAIL | cargo test {{ARGS}}" >> test-results.log
        exit 1
    fi

# View recent test results
test-log:
    tail -n 50 test-results.log

# Clear test log
test-clear:
    > test-results.log
    echo "Test log cleared"