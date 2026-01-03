#!/bin/bash
set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
NC='\033[0m'

echo "Running pre-commit checks..."

# Check only staged rust files
STAGED_RS_FILES=$(git diff --cached --name-only --diff-filter=ACM | grep '\.rs$' || true)

if [ -n "$STAGED_RS_FILES" ]; then
    echo "Staged Rust files detected. Running checks in auth-service..."
    cd auth-service
    
    echo "Checking formatting..."
    if ! cargo fmt -- --check; then
        echo -e "${RED}Formatting check failed. Running 'cargo fmt' to fix...${NC}"
        cargo fmt
        echo -e "${RED}Please review and re-stage the formatted files.${NC}"
        exit 1
    fi
    
    echo "Running clippy..."
    if ! cargo clippy -- -D warnings; then
        echo -e "${RED}Clippy check failed.${NC}"
        exit 1
    fi
    
    echo "Running tests..."
    if ! cargo test; then
        echo -e "${RED}Tests failed.${NC}"
        exit 1
    fi
    
    cd ..
fi

echo -e "${GREEN}All checks passed!${NC}"
