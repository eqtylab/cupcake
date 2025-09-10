#!/usr/bin/env bash
# Cupcake Installation Script
# 
# This script downloads and installs the Cupcake CLI tool.
# It automatically detects your OS and architecture, downloads the appropriate
# binary from GitHub releases, verifies checksums, and installs to your PATH.
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/eqtylab/cupcake/main/cupcake-rewrite/scripts/install.sh | sh
#   wget -qO- https://raw.githubusercontent.com/eqtylab/cupcake/main/cupcake-rewrite/scripts/install.sh | sh

set -e

# Configuration
GITHUB_REPO="${CUPCAKE_REPO:-eqtylab/cupcake}"
INSTALL_DIR="${CUPCAKE_INSTALL_DIR:-$HOME/.cupcake}"
BIN_DIR="$INSTALL_DIR/bin"
BINARY_NAME="cupcake"

# Colors for output (only if terminal supports it)
if [[ -t 1 ]]; then
    RED='\033[0;31m'
    GREEN='\033[0;32m'
    YELLOW='\033[1;33m'
    BLUE='\033[0;34m'
    NC='\033[0m' # No Color
else
    RED=''
    GREEN=''
    YELLOW=''
    BLUE=''
    NC=''
fi

# Helper functions
error() {
    echo -e "${RED}Error: $1${NC}" >&2
    exit 1
}

warning() {
    echo -e "${YELLOW}Warning: $1${NC}" >&2
}

info() {
    echo -e "${BLUE}$1${NC}"
}

success() {
    echo -e "${GREEN}$1${NC}"
}

# Detect OS and architecture
detect_platform() {
    local os
    local arch
    
    # Detect OS
    case "$(uname -s)" in
        Linux*)
            os="unknown-linux"
            # Check if musl or glibc
            if ldd --version 2>&1 | grep -q musl; then
                os="unknown-linux-musl"
            else
                os="unknown-linux-gnu"
            fi
            ;;
        Darwin*)
            os="apple-darwin"
            ;;
        CYGWIN*|MINGW*|MSYS*)
            error "Please use the PowerShell installer for Windows: irm https://raw.githubusercontent.com/${GITHUB_REPO}/main/cupcake-rewrite/scripts/install.ps1 | iex"
            ;;
        *)
            error "Unsupported operating system: $(uname -s)"
            ;;
    esac
    
    # Detect architecture
    case "$(uname -m)" in
        x86_64|amd64)
            arch="x86_64"
            ;;
        aarch64|arm64)
            arch="aarch64"
            ;;
        armv7l|armhf)
            # ARM 32-bit not supported yet
            error "ARM 32-bit architecture is not supported yet"
            ;;
        *)
            error "Unsupported architecture: $(uname -m)"
            ;;
    esac
    
    echo "${arch}-${os}"
}

# Get the latest release version from GitHub
get_latest_version() {
    local version
    version=$(curl -sL "https://api.github.com/repos/${GITHUB_REPO}/releases/latest" | \
              grep '"tag_name":' | \
              sed -E 's/.*"([^"]+)".*/\1/')
    
    if [[ -z "$version" ]]; then
        error "Failed to fetch latest version from GitHub"
    fi
    
    echo "$version"
}

# Download file with progress
download_file() {
    local url="$1"
    local output="$2"
    
    if command -v curl &> /dev/null; then
        curl -fSL --progress-bar "$url" -o "$output"
    elif command -v wget &> /dev/null; then
        wget --show-progress -qO "$output" "$url"
    else
        error "Neither curl nor wget found. Please install one of them."
    fi
}

# Verify checksum
verify_checksum() {
    local file="$1"
    local checksum_file="$2"
    
    if command -v sha256sum &> /dev/null; then
        sha256sum -c "$checksum_file" 2>/dev/null | grep -q OK
    elif command -v shasum &> /dev/null; then
        shasum -a 256 -c "$checksum_file" 2>/dev/null | grep -q OK
    else
        warning "Cannot verify checksum: sha256sum or shasum not found"
        return 0
    fi
}

