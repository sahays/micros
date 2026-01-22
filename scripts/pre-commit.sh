#!/bin/bash
# pre-commit.sh - Pre-commit checks for staged files
#
# Runs:
# - cargo fmt --check (formatting)
# - cargo clippy (linting)
# - cargo test --lib (unit tests)
# - Integration tests (if database is available)
# - buf lint (proto files)
#
# Environment variables:
#   SKIP_INTEG_TESTS=1  - Skip integration tests (faster commits)
#   DB_HOST, DB_PORT, DB_USER, DB_PASSWORD - Database connection (for integ tests)

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

# All services in the monorepo
ALL_SERVICES=("auth-service" "service-core" "document-service" "genai-service" "notification-service" "ledger-service" "payment-service")

# Check only staged rust files
STAGED_RS_FILES=$(git diff --cached --name-only --diff-filter=ACM | grep '\.rs$' || true)

# Track which services have changes
SERVICES_WITH_CHANGES=()

if [ -n "$STAGED_RS_FILES" ]; then
    # Detect which services have changes
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

        # Run checks for each service with changes
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

# Integration tests section
if [ "$SKIP_INTEG_TESTS" = "1" ]; then
    log_warn "Skipping integration tests (SKIP_INTEG_TESTS=1)"
else
    log_step "Running integration tests..."

    # Check if we have any PostgreSQL-dependent services with changes
    PG_SERVICES_CHANGED=()
    for service in "${SERVICES_WITH_CHANGES[@]}"; do
        if [ "$service" = "auth-service" ] || [ "$service" = "ledger-service" ]; then
            PG_SERVICES_CHANGED+=("$service")
        fi
    done

    if [ ${#PG_SERVICES_CHANGED[@]} -gt 0 ]; then
        # Check if PostgreSQL is available
        DB_HOST="${DB_HOST:-localhost}"
        DB_PORT="${DB_PORT:-5432}"
        DB_USER="${DB_USER:-postgres}"
        DB_PASSWORD="${DB_PASSWORD:-pass@word1}"

        if PGPASSWORD="${DB_PASSWORD}" psql -h "${DB_HOST}" -p "${DB_PORT}" -U "${DB_USER}" -d postgres -c "SELECT 1;" >/dev/null 2>&1; then
            log_info "PostgreSQL available, running integration tests for: ${PG_SERVICES_CHANGED[*]}"

            # Build the package arguments
            PKG_ARGS=""
            for service in "${PG_SERVICES_CHANGED[@]}"; do
                PKG_ARGS="$PKG_ARGS -p $service"
            done

            # Run integration tests
            if ! ./scripts/integ-tests.sh $PKG_ARGS; then
                log_error "Integration tests failed."
                exit 1
            fi
            log_info "Integration tests passed"
        else
            log_warn "PostgreSQL not available at ${DB_HOST}:${DB_PORT}"
            log_warn "Skipping integration tests. To run them:"
            log_warn "  1. Start PostgreSQL"
            log_warn "  2. Set DB_HOST, DB_PORT, DB_USER, DB_PASSWORD if needed"
            log_warn "  3. Or run manually: ./scripts/integ-tests.sh"
        fi
    else
        log_info "No PostgreSQL-dependent services changed, skipping database tests"
    fi
    echo ""
fi

echo ""
echo -e "${GREEN}═══════════════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}  All pre-commit checks passed!${NC}"
echo -e "${GREEN}═══════════════════════════════════════════════════════════════${NC}"
echo ""
