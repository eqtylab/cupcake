#!/bin/bash

echo "🧹 Cleaning up Cupcake evaluation environment"
echo "============================================="

# Remove Claude Code settings
if [ -d ".claude" ]; then
    echo "Removing .claude/ directory..."
    rm -rf .claude
    echo "✅ Claude Code settings removed"
else
    echo "• .claude/ directory not found (already clean)"
fi

# Remove Cupcake project files
if [ -d ".cupcake" ]; then
    echo "Removing .cupcake/ directory..."
    rm -rf .cupcake
    echo "✅ Cupcake project files removed"
else
    echo "• .cupcake/ directory not found (already clean)"
fi

# Remove compiled bundle
if [ -f "bundle.tar.gz" ]; then
    echo "Removing bundle.tar.gz..."
    rm bundle.tar.gz
    echo "✅ Policy bundle removed"
else
    echo "• bundle.tar.gz not found (already clean)"
fi

echo ""
echo "🎉 Cleanup complete!"
echo ""
echo "Next steps:"
echo "1. Run './setup.sh' to reinitialize"
echo "2. Restart Claude Code"
echo "3. Test policy enforcement"