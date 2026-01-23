#!/bin/bash
# pre-commit.sh - Pre-commit checks for all services
#
# Ensures every commit is:
# - Properly formatted (cargo fmt)
# - Lint-free (cargo clippy)
# - Fully tested (unit tests + integration tests)
#
# Runs:
# - cargo fmt --check (formatting)
# - cargo clippy (linting)
# - cargo test --lib (unit tests)
# - buf lint (proto files)
# - Integration tests for ALL services (requires PostgreSQL + MongoDB)
#
# Environment variables:
#   SKIP_INTEG_TESTS=1  - Skip integration tests (faster commits, use sparingly)

set -e

# Ensure cargo is in PATH
export PATH="$HOME/.cargo/bin:$PATH"

# Load environment variables from .env.test if exists
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

# All services in the monorepo
ALL_SERVICES=("auth-service" "service-core" "document-service" "genai-service" "notification-service" "ledger-service" "payment-service" "invoicing-service")

# Check only staged rust files for formatting/linting
STAGED_RS_FILES=$(git diff --cached --name-only --diff-filter=ACM | grep '\.rs$' || true)

# Track which services have changes (for targeted fmt/clippy)
SERVICES_WITH_CHANGES=()

if [ -n "$STAGED_RS_FILES" ]; then
    for service in "${ALL_SERVICES[@]}"; do
        if echo "$STAGED_RS_FILES" | grep -q "^${service}/"; then
            if [ -d "$service" ]; then
                SERVICES_WITH_CHANGES+=("$service")
            fi
        fi
    done

    if [ ${#SERVICES_WITH_CHANGES[@]} -gt 0 ]; then
        log_info "Staged Rust files detected in: ${SERVICES_WITH_CHANGES[*]}"
        echo ""

        # Run formatting and linting for services with changes
        for service in "${SERVICES_WITH_CHANGES[@]}"; do
            log_step "Checking $service..."

            cd "$service"

            # Formatting check
            echo "  Checking formatting..."
            if ! cargo fmt -- --check 2>/dev/null; then
                log_error "Formatting check failed. Running 'cargo fmt' to fix..."
                cargo fmt
                log_error "Please review and re-stage the formatted files."
                exit 1
            fi

            # Clippy check (strict mode: all warnings as errors)
            echo "  Running clippy (strict)..."
            set +e
            clippy_output=$(cargo clippy --jobs 2 -- -D warnings -D clippy::all 2>&1)
            clippy_exit=$?
            set -e
            echo "$clippy_output" | grep -v "^warning: profiles for the non root package" || true
            if [ $clippy_exit -ne 0 ]; then
                log_error "Clippy check failed."
                exit 1
            fi

            # Unit tests only (fast)
            echo "  Running unit tests..."
            set +e
            test_output=$(cargo test --lib --jobs 2 2>&1)
            test_exit=$?
            set -e
            echo "$test_output" | grep -v "^warning: profiles for the non root package" || true
            if [ $test_exit -ne 0 ]; then
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

# Integration tests - run for ALL services
if [ "$SKIP_INTEG_TESTS" = "1" ]; then
    log_warn "Skipping integration tests (SKIP_INTEG_TESTS=1)"
    log_warn "Note: Use sparingly - commits should be fully tested"
else
    log_step "Running integration tests for ALL services..."
    echo ""

    if ! ./scripts/integ-tests.sh; then
        log_error "Integration tests failed."
        exit 1
    fi

    log_info "All integration tests passed"
    echo ""
fi

echo ""
echo -e "${GREEN}═══════════════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}  All pre-commit checks passed!${NC}"
echo -e "${GREEN}═══════════════════════════════════════════════════════════════${NC}"
echo ""
