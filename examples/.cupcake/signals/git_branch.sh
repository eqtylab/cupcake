#!/bin/bash
# Simple signal: Get current git branch
# Output as JSON string to enable both string and structured access
branch=$(git branch --show-current 2>/dev/null || echo "unknown")
echo "\"$branch\""