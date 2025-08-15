#!/bin/bash
# Simple signal: Get current git branch
git branch --show-current 2>/dev/null || echo "unknown"