# Note: OPA is now bundled with Cupcake
check_bundled_opa() {
    if [[ -f "$BIN_DIR/opa" ]]; then
        success "✓ OPA is bundled with Cupcake"
    else
        warning "OPA binary not found in bundle (this should not happen)"
    fi
}

# Main installation
main() {
    info "Installing Cupcake CLI..."
    echo ""
    
    # Detect platform
    PLATFORM=$(detect_platform)
    info "Detected platform: $PLATFORM"
    
    # Get version
    VERSION="${CUPCAKE_VERSION:-$(get_latest_version)}"
    info "Installing version: $VERSION"
    
    # Create temporary directory
    TMP_DIR=$(mktemp -d)
    trap "rm -rf $TMP_DIR" EXIT
    
    # Construct download URLs
    ARCHIVE_NAME="cupcake-${VERSION}-${PLATFORM}.tar.gz"
    DOWNLOAD_URL="https://github.com/${GITHUB_REPO}/releases/download/${VERSION}/${ARCHIVE_NAME}"
    CHECKSUM_URL="${DOWNLOAD_URL}.sha256"
    
    # Download archive
    info "Downloading Cupcake..."
    download_file "$DOWNLOAD_URL" "$TMP_DIR/$ARCHIVE_NAME"
    
    # Download and verify checksum
    info "Verifying checksum..."
    download_file "$CHECKSUM_URL" "$TMP_DIR/${ARCHIVE_NAME}.sha256"
    
    cd "$TMP_DIR"
    if ! verify_checksum "$ARCHIVE_NAME" "${ARCHIVE_NAME}.sha256"; then
        error "Checksum verification failed!"
    fi
    success "✓ Checksum verified"
    
    # Extract archive
    info "Extracting archive..."
    tar xzf "$ARCHIVE_NAME"
    
    # Create installation directory
    mkdir -p "$BIN_DIR"
    
    # Install binaries
    info "Installing to $BIN_DIR..."
    EXTRACTED_DIR="${ARCHIVE_NAME%.tar.gz}"
    cp "$EXTRACTED_DIR/bin/$BINARY_NAME" "$BIN_DIR/"
    chmod +x "$BIN_DIR/$BINARY_NAME"
    
    # Copy bundled OPA
    if [[ -f "$EXTRACTED_DIR/bin/opa" ]]; then
        cp "$EXTRACTED_DIR/bin/opa" "$BIN_DIR/"
        chmod +x "$BIN_DIR/opa"
    fi
    
    # Check if bin directory is in PATH
    if [[ ":$PATH:" != *":$BIN_DIR:"* ]]; then
        warning "$BIN_DIR is not in your PATH"
        echo ""
        echo "Add Cupcake to your PATH by adding this to your shell profile:"
        echo ""
        
        # Detect shell and provide appropriate instructions
        if [[ -n "$ZSH_VERSION" ]]; then
            echo "  echo 'export PATH=\"$BIN_DIR:\$PATH\"' >> ~/.zshrc"
            echo "  source ~/.zshrc"
        elif [[ -n "$BASH_VERSION" ]]; then
            if [[ -f "$HOME/.bash_profile" ]]; then
                echo "  echo 'export PATH=\"$BIN_DIR:\$PATH\"' >> ~/.bash_profile"
                echo "  source ~/.bash_profile"
            else
                echo "  echo 'export PATH=\"$BIN_DIR:\$PATH\"' >> ~/.bashrc"
                echo "  source ~/.bashrc"
            fi
        else
            echo "  export PATH=\"$BIN_DIR:\$PATH\""
        fi
        echo ""
    fi
    
    # Verify installation
    if "$BIN_DIR/$BINARY_NAME" version &> /dev/null; then
        success "✓ Cupcake installed successfully!"
        echo ""
        "$BIN_DIR/$BINARY_NAME" version
    else
        error "Installation verification failed"
    fi
    
    echo ""
    
    # Check for bundled OPA
    check_bundled_opa
    
    echo ""
    success "Installation complete!"
    echo ""
    echo "Get started with:"
    echo "  cupcake init        # Initialize a new project"
    echo "  cupcake --help      # Show available commands"
    echo ""
    echo "Documentation: https://github.com/${GITHUB_REPO}"
}

# Run main installation
main "$@"