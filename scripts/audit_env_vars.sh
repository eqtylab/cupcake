#!/bin/bash
# Environment Variable Audit Script
#
# Searches the codebase for all environment variable usage
# to ensure complete migration to CLI flags.
#
# Usage: ./scripts/audit_env_vars.sh

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Deprecated environment variables
DEPRECATED_ENV_VARS=(
    "CUPCAKE_TRACE"
    "RUST_LOG"
    "CUPCAKE_GLOBAL_CONFIG"
    "CUPCAKE_WASM_MAX_MEMORY"
    "CUPCAKE_DEBUG_FILES"
    "CUPCAKE_DEBUG_ROUTING"
    "CUPCAKE_OPA_PATH"
)

echo -e "${BLUE}=======================================${NC}"
echo -e "${BLUE}Environment Variable Audit${NC}"
echo -e "${BLUE}=======================================${NC}"
echo ""

# Track if we found any issues
ISSUES_FOUND=0

# 1. Check Rust code for env::var calls
echo -e "${YELLOW}[1/5] Checking Rust code for env::var usage...${NC}"
echo ""

for env_var in "${DEPRECATED_ENV_VARS[@]}"; do
    echo -e "  Searching for: ${BLUE}${env_var}${NC}"

    # Search in Rust files
    RUST_MATCHES=$(rg -t rust "env::var.*${env_var}" --count-matches 2>/dev/null || echo "0")

    if [ "$RUST_MATCHES" != "0" ]; then
        echo -e "    ${RED}✗ Found ${RUST_MATCHES} occurrences in Rust code${NC}"
        rg -t rust "env::var.*${env_var}" --line-number --color always | sed 's/^/      /'
        ISSUES_FOUND=$((ISSUES_FOUND + 1))
    else
        echo -e "    ${GREEN}✓ No occurrences in Rust code${NC}"
    fi
done

echo ""

# 2. Check documentation for environment variable mentions
echo -e "${YELLOW}[2/5] Checking documentation for env var references...${NC}"
echo ""

for env_var in "${DEPRECATED_ENV_VARS[@]}"; do
    echo -e "  Searching docs for: ${BLUE}${env_var}${NC}"

    # Search in markdown files
    MD_MATCHES=$(rg -t md "${env_var}" --count-matches 2>/dev/null || echo "0")

    if [ "$MD_MATCHES" != "0" ]; then
        # Exclude expected mentions in migration docs
        EXCLUDED=$(rg -t md "${env_var}" -g '!ENVIRONMENT_VARIABLES.md' -g '!*MIGRATION*.md' -g '!ENV_VAR*.md' -g '!BASELINE*.md' --count-matches 2>/dev/null || echo "0")

        if [ "$EXCLUDED" != "0" ]; then
            echo -e "    ${YELLOW}⚠ Found ${EXCLUDED} occurrences in docs (may need updates)${NC}"
            rg -t md "${env_var}" -g '!ENVIRONMENT_VARIABLES.md' -g '!*MIGRATION*.md' -g '!ENV_VAR*.md' -g '!BASELINE*.md' --line-number --color always | sed 's/^/      /' | head -5
        else
            echo -e "    ${GREEN}✓ Only mentioned in migration docs${NC}"
        fi
    else
        echo -e "    ${GREEN}✓ No occurrences in docs${NC}"
    fi
done

echo ""

# 3. Check shell scripts for environment variable usage
echo -e "${YELLOW}[3/5] Checking shell scripts for env var usage...${NC}"
echo ""

for env_var in "${DEPRECATED_ENV_VARS[@]}"; do
    echo -e "  Searching scripts for: ${BLUE}${env_var}${NC}"

    # Search in shell scripts
    SH_MATCHES=$(rg -t sh "${env_var}" --count-matches 2>/dev/null || echo "0")

    if [ "$SH_MATCHES" != "0" ]; then
        # Exclude this audit script itself
        EXCLUDED=$(rg -t sh "${env_var}" -g '!audit_env_vars.sh' --count-matches 2>/dev/null || echo "0")

        if [ "$EXCLUDED" != "0" ]; then
            echo -e "    ${RED}✗ Found ${EXCLUDED} occurrences in shell scripts${NC}"
            rg -t sh "${env_var}" -g '!audit_env_vars.sh' --line-number --color always | sed 's/^/      /'
            ISSUES_FOUND=$((ISSUES_FOUND + 1))
        else
            echo -e "    ${GREEN}✓ Only in audit script${NC}"
        fi
    else
        echo -e "    ${GREEN}✓ No occurrences in shell scripts${NC}"
    fi
done

echo ""

# 4. Check YAML/TOML config files
echo -e "${YELLOW}[4/5] Checking config files for env var references...${NC}"
echo ""

for env_var in "${DEPRECATED_ENV_VARS[@]}"; do
    echo -e "  Searching configs for: ${BLUE}${env_var}${NC}"

    # Search in YAML and TOML files
    CONFIG_MATCHES=$(rg -t yaml -t toml "${env_var}" --count-matches 2>/dev/null || echo "0")

    if [ "$CONFIG_MATCHES" != "0" ]; then
        echo -e "    ${YELLOW}⚠ Found ${CONFIG_MATCHES} occurrences in config files${NC}"
        rg -t yaml -t toml "${env_var}" --line-number --color always | sed 's/^/      /'
    else
        echo -e "    ${GREEN}✓ No occurrences in config files${NC}"
    fi
done

echo ""

# 5. Check for generic env::var patterns that might be missed
echo -e "${YELLOW}[5/5] Checking for any remaining env::var calls...${NC}"
echo ""

ALL_ENV_VAR_CALLS=$(rg -t rust 'env::var\("' --count 2>/dev/null || echo "0")

if [ "$ALL_ENV_VAR_CALLS" != "0" ]; then
    echo -e "  ${BLUE}Found ${ALL_ENV_VAR_CALLS} total env::var calls in Rust code:${NC}"
    echo ""
    rg -t rust 'env::var\("' --line-number --color always | sed 's/^/    /'
    echo ""
    echo -e "  ${YELLOW}⚠ Review these calls to ensure they're not behavioral config${NC}"
else
    echo -e "  ${GREEN}✓ No env::var calls found${NC}"
fi

echo ""

# Summary
echo -e "${BLUE}=======================================${NC}"
echo -e "${BLUE}Audit Summary${NC}"
echo -e "${BLUE}=======================================${NC}"
echo ""

if [ $ISSUES_FOUND -eq 0 ]; then
    echo -e "${GREEN}✓ No critical issues found!${NC}"
    echo ""
    echo "All deprecated environment variables have been removed from"
    echo "Rust code and shell scripts."
    echo ""
else
    echo -e "${RED}✗ Found ${ISSUES_FOUND} critical issues${NC}"
    echo ""
    echo "The following need to be addressed:"
    echo "  1. Remove env::var calls for deprecated variables in Rust code"
    echo "  2. Update shell scripts to use CLI flags instead"
    echo ""
    exit 1
fi

echo "Recommendations:"
echo "  • Update documentation to reference CLI flags"
echo "  • Review all env::var calls to ensure they're necessary"
echo "  • Run migration script on guidebook files:"
echo "    python scripts/migrate_guidebook.py .cupcake/guidebook.yml"
echo ""

exit 0
