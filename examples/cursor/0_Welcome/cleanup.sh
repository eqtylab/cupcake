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

# Ask about global hooks cleanup
echo ""
echo "⚠️  Global Cursor hooks configuration detected at ~/.cursor/hooks.json"
read -p "Do you want to remove the global hooks configuration? (y/n) " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    HOOKS_FILE="$HOME/.cursor/hooks.json"

    # Check for backup
    LATEST_BACKUP=$(ls -t "$HOOKS_FILE.backup."* 2>/dev/null | head -n1)
    if [ -n "$LATEST_BACKUP" ]; then
        echo "Found backup: $LATEST_BACKUP"
        read -p "Restore from backup? (y/n) " -n 1 -r
        echo
        if [[ $REPLY =~ ^[Yy]$ ]]; then
            mv "$LATEST_BACKUP" "$HOOKS_FILE"
            echo "✅ Restored hooks.json from backup"
        else
            rm -f "$HOOKS_FILE"
            echo "✅ Removed hooks.json (backup preserved)"
        fi
    else
        rm -f "$HOOKS_FILE"
        echo "✅ Removed hooks.json"
    fi
else
    echo "ℹ️  Keeping global hooks configuration"
fi

echo ""
echo "🧹 Cleanup complete!"
echo ""
echo "Run ./setup.sh to reinitialize the evaluation environment."