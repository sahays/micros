#!/bin/bash
# integ-tests.sh - Run integration tests with database setup/teardown
#
# This script runs integration tests for all services that require databases.
# It handles both PostgreSQL (auth-service, ledger-service) and MongoDB
# (document-service, genai-service, notification-service) services.
#
# Usage: ./scripts/integ-tests.sh [options]
#
# Options:
#   -p, --package <name>    Run tests for specific package (can be repeated)
#   --all                   Run all workspace tests (default if no -p specified)
#   --                      Pass remaining args to cargo test
#
# Database requirements:
#   - PostgreSQL: localhost:5432 (for auth-service, ledger-service)
#   - MongoDB: localhost:27017 (for document-service, genai-service, notification-service)
#
# Examples:
#   ./scripts/integ-tests.sh                           # Run all tests
#   ./scripts/integ-tests.sh -p auth-service           # Run auth-service tests
#   ./scripts/integ-tests.sh -p ledger-service         # Run ledger-service tests
#   ./scripts/integ-tests.sh -p document-service       # Run document-service tests

set -e

# PostgreSQL Configuration
PG_HOST="${DB_HOST:-localhost}"
PG_PORT="${DB_PORT:-5432}"
PG_USER="${DB_USER:-postgres}"
PG_PASSWORD="${DB_PASSWORD:-pass@word1}"
PG_DB_NAME="${DB_NAME:-micros_test_$(date +%s)}"

# MongoDB Configuration
MONGO_HOST="${MONGO_HOST:-localhost}"
MONGO_PORT="${MONGO_PORT:-27017}"
MONGO_URI="${MONGODB_URI:-mongodb://${MONGO_HOST}:${MONGO_PORT}}"

# URL-encode the password (@ becomes %40)
PG_PASSWORD_ENCODED="${PG_PASSWORD//@/%40}"

# Export for tests
export TEST_DATABASE_URL="postgres://${PG_USER}:${PG_PASSWORD_ENCODED}@${PG_HOST}:${PG_PORT}/${PG_DB_NAME}"
export MONGODB_URI="${MONGO_URI}"

# Track database states
PG_DB_CREATED=false
PG_AVAILABLE=false
MONGO_AVAILABLE=false

# Services by database type
PG_SERVICES=("auth-service" "ledger-service")
MONGO_SERVICES=("document-service" "genai-service" "notification-service")

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Parse arguments
PACKAGES=()
RUN_ALL=false
CARGO_ARGS=()

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

    if [ "$PG_DB_CREATED" = true ]; then
        log_info "Dropping PostgreSQL test database: ${PG_DB_NAME}"
        PGPASSWORD="${PG_PASSWORD}" psql -h "${PG_HOST}" -p "${PG_PORT}" -U "${PG_USER}" -d postgres \
            -c "DROP DATABASE IF EXISTS ${PG_DB_NAME};" 2>/dev/null || true
    fi

    if [ $exit_code -ne 0 ]; then
        log_error "Tests failed with exit code: $exit_code"
    else
        log_info "All tests completed successfully"
    fi

    exit $exit_code
}

# Set up trap for cleanup
trap cleanup EXIT INT TERM

# Check PostgreSQL availability
check_postgres() {
    log_step "Checking PostgreSQL connection..."
    if PGPASSWORD="${PG_PASSWORD}" psql -h "${PG_HOST}" -p "${PG_PORT}" -U "${PG_USER}" -d postgres -c "SELECT 1;" >/dev/null 2>&1; then
        log_info "PostgreSQL available at ${PG_HOST}:${PG_PORT}"
        PG_AVAILABLE=true
        return 0
    else
        log_warn "PostgreSQL not available at ${PG_HOST}:${PG_PORT}"
        return 1
    fi
}

# Check MongoDB availability
check_mongo() {
    log_step "Checking MongoDB connection..."
    if command -v mongosh &> /dev/null; then
        if mongosh "${MONGO_URI}" --eval "db.runCommand({ping:1})" --quiet >/dev/null 2>&1; then
            log_info "MongoDB available at ${MONGO_URI}"
            MONGO_AVAILABLE=true
            return 0
        fi
    elif command -v mongo &> /dev/null; then
        if mongo "${MONGO_URI}" --eval "db.runCommand({ping:1})" --quiet >/dev/null 2>&1; then
            log_info "MongoDB available at ${MONGO_URI}"
            MONGO_AVAILABLE=true
            return 0
        fi
    else
        # Try with nc as fallback
        if nc -z "${MONGO_HOST}" "${MONGO_PORT}" 2>/dev/null; then
            log_info "MongoDB port open at ${MONGO_HOST}:${MONGO_PORT} (assuming available)"
            MONGO_AVAILABLE=true
            return 0
        fi
    fi
    log_warn "MongoDB not available at ${MONGO_URI}"
    return 1
}

# Create PostgreSQL test database
create_pg_database() {
    log_step "Creating PostgreSQL test database: ${PG_DB_NAME}"

    PGPASSWORD="${PG_PASSWORD}" psql -h "${PG_HOST}" -p "${PG_PORT}" -U "${PG_USER}" -d postgres \
        -c "DROP DATABASE IF EXISTS ${PG_DB_NAME};" 2>/dev/null || true

    PGPASSWORD="${PG_PASSWORD}" psql -h "${PG_HOST}" -p "${PG_PORT}" -U "${PG_USER}" -d postgres \
        -c "CREATE DATABASE ${PG_DB_NAME};"

    PG_DB_CREATED=true
    log_info "PostgreSQL test database created"
}

