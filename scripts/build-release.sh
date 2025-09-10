#!/usr/bin/env bash
# Build release artifacts for Cupcake
# This script is primarily used for local testing of release builds

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Get the directory of this script
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

# Parse arguments
TARGET="${1:-}"
VERSION="${2:-dev}"

if [[ -z "$TARGET" ]]; then
    echo -e "${RED}Error: Target not specified${NC}"
    echo "Usage: $0 <target> [version]"
    echo ""
    echo "Available targets:"
    echo "  x86_64-apple-darwin       - macOS Intel"
    echo "  aarch64-apple-darwin      - macOS Apple Silicon"
    echo "  x86_64-unknown-linux-gnu  - Linux x64 (glibc)"
    echo "  x86_64-unknown-linux-musl - Linux x64 (musl)"
    echo "  aarch64-unknown-linux-gnu - Linux ARM64"
    echo "  x86_64-pc-windows-msvc   - Windows x64"
    exit 1
fi

echo -e "${GREEN}Building Cupcake ${VERSION} for ${TARGET}${NC}"

# Change to project root
cd "$PROJECT_ROOT"

# Install target if needed
echo "Adding Rust target ${TARGET}..."
rustup target add "$TARGET" 2>/dev/null || true

# Determine if we need to use cross
USE_CROSS=false
case "$TARGET" in
    *-musl|aarch64-unknown-linux-gnu)
        USE_CROSS=true
        ;;
esac

# Build the binary
if [[ "$USE_CROSS" == "true" ]]; then
    echo -e "${YELLOW}Using cross for compilation${NC}"
    if ! command -v cross &> /dev/null; then
        echo "Installing cross..."
        cargo install cross --git https://github.com/cross-rs/cross
    fi
    cross build --profile dist --target "$TARGET" --bin cupcake
else
    echo "Using cargo for native compilation"
    cargo build --profile dist --target "$TARGET" --bin cupcake
fi

# Determine binary name and archive format
BINARY_NAME="cupcake"
ARCHIVE_EXT="tar.gz"
if [[ "$TARGET" == *"windows"* ]]; then
    BINARY_NAME="cupcake.exe"
    ARCHIVE_EXT="zip"
fi

# Create release directory structure
RELEASE_NAME="cupcake-v${VERSION}-${TARGET}"
RELEASE_DIR="target/release-artifacts/${RELEASE_NAME}"
mkdir -p "${RELEASE_DIR}/bin"

# Copy binary
echo "Copying binary..."
cp "target/${TARGET}/dist/${BINARY_NAME}" "${RELEASE_DIR}/bin/"

# Make binary executable (Unix only)
if [[ "$TARGET" != *"windows"* ]]; then
    chmod +x "${RELEASE_DIR}/bin/cupcake"
fi

# Download and bundle OPA
echo "Downloading OPA v1.7.1 for ${TARGET}..."
OPA_VERSION="v1.7.1"
OPA_BINARY=""
case "$TARGET" in
    x86_64-apple-darwin)
        OPA_BINARY="opa_darwin_amd64"
        ;;
    aarch64-apple-darwin)
        OPA_BINARY="opa_darwin_arm64_static"
        ;;
    x86_64-unknown-linux-gnu)
        OPA_BINARY="opa_linux_amd64_static"
        ;;
    x86_64-unknown-linux-musl)
        OPA_BINARY="opa_linux_amd64_static"
        ;;
    aarch64-unknown-linux-gnu)
        OPA_BINARY="opa_linux_arm64_static"
        ;;
    x86_64-pc-windows-msvc)
        OPA_BINARY="opa_windows_amd64.exe"
        ;;
    *)
        echo -e "${YELLOW}Warning: Unknown target for OPA download: $TARGET${NC}"
        ;;
esac

