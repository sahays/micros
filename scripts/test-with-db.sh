#!/bin/bash
# test-with-db.sh - Run workspace tests with a fresh PostgreSQL test database
#
# This script:
# 1. Creates a fresh PostgreSQL test database
# 2. Runs database migrations
# 3. Executes cargo test for all workspace members
# 4. Cleans up the test database after tests complete
# 5. Handles interrupts gracefully (Ctrl+C)
#
# Usage: ./scripts/test-with-db.sh [cargo test options]
# Example: ./scripts/test-with-db.sh --test-threads=1
#          ./scripts/test-with-db.sh -p auth-service

set -e

# Configuration
DB_HOST="${DB_HOST:-localhost}"
DB_PORT="${DB_PORT:-5432}"
DB_USER="${DB_USER:-postgres}"
DB_PASSWORD="${DB_PASSWORD:-pass@word1}"
DB_NAME="${DB_NAME:-micros_test}"

# URL-encode the password (@ becomes %40)
DB_PASSWORD_ENCODED="${DB_PASSWORD//@/%40}"

# Export for tests
export TEST_DATABASE_URL="postgres://${DB_USER}:${DB_PASSWORD_ENCODED}@${DB_HOST}:${DB_PORT}/${DB_NAME}"

# Track if we've created the database
DB_CREATED=false

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Cleanup function
cleanup() {
    local exit_code=$?
    log_info "Cleaning up..."

    if [ "$DB_CREATED" = true ]; then
        log_info "Dropping test database: ${DB_NAME}"
        PGPASSWORD="${DB_PASSWORD}" psql -h "${DB_HOST}" -p "${DB_PORT}" -U "${DB_USER}" -d postgres \
            -c "DROP DATABASE IF EXISTS ${DB_NAME};" 2>/dev/null || true
    fi

    if [ $exit_code -ne 0 ]; then
        log_error "Tests failed with exit code: $exit_code"
    else
        log_info "Cleanup complete"
    fi

    exit $exit_code
}

# Set up trap for cleanup on exit, interrupt, or termination
trap cleanup EXIT INT TERM

# Check if PostgreSQL is accessible
check_postgres() {
    log_info "Checking PostgreSQL connection..."
    if ! PGPASSWORD="${DB_PASSWORD}" psql -h "${DB_HOST}" -p "${DB_PORT}" -U "${DB_USER}" -d postgres -c "SELECT 1;" >/dev/null 2>&1; then
        log_error "Cannot connect to PostgreSQL at ${DB_HOST}:${DB_PORT}"
        log_error "Make sure PostgreSQL is running and accessible"
        exit 1
    fi
    log_info "PostgreSQL connection successful"
}

# Create test database
create_database() {
    log_info "Creating test database: ${DB_NAME}"

    # Drop if exists and create fresh
    PGPASSWORD="${DB_PASSWORD}" psql -h "${DB_HOST}" -p "${DB_PORT}" -U "${DB_USER}" -d postgres \
        -c "DROP DATABASE IF EXISTS ${DB_NAME};" 2>/dev/null || true

    PGPASSWORD="${DB_PASSWORD}" psql -h "${DB_HOST}" -p "${DB_PORT}" -U "${DB_USER}" -d postgres \
        -c "CREATE DATABASE ${DB_NAME};"

    DB_CREATED=true
    log_info "Test database created"
}

# Run migrations
run_migrations() {
    log_info "Running database migrations..."

    # Check if migrations directory exists
    if [ -d "auth-service/migrations" ]; then
        # Run migrations using sqlx (if available) or direct SQL
        if command -v sqlx &> /dev/null; then
            cd auth-service
            DATABASE_URL="${TEST_DATABASE_URL}" sqlx migrate run
            cd ..
        else
            # Fallback: run migration files directly
            for migration in auth-service/migrations/*.sql; do
                if [ -f "$migration" ]; then
                    log_info "Applying migration: $(basename "$migration")"
                    PGPASSWORD="${DB_PASSWORD}" psql -h "${DB_HOST}" -p "${DB_PORT}" -U "${DB_USER}" -d "${DB_NAME}" \
                        -f "$migration" >/dev/null
                fi
            done
        fi
        log_info "Migrations completed"
    else
        log_warn "No migrations directory found at auth-service/migrations"
    fi
}

# Run tests
run_tests() {
    log_info "Running workspace tests..."
    log_info "TEST_DATABASE_URL is set to: postgres://${DB_USER}:****@${DB_HOST}:${DB_PORT}/${DB_NAME}"

    # Pass any additional arguments to cargo test
    # Include --ignored to run database-dependent tests
    # Use --test-threads=1 to avoid race conditions
    cargo test --workspace -- --ignored --test-threads=1 "$@"
}

# Main execution
main() {
    log_info "Starting test-with-db.sh"
    log_info "Using database: ${DB_NAME} on ${DB_HOST}:${DB_PORT}"

    check_postgres
    create_database
    run_migrations
    run_tests "$@"

    log_info "All tests completed successfully"
}

main "$@"
