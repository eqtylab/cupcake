#!/bin/bash

# Cupcake TUI Testing Script
# 
# This script provides a fast way to test the TUI initialization wizard
# by cleaning up any previous state and launching the TUI in a clean environment.

set -e  # Exit on any error

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${BLUE}🧁 Cupcake TUI Testing Script${NC}"
echo "================================="

# Build the project first to ensure we have the latest binary
echo -e "${YELLOW}Building cupcake binary...${NC}"
cd .. && cargo build --quiet
if [ $? -ne 0 ]; then
    echo -e "${RED}❌ Build failed! Please fix compilation errors.${NC}"
    exit 1
fi

# Change to the manual test directory
echo -e "${YELLOW}Changing to tests/manual-test directory...${NC}"
cd tests/manual-test

# Clean up any existing generated files
echo -e "${YELLOW}Cleaning up previous test artifacts...${NC}"

# Remove guardrails directory if it exists
if [ -d "guardrails" ]; then
    echo "  - Removing guardrails/ directory"
    rm -rf guardrails
fi

# Remove .cupcake state directory if it exists
if [ -d ".cupcake" ]; then
    echo "  - Removing .cupcake/ state directory"
    rm -rf .cupcake
fi

# Reset Claude settings to default state
echo "  - Resetting Claude Code settings"
cat > .claude/settings.local.json << 'EOF'
{
  "memory": {
    "file_paths": ["CLAUDE.md"],
    "context_directory": "../../context"
  },
  "hooks": {
    "user_prompt_submit": {
      "enabled": false
    },
    "pre_tool_use": {
      "enabled": false
    },
    "post_tool_use": {
      "enabled": false
    }
  },
  "tools": {
    "disabled": []
  }
}
EOF

echo -e "${GREEN}✅ Cleanup complete!${NC}"
echo ""
echo -e "${BLUE}📁 Test Environment Status:${NC}"
echo "  - Working directory: $(pwd)"
echo "  - CLAUDE.md exists: $([ -f "CLAUDE.md" ] && echo "✅" || echo "❌")"
echo "  - .claude/settings.local.json reset: ✅"
echo "  - guardrails/ removed: ✅"
echo "  - .cupcake/ state cleared: ✅"
echo ""

# Show what files will be discovered
echo -e "${BLUE}📋 Files that should be discovered:${NC}"
find . -name "CLAUDE.md" -o -name "*.md" | grep -E "(CLAUDE|rules)" | head -5
echo ""

echo -e "${GREEN}🚀 Launching Cupcake TUI...${NC}"
echo "   Use Ctrl+C to exit the TUI and return to this script"
echo ""

# Launch the TUI
../../target/debug/cupcake init

# After TUI exits, show what was generated
echo ""
echo -e "${BLUE}📊 Generated Files Summary:${NC}"
echo "================================="

if [ -d "guardrails" ]; then
    echo -e "${GREEN}✅ guardrails/ directory created${NC}"
    if [ -f "guardrails/cupcake.yaml" ]; then
        echo "  - cupcake.yaml: ✅"
    else
        echo "  - cupcake.yaml: ❌"
    fi
    if [ -d "guardrails/policies" ]; then
        echo "  - policies/ directory: ✅"
        echo "  - Policy files: $(find guardrails/policies -name "*.yaml" | wc -l)"
    else
        echo "  - policies/ directory: ❌"
    fi
else
    echo -e "${RED}❌ guardrails/ directory not created${NC}"
fi

if [ -d ".cupcake" ]; then
    echo -e "${GREEN}✅ .cupcake/ state directory created${NC}"
    if [ -d ".cupcake/state" ]; then
        echo "  - state files: $(find .cupcake/state -name "*.json" | wc -l)"
    fi
else
    echo -e "${RED}❌ .cupcake/ state directory not created${NC}"
fi

# Check if Claude settings were updated
if grep -q '"enabled": true' .claude/settings.local.json 2>/dev/null; then
    echo -e "${GREEN}✅ Claude Code hooks enabled${NC}"
else
    echo -e "${YELLOW}ℹ️  Claude Code hooks not enabled (this is expected for stub implementation)${NC}"
fi

echo ""
echo -e "${BLUE}🔄 To test again, simply run: cd .. && ./test-tui.sh${NC}"
echo -e "${BLUE}💡 Tip: You can also run 'cupcake inspect' to view loaded policies${NC}"