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

## Code Freeze Status

**Status: CODE FREEZE** 

To activate code freeze, add the all-caps phrase that indicates a freeze anywhere in this README file.

## How Code Freeze Works

When the freeze phrase appears anywhere in this README file, the code freeze policy will:
- Block all Write and Edit operations
- Display a clear message about the active freeze  
- Require removing the freeze phrase from this file to resume normal operations

This provides a simple way to temporarily disable file modifications across the entire project.