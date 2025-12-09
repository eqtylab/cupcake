#!/bin/bash
set -e

echo "Cupcake Cursor Evaluation Cleanup"
echo "=================================="

# Remove Cupcake project files
if [ -d ".cupcake" ]; then
    echo "Removing .cupcake directory..."
    rm -rf .cupcake
    echo "✅ .cupcake directory removed"
fi

# Remove compiled bundle
if [ -f "bundle.tar.gz" ]; then
    echo "Removing compiled bundle..."
    rm -f bundle.tar.gz
    echo "✅ Bundle removed"
fi

# Remove test events
if [ -d "test-events" ]; then
    echo "Removing test-events directory..."
    rm -rf test-events
    echo "✅ Test events removed"
fi

# Remove project-level Cursor hooks
if [ -f ".cursor/hooks.json" ]; then
    echo "Removing project hooks..."
    rm -f .cursor/hooks.json
    echo "✅ Project hooks removed"
fi

# Remove .cursor directory if empty
if [ -d ".cursor" ] && [ -z "$(ls -A .cursor)" ]; then
    rmdir .cursor
    echo "✅ Empty .cursor directory removed"
fi

echo ""
echo "🧹 Cleanup complete!"
echo ""
echo "Run ./setup.sh to reinitialize the evaluation environment."
