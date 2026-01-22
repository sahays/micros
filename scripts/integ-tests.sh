#!/bin/bash
# integ-tests.sh - Run integration tests with database setup/teardown
#
# This script replaces cargo test for integration tests that need a database.
# It:
# 1. Creates a fresh PostgreSQL test database
# 2. Runs database migrations for all services
# 3. Executes cargo test for specified packages
# 4. Cleans up the test database after tests complete
# 5. Handles interrupts gracefully (Ctrl+C)
#
# Usage: ./scripts/integ-tests.sh [options]
#
# Options:
#   -p, --package <name>    Run tests for specific package (can be repeated)
#   --all                   Run all workspace tests (default if no -p specified)
#   --lib                   Run only library tests (no integration tests)
#   --ignored               Run only ignored tests (database tests)
#   --skip-db               Skip database setup (for non-DB tests)
#   --                      Pass remaining args to cargo test
#
# Examples:
#   ./scripts/integ-tests.sh                           # Run all tests
#   ./scripts/integ-tests.sh -p auth-service           # Run auth-service tests
#   ./scripts/integ-tests.sh -p ledger-service         # Run ledger-service tests
#   ./scripts/integ-tests.sh -p auth-service --ignored # Run database tests only
#   ./scripts/integ-tests.sh -- --test-threads=1       # Pass args to cargo

set -e

# Configuration
DB_HOST="${DB_HOST:-localhost}"
DB_PORT="${DB_PORT:-5432}"
DB_USER="${DB_USER:-postgres}"
DB_PASSWORD="${DB_PASSWORD:-pass@word1}"
DB_NAME="${DB_NAME:-micros_test_$(date +%s)}"

# URL-encode the password (@ becomes %40)
DB_PASSWORD_ENCODED="${DB_PASSWORD//@/%40}"

# Export for tests
export TEST_DATABASE_URL="postgres://${DB_USER}:${DB_PASSWORD_ENCODED}@${DB_HOST}:${DB_PORT}/${DB_NAME}"

# Track if we've created the database
DB_CREATED=false
SKIP_DB=false

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Parse arguments
PACKAGES=()
RUN_ALL=false
CARGO_ARGS=()
TEST_TYPE=""

while [[ $# -gt 0 ]]; do
    case $1 in
        -p|--package)
            PACKAGES+=("$2")
            shift 2
            ;;
        --all)
            RUN_ALL=true
            shift
            ;;
        --lib)
            TEST_TYPE="--lib"
            shift
            ;;
        --ignored)
            CARGO_ARGS+=("--ignored")
            shift
            ;;
        --skip-db)
            SKIP_DB=true
            shift
            ;;
        --)
            shift
            CARGO_ARGS+=("$@")
            break
            ;;
        *)
            CARGO_ARGS+=("$1")
            shift
            ;;
    esac
done

