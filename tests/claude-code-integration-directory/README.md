# Cupcake Integration Test Directory

This directory is used for testing Cupcake integration with Claude Code.

## Test Files

- `README.md` - This file (for testing Read tool feedback)
- `sample.json` - Sample JSON file 
- `example.py` - Sample Python file

## What to Test

1. `echo "hello"` - Should get feedback but work
2. `cat README.md` - Should be blocked
3. Ask Claude to read this README.md - Should get feedback
4. Ask Claude to create a .txt file - Should be blocked
