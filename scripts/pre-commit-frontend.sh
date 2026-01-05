#!/bin/bash
set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
NC='\033[0m'

echo "Running frontend pre-commit checks..."

# Check only staged frontend files
STAGED_FE_FILES=$(git diff --cached --name-only --diff-filter=ACM | grep '^sample-frontend/' || true)

if [ -n "$STAGED_FE_FILES" ]; then
    echo "Staged frontend files detected. Running checks in sample-frontend..."
    cd sample-frontend
    
    # Use Bun for all operations
    
    echo "Running Formatting Check..."
    if ! bun x prettier --check "src/**/*.{ts,tsx,css}"; then
        echo -e "${RED}Formatting Check failed. Run 'bun x prettier --write src/' to fix.${NC}"
        exit 1
    fi
    
    echo "Running Type Check..."
    if ! bun x tsc --noEmit; then
        echo -e "${RED}Type Check failed.${NC}"
        exit 1
    fi
    
    echo "Running Linter..."
    if ! bun run lint; then
        echo -e "${RED}Linting failed.${NC}"
        exit 1
    fi
    
    # Only run build if we want to ensure total consistency
    # echo "Verifying Build..."
    # if ! bun run build; then
    #     echo -e "${RED}Build failed.${NC}"
    #     exit 1
    # fi
    
    cd ..
fi

echo -e "${GREEN}Frontend checks passed!${NC}"
