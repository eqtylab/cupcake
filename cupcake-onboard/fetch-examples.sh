#!/bin/bash
# Fetch example repositories for testing cupcake-onboard

set -e

EXAMPLES_DIR="$(dirname "$0")/repo-examples"

mkdir -p "$EXAMPLES_DIR"

echo "Fetching shopsys/shopsys..."
if [ -d "$EXAMPLES_DIR/shopsys" ]; then
    echo "  Already exists, skipping (delete to re-fetch)"
else
    git clone --depth 1 git@github.com:shopsys/shopsys.git "$EXAMPLES_DIR/shopsys"
    rm -rf "$EXAMPLES_DIR/shopsys/.git"
    echo "  Done"
fi

echo ""
echo "Example repos ready in: $EXAMPLES_DIR"
echo ""
echo "Test with:"
echo "  node dist/cli.js --cwd=$EXAMPLES_DIR/shopsys"