if [[ -n "$OPA_BINARY" ]]; then
    OPA_URL="https://github.com/open-policy-agent/opa/releases/download/${OPA_VERSION}/${OPA_BINARY}"
    OPA_CHECKSUM_URL="${OPA_URL}.sha256"
    
    # Download OPA
    echo "Downloading from ${OPA_URL}..."
    if [[ "$TARGET" == *"windows"* ]]; then
        curl -L -o "${RELEASE_DIR}/bin/opa.exe" "$OPA_URL" || {
            echo -e "${YELLOW}Warning: Failed to download OPA${NC}"
        }
        curl -L -o "${RELEASE_DIR}/bin/opa.exe.sha256" "$OPA_CHECKSUM_URL" 2>/dev/null || true
        
        # Verify checksum if downloaded
        if [[ -f "${RELEASE_DIR}/bin/opa.exe.sha256" ]]; then
            cd "${RELEASE_DIR}/bin"
            if [[ "$(uname)" == "Darwin" ]]; then
                echo "$(cat opa.exe.sha256)  opa.exe" | shasum -a 256 -c || {
                    echo -e "${RED}OPA checksum verification failed${NC}"
                    rm -f opa.exe opa.exe.sha256
                }
            else
                echo "$(cat opa.exe.sha256)  opa.exe" | sha256sum -c || {
                    echo -e "${RED}OPA checksum verification failed${NC}"
                    rm -f opa.exe opa.exe.sha256
                }
            fi
            rm -f opa.exe.sha256
            cd - > /dev/null
        fi
    else
        curl -L -o "${RELEASE_DIR}/bin/opa" "$OPA_URL" || {
            echo -e "${YELLOW}Warning: Failed to download OPA${NC}"
        }
        curl -L -o "${RELEASE_DIR}/bin/opa.sha256" "$OPA_CHECKSUM_URL" 2>/dev/null || true
        
        # Verify checksum if downloaded
        if [[ -f "${RELEASE_DIR}/bin/opa.sha256" ]]; then
            cd "${RELEASE_DIR}/bin"
            if [[ "$(uname)" == "Darwin" ]]; then
                echo "$(cat opa.sha256)  opa" | shasum -a 256 -c || {
                    echo -e "${RED}OPA checksum verification failed${NC}"
                    rm -f opa opa.sha256
                }
            else
                echo "$(cat opa.sha256)  opa" | sha256sum -c || {
                    echo -e "${RED}OPA checksum verification failed${NC}"
                    rm -f opa opa.sha256
                }
            fi
            rm -f opa.sha256
            cd - > /dev/null
            chmod +x "${RELEASE_DIR}/bin/opa" 2>/dev/null || true
        fi
    fi
    
    if [[ -f "${RELEASE_DIR}/bin/opa" ]] || [[ -f "${RELEASE_DIR}/bin/opa.exe" ]]; then
        echo -e "${GREEN}✓ OPA bundled successfully${NC}"
    fi
fi

# Copy documentation
echo "Copying documentation..."
cp README.md LICENSE "${RELEASE_DIR}/"

# Create archive
echo "Creating archive..."
cd "target/release-artifacts"

if [[ "$ARCHIVE_EXT" == "tar.gz" ]]; then
    tar czf "${RELEASE_NAME}.tar.gz" "${RELEASE_NAME}"
else
    # For Windows, we need zip
    if command -v zip &> /dev/null; then
        zip -r "${RELEASE_NAME}.zip" "${RELEASE_NAME}"
    else
        echo -e "${YELLOW}Warning: zip command not found, skipping archive creation${NC}"
    fi
fi

# Generate checksum
if [[ -f "${RELEASE_NAME}.${ARCHIVE_EXT}" ]]; then
    echo "Generating checksum..."
    if [[ "$(uname)" == "Darwin" ]]; then
        shasum -a 256 "${RELEASE_NAME}.${ARCHIVE_EXT}" > "${RELEASE_NAME}.${ARCHIVE_EXT}.sha256"
    else
        sha256sum "${RELEASE_NAME}.${ARCHIVE_EXT}" > "${RELEASE_NAME}.${ARCHIVE_EXT}.sha256"
    fi
    
    echo -e "${GREEN}Build complete!${NC}"
    echo "Archive: target/release-artifacts/${RELEASE_NAME}.${ARCHIVE_EXT}"
    echo "Checksum: target/release-artifacts/${RELEASE_NAME}.${ARCHIVE_EXT}.sha256"
else
    echo -e "${RED}Archive creation failed${NC}"
    exit 1
fi