#!/bin/bash
# pre-commit.sh - Fast pre-commit checks for staged files
#
# Runs:
# - cargo fmt --check (formatting)
# - cargo clippy (linting)
# - cargo test --lib (unit tests only, no database required)
# - buf lint (proto files)
#
# For full integration tests with database, run: ./scripts/integ-tests.sh

set -e

# Ensure cargo is in PATH (common locations)
export PATH="$HOME/.cargo/bin:$PATH"

# Load environment variables from .env.test for local tests
if [ -f ".env.test" ]; then
    export $(grep -v '^#' .env.test | xargs 2>/dev/null) || true
fi

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

log_step() {
    echo -e "${BLUE}[STEP]${NC} $1"
}

echo ""
echo -e "${GREEN}═══════════════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}  Pre-Commit Checks${NC}"
echo -e "${GREEN}═══════════════════════════════════════════════════════════════${NC}"
echo ""

# Check only staged rust files
STAGED_RS_FILES=$(git diff --cached --name-only --diff-filter=ACM | grep '\.rs$' || true)

# Track which services have changes
SERVICES_WITH_CHANGES=()

if [ -n "$STAGED_RS_FILES" ]; then
    # Detect which services have changes
    if echo "$STAGED_RS_FILES" | grep -q "auth-service/"; then
        SERVICES_WITH_CHANGES+=("auth-service")
    fi
    if echo "$STAGED_RS_FILES" | grep -q "service-core/"; then
        SERVICES_WITH_CHANGES+=("service-core")
    fi
    if echo "$STAGED_RS_FILES" | grep -q "document-service/"; then
        SERVICES_WITH_CHANGES+=("document-service")
    fi
    if echo "$STAGED_RS_FILES" | grep -q "notification-service/"; then
        SERVICES_WITH_CHANGES+=("notification-service")
    fi

    if [ ${#SERVICES_WITH_CHANGES[@]} -gt 0 ]; then
        log_info "Staged Rust files detected in: ${SERVICES_WITH_CHANGES[*]}"
        echo ""

        # Run checks for each service with changes
        for service in "${SERVICES_WITH_CHANGES[@]}"; do
            log_step "Checking $service..."

            # Check if service directory exists
            if [ ! -d "$service" ]; then
                log_warn "Directory $service not found, skipping"
                continue
            fi

            cd "$service"

            # Formatting check
            echo "  Checking formatting..."
            if ! cargo fmt -- --check 2>/dev/null; then
                log_error "Formatting check failed. Running 'cargo fmt' to fix..."
                cargo fmt
                log_error "Please review and re-stage the formatted files."
                exit 1
            fi

            # Clippy check
            echo "  Running clippy..."
            if ! cargo clippy --jobs 2 -- -D warnings 2>&1 | grep -v "^warning: profiles for the non root package"; then
                log_error "Clippy check failed."
                exit 1
            fi

            # Unit tests only (fast, no database required)
            echo "  Running unit tests..."
            if ! cargo test --lib --jobs 2 2>&1 | grep -v "^warning: profiles for the non root package"; then
                log_error "Unit tests failed."
                exit 1
            fi

            cd ..
            log_info "$service checks passed"
            echo ""
        done
    fi
fi

# Check staged proto files
STAGED_PROTO_FILES=$(git diff --cached --name-only --diff-filter=ACM | grep '\.proto$' || true)

if [ -n "$STAGED_PROTO_FILES" ]; then
    log_step "Checking proto files..."

    # Check if buf is installed
    if command -v buf &> /dev/null; then
        cd proto
        if ! buf lint; then
            log_error "Proto lint check failed."
            exit 1
        fi
        cd ..
        log_info "Proto lint passed"
    else
        log_warn "buf not installed. Skipping proto lint."
        echo "Install buf with: brew install bufbuild/buf/buf"
    fi
    echo ""
fi

echo ""
echo -e "${GREEN}═══════════════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}  All pre-commit checks passed!${NC}"
echo -e "${GREEN}═══════════════════════════════════════════════════════════════${NC}"
echo ""
log_info "For full integration tests with database, run: ./scripts/integ-tests.sh"
echo ""
