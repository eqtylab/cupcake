#!/usr/bin/env bash
set -euo pipefail

# Verify latest Cupcake release with SLSA provenance
# Usage: ./verify-release.sh [version]
#   If version not provided, uses latest release

REPO="eqtylab/cupcake"
VERSION="${1:-}"

# Detect platform
detect_platform() {
    local os arch
    os="$(uname -s)"
    arch="$(uname -m)"

    case "$os" in
        Linux)
            case "$arch" in
                x86_64) echo "x86_64-unknown-linux-gnu" ;;
                aarch64|arm64) echo "aarch64-unknown-linux-gnu" ;;
                *) echo "Unsupported architecture: $arch" >&2; exit 1 ;;
            esac
            ;;
        Darwin)
            case "$arch" in
                x86_64) echo "x86_64-apple-darwin" ;;
                arm64) echo "aarch64-apple-darwin" ;;
                *) echo "Unsupported architecture: $arch" >&2; exit 1 ;;
            esac
            ;;
        MINGW*|MSYS*|CYGWIN*)
            echo "x86_64-pc-windows-msvc"
            ;;
        *)
            echo "Unsupported OS: $os" >&2
            exit 1
            ;;
    esac
}

# Check for required tools
check_requirements() {
    if ! command -v slsa-verifier &>/dev/null; then
        echo "Error: slsa-verifier not found" >&2
        echo "" >&2
        echo "Install with:" >&2
        echo "  macOS:  brew install slsa-verifier" >&2
        echo "  Linux:  See https://github.com/slsa-framework/slsa-verifier/releases" >&2
        exit 1
    fi

    if ! command -v jq &>/dev/null; then
        echo "Error: jq not found (needed to parse GitHub API)" >&2
        echo "" >&2
        echo "Install with:" >&2
        echo "  macOS:  brew install jq" >&2
        echo "  Linux:  apt install jq / yum install jq" >&2
        exit 1
    fi
}

# Get latest release version
get_latest_version() {
    echo "Fetching latest release..." >&2
    local latest
    latest="$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" | jq -r '.tag_name')"
    if [[ -z "$latest" || "$latest" == "null" ]]; then
        echo "Error: Could not fetch latest release" >&2
        exit 1
    fi
    echo "$latest"
}

# Main verification flow
main() {
    check_requirements

    # Get version
    if [[ -z "$VERSION" ]]; then
        VERSION="$(get_latest_version)"
        echo "Latest release: $VERSION"
    else
        echo "Verifying release: $VERSION"
    fi

    # Detect platform
    PLATFORM="$(detect_platform)"
    echo "Platform: $PLATFORM"

    # Determine file extension
    if [[ "$PLATFORM" == *"windows"* ]]; then
        EXT="zip"
    else
        EXT="tar.gz"
    fi

    ARTIFACT="cupcake-${VERSION}-${PLATFORM}.${EXT}"
    PROVENANCE="multiple.intoto.jsonl"

    # Create temp directory
    TEMP_DIR="$(mktemp -d)"
    trap 'rm -rf "$TEMP_DIR"' EXIT
    cd "$TEMP_DIR"

    echo ""
    echo "Downloading artifacts..."

    # Download artifact
    curl -fsSL -o "$ARTIFACT" \
        "https://github.com/${REPO}/releases/download/${VERSION}/${ARTIFACT}"
    echo "  ✓ $ARTIFACT ($(du -h "$ARTIFACT" | cut -f1))"

    # Download provenance
    curl -fsSL -o "$PROVENANCE" \
        "https://github.com/${REPO}/releases/download/${VERSION}/${PROVENANCE}"
    echo "  ✓ $PROVENANCE ($(du -h "$PROVENANCE" | cut -f1))"

    echo ""
    echo "Verifying with slsa-verifier..."
    echo ""

    # Run verification
    slsa-verifier verify-artifact \
        --provenance-path "$PROVENANCE" \
        --source-uri "github.com/${REPO}" \
        --source-tag "${VERSION}" \
        "$ARTIFACT"

    echo ""
    echo "Verification successful!"
    echo ""
    echo "Artifact location: ${TEMP_DIR}/${ARTIFACT}"
    echo "Press Enter to clean up temp files, or Ctrl+C to keep them..."
    read -r
}

main "$@"
