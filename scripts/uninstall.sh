#!/usr/bin/env bash
# Cupcake Uninstallation Script
#
# This script removes Cupcake CLI from your system by:
# - Removing the installation directory (~/.cupcake)
# - Removing PATH modifications from shell profile files
#
# Usage:
#   bash scripts/uninstall.sh
#   curl -fsSL https://raw.githubusercontent.com/eqtylab/cupcake/main/scripts/uninstall.sh | bash

set -e

# Configuration
INSTALL_DIR="${CUPCAKE_INSTALL_DIR:-$HOME/.cupcake}"
BIN_DIR="$INSTALL_DIR/bin"

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
    echo -e "${YELLOW}$1${NC}"
}

info() {
    echo -e "${BLUE}$1${NC}"
}

success() {
    echo -e "${GREEN}$1${NC}"
}

# Scan for Cupcake installations
scan_installations() {
    FOUND_COUNT=0

    echo ""
    info "Scanning for Cupcake installations..."
    echo ""

    # Check installation directory
    if [[ -d "$INSTALL_DIR" ]]; then
        FOUND_COUNT=$((FOUND_COUNT + 1))
        echo "  [${FOUND_COUNT}] Directory: $INSTALL_DIR"

        # Check for binaries
        if [[ -f "$BIN_DIR/cupcake" ]]; then
            local size=$(du -h "$BIN_DIR/cupcake" 2>/dev/null | cut -f1)
            echo "      - cupcake binary ($size)"
        fi
        if [[ -f "$BIN_DIR/opa" ]]; then
            local size=$(du -h "$BIN_DIR/opa" 2>/dev/null | cut -f1)
            echo "      - opa binary ($size)"
        fi

        # Check total size
        local total_size=$(du -sh "$INSTALL_DIR" 2>/dev/null | cut -f1)
        echo "      Total size: $total_size"
    fi

    # Check shell profile files for PATH modifications
    local profile_files=(
        "$HOME/.zshrc"
        "$HOME/.bash_profile"
        "$HOME/.bashrc"
        "$HOME/.config/fish/config.fish"
    )

    for profile in "${profile_files[@]}"; do
        if [[ -f "$profile" ]]; then
            local matches=$(grep -n "cupcake" "$profile" 2>/dev/null || true)
            if [[ -n "$matches" ]]; then
                FOUND_COUNT=$((FOUND_COUNT + 1))
                echo "  [${FOUND_COUNT}] PATH in: $profile"
                while IFS= read -r line; do
                    echo "      Line $line"
                done <<< "$matches"
            fi
        fi
    done

    echo ""

    if [[ $FOUND_COUNT -eq 0 ]]; then
        success "No Cupcake installation found."
        echo ""
        info "Cupcake is not installed or has already been removed."
        exit 0
    fi
}

# Confirm with user
confirm_uninstall() {
    local item_count=$1

    warning "This will remove $item_count item(s) listed above."
    echo ""
    echo -n "Do you want to continue? [y/N] "

    # Read user input
    read -r response

    case "$response" in
        [yY][eE][sS]|[yY])
            return 0
            ;;
        *)
            info "Uninstallation cancelled."
            exit 0
            ;;
    esac
}

# Remove installation directory
remove_install_dir() {
    if [[ -d "$INSTALL_DIR" ]]; then
        info "Removing $INSTALL_DIR..."
        rm -rf "$INSTALL_DIR"
        success "✓ Removed installation directory"
    fi
}

# Remove PATH from profile file
remove_path_from_profile() {
    local profile="$1"

    if [[ -f "$profile" ]]; then
        local matches=$(grep -c "cupcake" "$profile" 2>/dev/null || true)
        if [[ $matches -gt 0 ]]; then
            info "Removing PATH from $profile..."

            # Create backup
            cp "$profile" "${profile}.backup.$(date +%s)"

            # Remove lines containing cupcake (both the comment and the export)
            if [[ "$(uname -s)" == "Darwin" ]]; then
                # macOS sed requires empty string for -i
                sed -i '' '/cupcake/d' "$profile"
            else
                # Linux sed
                sed -i '/cupcake/d' "$profile"
            fi

            success "✓ Removed PATH from $profile"
            info "  (Backup saved as ${profile}.backup.*)"
        fi
    fi
}

# Verify uninstallation
verify_uninstall() {
    echo ""
    info "Verifying uninstallation..."

    local issues=0

    # Check if directory still exists
    if [[ -d "$INSTALL_DIR" ]]; then
        warning "✗ Installation directory still exists: $INSTALL_DIR"
        issues=$((issues + 1))
    else
        success "✓ Installation directory removed"
    fi

    # Check if cupcake is still in PATH
    if command -v cupcake &> /dev/null; then
        warning "✗ 'cupcake' command still found in PATH"
        info "  Location: $(which cupcake)"
        issues=$((issues + 1))
    else
        success "✓ 'cupcake' command not found in PATH"
    fi

    # Check profile files
    local profile_files=(
        "$HOME/.zshrc"
        "$HOME/.bash_profile"
        "$HOME/.bashrc"
        "$HOME/.config/fish/config.fish"
    )

    for profile in "${profile_files[@]}"; do
        if [[ -f "$profile" ]]; then
            if grep -q "cupcake" "$profile" 2>/dev/null; then
                warning "✗ References still found in $profile"
                issues=$((issues + 1))
            fi
        fi
    done

    if [[ $issues -eq 0 ]]; then
        success "✓ All profile files clean"
    fi

    echo ""

    if [[ $issues -gt 0 ]]; then
        warning "Uninstallation completed with $issues issue(s)."
        echo ""
        info "You may need to:"
        echo "  - Restart your terminal"
        echo "  - Source your shell profile (e.g., source ~/.zshrc)"
        echo "  - Manually check the files listed above"
    else
        success "Uninstallation completed successfully!"
        echo ""
        info "You may need to restart your terminal or run:"
        echo "  source ~/.zshrc    (or your shell's profile file)"
    fi
}

# Main uninstallation
main() {
    info "Cupcake Uninstaller"
    info "==================="

    # Scan for installations
    scan_installations

    # Confirm with user
    confirm_uninstall "$FOUND_COUNT"

    echo ""
    info "Uninstalling Cupcake..."
    echo ""

    # Remove installation directory
    remove_install_dir

    # Remove PATH from profile files
    local profile_files=(
        "$HOME/.zshrc"
        "$HOME/.bash_profile"
        "$HOME/.bashrc"
        "$HOME/.config/fish/config.fish"
    )

    for profile in "${profile_files[@]}"; do
        remove_path_from_profile "$profile"
    done

    # Verify uninstallation
    verify_uninstall

    echo ""
    info "Thank you for trying Cupcake!"
}

# Run main uninstallation
main "$@"
