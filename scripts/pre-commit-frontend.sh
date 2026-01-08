#!/bin/bash
set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
NC='\033[0m'

echo "Running Rust frontend pre-commit checks..."

# Check only staged secure-frontend files
STAGED_FE_FILES=$(git diff --cached --name-only --diff-filter=ACM | grep '^secure-frontend/' || true)

if [ -n "$STAGED_FE_FILES" ]; then
    echo "Staged secure-frontend files detected. Running checks..."
    cd secure-frontend

    echo "Running cargo fmt check..."
    if ! cargo fmt -- --check; then
        echo -e "${RED}Format check failed. Run 'cargo fmt' to fix.${NC}"
        exit 1
    fi

    echo "Running cargo clippy..."
    if ! cargo clippy -- -D warnings; then
        echo -e "${RED}Clippy failed.${NC}"
        exit 1
    fi

    cd ..
fi

echo -e "${GREEN}Frontend checks passed!${NC}"
