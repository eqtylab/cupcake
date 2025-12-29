#!/bin/bash
# Signal: package_check
# Called when pip/uv attempts to install a package
# Input: JSON with command details via stdin
# Output: JSON with error field if blocked, empty object if allowed

# Read input from stdin
input=$(cat)

# Extract command from input (stub - adjust based on actual signal input format)
command=$(echo "$input" | jq -r '.command // ""')

# Stub: check for known bad packages
# In production, this would call a package safety API
if echo "$command" | grep -qE "(malicious-pkg|unsafe-package|cryptominer)"; then
    echo '{"error": "Package is on the blocklist"}'
    exit 0
fi

# No error - package is allowed
echo '{}'