# Run migrations for PostgreSQL services
run_pg_migrations() {
    log_step "Running PostgreSQL migrations..."

    for service in "${PG_SERVICES[@]}"; do
        if [ -d "${service}/migrations" ]; then
            log_info "Running migrations for ${service}..."

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
        fi
    done

    log_info "All PostgreSQL migrations completed"
}

# Run migrations using direct SQL (fallback)
run_migrations_sql() {
    local service=$1
    local migrations_dir="${service}/migrations"

    for migration in $(ls "${migrations_dir}"/*.sql 2>/dev/null | sort); do
        if [ -f "$migration" ]; then
            log_info "  Applying: $(basename "$migration")"
            PGPASSWORD="${PG_PASSWORD}" psql -h "${PG_HOST}" -p "${PG_PORT}" -U "${PG_USER}" -d "${PG_DB_NAME}" \
                -f "$migration" >/dev/null 2>&1 || {
                log_warn "  Migration may have already been applied: $(basename "$migration")"
            }
        fi
    done
}

# Check if package is in list
is_in_list() {
    local item="$1"
    shift
    local list=("$@")
    for i in "${list[@]}"; do
        if [ "$i" = "$item" ]; then
            return 0
        fi
    done
    return 1
}

# Determine which services to test
get_services_to_test() {
    local pg_to_test=()
    local mongo_to_test=()

    if [ "$RUN_ALL" = true ]; then
        pg_to_test=("${PG_SERVICES[@]}")
        mongo_to_test=("${MONGO_SERVICES[@]}")
    else
        for pkg in "${PACKAGES[@]}"; do
            if is_in_list "$pkg" "${PG_SERVICES[@]}"; then
                pg_to_test+=("$pkg")
            elif is_in_list "$pkg" "${MONGO_SERVICES[@]}"; then
                mongo_to_test+=("$pkg")
            fi
        done
    fi

    echo "${pg_to_test[*]}|${mongo_to_test[*]}"
}

# Run tests for PostgreSQL services
run_pg_tests() {
    local services=("$@")

    if [ ${#services[@]} -eq 0 ]; then
        return 0
    fi

    log_step "Running PostgreSQL service tests: ${services[*]}"

    for service in "${services[@]}"; do
        log_info "Testing ${service}..."

        # PostgreSQL services use #[ignore] for integration tests to separate
        # them from unit tests. We run with --ignored to include these.
        # --test-threads=1 prevents race conditions when tests share a database.
        local extra_args=("--ignored" "--test-threads=1")

        if [ ${#CARGO_ARGS[@]} -gt 0 ]; then
            extra_args+=("${CARGO_ARGS[@]}")
        fi

        log_info "Running: cargo test -p ${service} -- ${extra_args[*]}"
        cargo test -p "${service}" -- "${extra_args[@]}"
    done
}

# Run tests for MongoDB services
run_mongo_tests() {
    local services=("$@")

    if [ ${#services[@]} -eq 0 ]; then
        return 0
    fi

    log_step "Running MongoDB service tests: ${services[*]}"

    for service in "${services[@]}"; do
        log_info "Testing ${service}..."

        if [ ${#CARGO_ARGS[@]} -gt 0 ]; then
            log_info "Running: cargo test -p ${service} -- ${CARGO_ARGS[*]}"
            cargo test -p "${service}" -- "${CARGO_ARGS[@]}"
        else
            log_info "Running: cargo test -p ${service}"
            cargo test -p "${service}"
        fi
    done
}

# Main execution
main() {
    echo ""
    echo -e "${GREEN}═══════════════════════════════════════════════════════════════${NC}"
    echo -e "${GREEN}  Integration Tests${NC}"
    echo -e "${GREEN}═══════════════════════════════════════════════════════════════${NC}"
    echo ""

    # Get services to test
    local services_result
    services_result=$(get_services_to_test)
    local pg_services_str="${services_result%%|*}"
    local mongo_services_str="${services_result##*|}"

    # Convert to arrays
    read -ra pg_services_to_test <<< "$pg_services_str"
    read -ra mongo_services_to_test <<< "$mongo_services_str"

    if [ "$RUN_ALL" = true ]; then
        log_info "Running: all services"
    else
        log_info "Running: ${PACKAGES[*]}"
    fi
    echo ""

    local has_failures=false

    # PostgreSQL services
    if [ ${#pg_services_to_test[@]} -gt 0 ] && [ -n "${pg_services_to_test[0]}" ]; then
        log_info "PostgreSQL services to test: ${pg_services_to_test[*]}"

        if check_postgres; then
            create_pg_database
            run_pg_migrations
            log_info "TEST_DATABASE_URL: postgres://${PG_USER}:****@${PG_HOST}:${PG_PORT}/${PG_DB_NAME}"
            run_pg_tests "${pg_services_to_test[@]}" || has_failures=true
        else
            log_error "PostgreSQL required for: ${pg_services_to_test[*]}"
            has_failures=true
        fi
        echo ""
    fi

    # MongoDB services
    if [ ${#mongo_services_to_test[@]} -gt 0 ] && [ -n "${mongo_services_to_test[0]}" ]; then
        log_info "MongoDB services to test: ${mongo_services_to_test[*]}"

        if check_mongo; then
            log_info "MONGODB_URI: ${MONGO_URI}"
            run_mongo_tests "${mongo_services_to_test[@]}" || has_failures=true
        else
            log_error "MongoDB required for: ${mongo_services_to_test[*]}"
            has_failures=true
        fi
        echo ""
    fi

    if [ "$has_failures" = true ]; then
        exit 1
    fi
}

main
