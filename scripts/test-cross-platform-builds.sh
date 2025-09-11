#!/usr/bin/env bash
# Test cross-platform builds for all supported targets
# This script builds release artifacts for each platform and validates them

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Test configuration
TEST_VERSION="v0.2.0-test"
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

# Supported targets
TARGETS=(
    "x86_64-apple-darwin"
    "aarch64-apple-darwin"
    "x86_64-unknown-linux-gnu"
    "x86_64-unknown-linux-musl"
    "aarch64-unknown-linux-gnu"
    "x86_64-pc-windows-msvc"
)

# Track test results (using regular arrays for compatibility with older Bash)
BUILD_RESULTS=()
CHECKSUM_RESULTS=()
FAILED_BUILDS=()
FAILED_CHECKSUMS=()
SUCCESSFUL_BUILDS=()
SUCCESSFUL_CHECKSUMS=()

# Helper functions
info() {
    echo -e "${BLUE}ℹ $1${NC}"
}

success() {
    echo -e "${GREEN}✓ $1${NC}"
}

warning() {
    echo -e "${YELLOW}⚠ $1${NC}"
}

error() {
    echo -e "${RED}✗ $1${NC}"
}

# Clean up function
cleanup() {
    info "Cleaning up release artifacts..."
    if [[ -d "$PROJECT_ROOT/target/release-artifacts" ]]; then
        rm -rf "$PROJECT_ROOT/target/release-artifacts"/*
        success "Cleaned up release artifacts"
    fi
}

# Verify archive contents
verify_archive() {
    local archive="$1"
    local target="$2"
    
    info "Verifying contents of $archive..."
    
    if [[ "$target" == *"windows"* ]]; then
        # For Windows, use unzip to list contents
        if command -v unzip &> /dev/null; then
            local contents=$(unzip -l "$archive" 2>/dev/null | grep -E "(cupcake\.exe|opa\.exe|README|LICENSE)" | wc -l)
            if [[ $contents -ge 4 ]]; then
                success "Archive contains all required files"
                return 0
            else
                error "Archive missing required files (found $contents/4)"
                return 1
            fi
        else
            warning "unzip not available, skipping Windows archive verification"
            return 0
        fi
    else
        # For Unix, use tar to list contents
        local contents=$(tar -tzf "$archive" 2>/dev/null | grep -E "(bin/cupcake|bin/opa|README|LICENSE)" | wc -l)
        if [[ $contents -ge 4 ]]; then
            success "Archive contains all required files"
            return 0
        else
            error "Archive missing required files (found $contents/4)"
            return 1
        fi
    fi
}

# Main test execution
main() {
    echo "========================================="
    echo "Cross-Platform Build Test Suite"
    echo "========================================="
    echo ""
    info "Testing ${#TARGETS[@]} platform targets"
    info "Test version: $TEST_VERSION"
    echo ""
    
    # Change to project root
    cd "$PROJECT_ROOT"
    
    # Clean any existing artifacts first
    cleanup
    
    # Test each target
    for target in "${TARGETS[@]}"; do
        echo "========================================="
        echo "Testing: $target"
        echo "========================================="
        
        local test_version="${TEST_VERSION}-${target}"
        
        # Run build
        info "Building for $target..."
        if ./scripts/build-release.sh "$target" "$test_version" 2>&1 | tee /tmp/build-${target}.log | grep -E "(Building|Downloading|✓|✗|complete|failed|Error)"; then
            SUCCESSFUL_BUILDS+=("$target")
            success "Build completed for $target"
        else
            FAILED_BUILDS+=("$target")
            error "Build failed for $target"
            echo "  Check /tmp/build-${target}.log for details"
            continue
        fi
        
        # Determine expected file extension
        local ext="tar.gz"
        if [[ "$target" == *"windows"* ]]; then
            ext="zip"
        fi
        
        # Verify archive was created
        local archive_name="cupcake-${test_version}-${target}.${ext}"
        local archive_path="target/release-artifacts/${archive_name}"
        local checksum_path="${archive_path}.sha256"
        
        if [[ -f "$archive_path" ]]; then
            success "Archive created: $archive_name"
            ls -lh "$archive_path" | awk '{print "  Size: " $5}'
            
            # Verify checksum file exists
            if [[ -f "$checksum_path" ]]; then
                success "Checksum file created"
                
                # Verify checksum
                info "Verifying checksum..."
                cd target/release-artifacts
                if [[ "$target" == *"darwin"* ]] || [[ "$(uname)" == "Darwin" ]]; then
                    if shasum -a 256 -c "${archive_name}.sha256" &>/dev/null; then
                        SUCCESSFUL_CHECKSUMS+=("$target")
                        success "Checksum verified"
                    else
                        FAILED_CHECKSUMS+=("$target")
                        error "Checksum verification failed"
                    fi
                else
                    if sha256sum -c "${archive_name}.sha256" &>/dev/null; then
                        SUCCESSFUL_CHECKSUMS+=("$target")
                        success "Checksum verified"
                    else
                        FAILED_CHECKSUMS+=("$target")
                        error "Checksum verification failed"
                    fi
                fi
                cd - > /dev/null
                
                # Verify archive contents
                verify_archive "$archive_path" "$target"
            else
                FAILED_CHECKSUMS+=("$target")
                error "Checksum file not found"
            fi
        else
            error "Archive not created: $archive_name"
            FAILED_BUILDS+=("$target")
        fi
        
        echo ""
    done
    
    # Verify file types
    echo "========================================="
    echo "File Type Verification"
    echo "========================================="
    
    info "Checking Windows builds (.zip)..."
    local zip_count=$(ls target/release-artifacts/*.zip 2>/dev/null | wc -l)
    if [[ $zip_count -gt 0 ]]; then
        success "Found $zip_count Windows archive(s):"
        ls -1 target/release-artifacts/*.zip 2>/dev/null | xargs -n1 basename
    else
        warning "No Windows archives found (may have failed to build)"
    fi
    
    echo ""
    info "Checking Unix builds (.tar.gz)..."
    local tar_count=$(ls target/release-artifacts/*.tar.gz 2>/dev/null | wc -l)
    if [[ $tar_count -gt 0 ]]; then
        success "Found $tar_count Unix archive(s):"
        ls -1 target/release-artifacts/*.tar.gz 2>/dev/null | xargs -n1 basename
    else
        error "No Unix archives found"
    fi
    
    # Summary report
    echo ""
    echo "========================================="
    echo "Test Summary"
    echo "========================================="
    
    local total_builds=${#TARGETS[@]}
    local successful_builds=${#SUCCESSFUL_BUILDS[@]}
    local successful_checksums=${#SUCCESSFUL_CHECKSUMS[@]}
    
    echo "Builds:    $successful_builds/$total_builds successful"
    echo "Checksums: $successful_checksums/$total_builds valid"
    
    if [[ ${#FAILED_BUILDS[@]} -gt 0 ]]; then
        error "Failed builds: ${FAILED_BUILDS[*]}"
    fi
    
    if [[ ${#FAILED_CHECKSUMS[@]} -gt 0 ]]; then
        error "Failed checksums: ${FAILED_CHECKSUMS[*]}"
    fi
    
    # Disk usage
    echo ""
    info "Total disk usage:"
    du -sh target/release-artifacts 2>/dev/null || echo "  Unable to calculate"
    
    # Cleanup prompt
    echo ""
    read -p "Do you want to clean up the release artifacts? (y/N) " -n 1 -r
    echo ""
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        cleanup
    else
        info "Artifacts preserved in target/release-artifacts/"
    fi
    
    # Exit code based on results
    if [[ ${#FAILED_BUILDS[@]} -eq 0 ]] && [[ ${#FAILED_CHECKSUMS[@]} -eq 0 ]]; then
        echo ""
        success "All cross-platform build tests passed!"
        exit 0
    else
        echo ""
        error "Some tests failed. Check the logs for details."
        exit 1
    fi
}

# Handle interrupts gracefully
trap 'echo ""; warning "Test interrupted. Cleaning up..."; cleanup; exit 130' INT TERM

# Run main function
main "$@"