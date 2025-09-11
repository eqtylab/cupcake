#!/usr/bin/env bash
# Cupcake Installation Script
# 
# This script downloads and installs the Cupcake CLI tool.
# It automatically detects your OS and architecture, downloads the appropriate
# binary from GitHub releases, verifies checksums, and installs to your PATH.
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/eqtylab/cupcake/main/scripts/install.sh | sh
#   wget -qO- https://raw.githubusercontent.com/eqtylab/cupcake/main/scripts/install.sh | sh

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
            error "Please use the PowerShell installer for Windows: irm https://raw.githubusercontent.com/${GITHUB_REPO}/main/scripts/install.ps1 | iex"
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
    
    # Add to PATH if not already present
    if [[ ":$PATH:" != *":$BIN_DIR:"* ]]; then
        info "Adding $BIN_DIR to PATH..."
        
        # Detect shell and profile file
        PROFILE_FILE=""
        SHELL_NAME=""
        
        # Check for Zsh
        if [[ -n "$ZSH_VERSION" ]] || [[ "$SHELL" == */zsh ]]; then
            PROFILE_FILE="$HOME/.zshrc"
            SHELL_NAME="zsh"
        # Check for Bash
        elif [[ -n "$BASH_VERSION" ]] || [[ "$SHELL" == */bash ]]; then
            # On macOS, use .bash_profile; on Linux, prefer .bashrc
            if [[ "$PLATFORM" == *"apple-darwin" ]]; then
                PROFILE_FILE="$HOME/.bash_profile"
            elif [[ -f "$HOME/.bashrc" ]]; then
                PROFILE_FILE="$HOME/.bashrc"
            else
                PROFILE_FILE="$HOME/.bash_profile"
            fi
            SHELL_NAME="bash"
        # Check for Fish
        elif [[ "$SHELL" == */fish ]]; then
            PROFILE_FILE="$HOME/.config/fish/config.fish"
            SHELL_NAME="fish"
        # Default fallback
        else
            # Try to detect from SHELL variable
            case "$SHELL" in
                */zsh)
                    PROFILE_FILE="$HOME/.zshrc"
                    SHELL_NAME="zsh"
                    ;;
                */bash)
                    PROFILE_FILE="$HOME/.bashrc"
                    SHELL_NAME="bash"
                    ;;
                *)
                    # Can't auto-detect, provide manual instructions
                    warning "Could not detect shell profile. Please add this to your shell configuration:"
                    echo "  export PATH=\"$BIN_DIR:\$PATH\""
                    echo ""
                    ;;
            esac
        fi
        
        # Add to profile if we detected it
        if [[ -n "$PROFILE_FILE" ]]; then
            # Create profile file if it doesn't exist
            if [[ ! -f "$PROFILE_FILE" ]]; then
                touch "$PROFILE_FILE"
            fi
            
            # Check if PATH export already exists for our bin directory
            if ! grep -q "$BIN_DIR" "$PROFILE_FILE" 2>/dev/null; then
                # Add newline if file doesn't end with one
                [[ -s "$PROFILE_FILE" ]] && [[ $(tail -c1 "$PROFILE_FILE" | wc -l) -eq 0 ]] && echo "" >> "$PROFILE_FILE"
                
                # Add PATH export based on shell type
                if [[ "$SHELL_NAME" == "fish" ]]; then
                    echo "# Added by Cupcake installer" >> "$PROFILE_FILE"
                    echo "set -gx PATH \"$BIN_DIR\" \$PATH" >> "$PROFILE_FILE"
                else
                    echo "# Added by Cupcake installer" >> "$PROFILE_FILE"
                    echo "export PATH=\"$BIN_DIR:\$PATH\"" >> "$PROFILE_FILE"
                fi
                
                success "✓ Added $BIN_DIR to PATH in $PROFILE_FILE"
                echo ""
                info "To use 'cupcake' in your current shell, run:"
                echo "  source $PROFILE_FILE"
                echo ""
                info "Or restart your terminal."
            else
                info "$BIN_DIR already in $PROFILE_FILE"
            fi
        fi
        
        # Also export for current session
        export PATH="$BIN_DIR:$PATH"
    else
        info "Already in PATH: $BIN_DIR"
    fi
    
    # Verify installation
    if "$BIN_DIR/$BINARY_NAME" --version &> /dev/null; then
        success "✓ Cupcake installed successfully!"
        echo ""
        "$BIN_DIR/$BINARY_NAME" --version
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