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

# Remove Cursor hooks
if [ -d ".cursor" ]; then
    echo "Removing .cursor directory..."
    rm -rf .cursor
    echo "✅ .cursor directory removed"
fi

# Remove compiled bundle
if [ -f "bundle.tar.gz" ]; then
    echo "Removing compiled bundle..."
    rm -f bundle.tar.gz
    echo "✅ Bundle removed"
fi

echo ""
echo "🧹 Cleanup complete!"
echo ""
echo "Run ./setup.sh to reinitialize the evaluation environment."