# If no packages specified and not --all, default to all
if [ ${#PACKAGES[@]} -eq 0 ] && [ "$RUN_ALL" = false ]; then
    RUN_ALL=true
fi

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

# Cleanup function
cleanup() {
    local exit_code=$?

    if [ "$DB_CREATED" = true ]; then
        log_info "Dropping test database: ${DB_NAME}"
        PGPASSWORD="${DB_PASSWORD}" psql -h "${DB_HOST}" -p "${DB_PORT}" -U "${DB_USER}" -d postgres \
            -c "DROP DATABASE IF EXISTS ${DB_NAME};" 2>/dev/null || true
    fi

    if [ $exit_code -ne 0 ]; then
        log_error "Tests failed with exit code: $exit_code"
    else
        log_info "Tests completed successfully"
    fi

    exit $exit_code
}

# Set up trap for cleanup on exit, interrupt, or termination
trap cleanup EXIT INT TERM

# Check if PostgreSQL is accessible
check_postgres() {
    log_step "Checking PostgreSQL connection..."
    if ! PGPASSWORD="${DB_PASSWORD}" psql -h "${DB_HOST}" -p "${DB_PORT}" -U "${DB_USER}" -d postgres -c "SELECT 1;" >/dev/null 2>&1; then
        log_error "Cannot connect to PostgreSQL at ${DB_HOST}:${DB_PORT}"
        log_error "Make sure PostgreSQL is running and accessible"
        log_error "You can configure connection with: DB_HOST, DB_PORT, DB_USER, DB_PASSWORD"
        return 1
    fi
    log_info "PostgreSQL connection successful"
    return 0
}

# Create test database
create_database() {
    log_step "Creating test database: ${DB_NAME}"

    # Drop if exists and create fresh
    PGPASSWORD="${DB_PASSWORD}" psql -h "${DB_HOST}" -p "${DB_PORT}" -U "${DB_USER}" -d postgres \
        -c "DROP DATABASE IF EXISTS ${DB_NAME};" 2>/dev/null || true

    PGPASSWORD="${DB_PASSWORD}" psql -h "${DB_HOST}" -p "${DB_PORT}" -U "${DB_USER}" -d postgres \
        -c "CREATE DATABASE ${DB_NAME};"

    DB_CREATED=true
    log_info "Test database created"
}

# Run migrations for a specific service
run_service_migrations() {
    local service=$1
    local migrations_dir="${service}/migrations"

    if [ ! -d "$migrations_dir" ]; then
        return 0
    fi

    log_info "Running migrations for ${service}..."

    # Run migrations using sqlx (if available) or direct SQL
    if command -v sqlx &> /dev/null; then
        (
            cd "${service}"
            DATABASE_URL="${TEST_DATABASE_URL}" sqlx migrate run 2>&1 || {
                log_warn "sqlx migrate failed for ${service}, trying direct SQL"
                cd ..
                run_migrations_sql "$service"
            }
        )
    else
        run_migrations_sql "$service"
    fi

    log_info "${service} migrations completed"
}

# Run migrations using direct SQL (fallback)
run_migrations_sql() {
    local service=$1
    local migrations_dir="${service}/migrations"

    # Run migration files directly in order
    for migration in $(ls "${migrations_dir}"/*.sql 2>/dev/null | sort); do
        if [ -f "$migration" ]; then
            log_info "  Applying: $(basename "$migration")"
            PGPASSWORD="${DB_PASSWORD}" psql -h "${DB_HOST}" -p "${DB_PORT}" -U "${DB_USER}" -d "${DB_NAME}" \
                -f "$migration" >/dev/null 2>&1 || {
                log_warn "  Migration may have already been applied: $(basename "$migration")"
            }
        fi
    done
}

# Run all migrations
run_migrations() {
    log_step "Running database migrations..."

    # Services with PostgreSQL migrations (order matters for dependencies)
    local pg_services=("auth-service" "ledger-service")

    for service in "${pg_services[@]}"; do
        if [ -d "${service}/migrations" ]; then
            run_service_migrations "$service"
        fi
    done

    log_info "All migrations completed"
}

# Run tests
run_tests() {
    log_step "Running tests..."
    log_info "TEST_DATABASE_URL: postgres://${DB_USER}:****@${DB_HOST}:${DB_PORT}/${DB_NAME}"

    # Build cargo test command
    local cmd="cargo test"

    if [ "$RUN_ALL" = true ]; then
        cmd="$cmd --workspace"
    else
        for pkg in "${PACKAGES[@]}"; do
            cmd="$cmd -p $pkg"
        done
    fi

    if [ -n "$TEST_TYPE" ]; then
        cmd="$cmd $TEST_TYPE"
    fi

    # Add -- separator if we have cargo args
    if [ ${#CARGO_ARGS[@]} -gt 0 ]; then
        cmd="$cmd -- ${CARGO_ARGS[*]}"
    fi

    log_info "Running: $cmd"
    eval $cmd
}

# Main execution
main() {
    echo ""
    echo -e "${GREEN}═══════════════════════════════════════════════════════════════${NC}"
    echo -e "${GREEN}  Integration Tests with Database Setup${NC}"
    echo -e "${GREEN}═══════════════════════════════════════════════════════════════${NC}"
    echo ""

    log_info "Database: ${DB_NAME} on ${DB_HOST}:${DB_PORT}"

    if [ "$RUN_ALL" = true ]; then
        log_info "Packages: all workspace packages"
    else
        log_info "Packages: ${PACKAGES[*]}"
    fi

    echo ""

    if [ "$SKIP_DB" = true ]; then
        log_info "Skipping database setup (--skip-db)"
        run_tests
    else
        check_postgres || exit 1
        create_database
        run_migrations
        run_tests
    fi
}

main
