#!/bin/bash
# integ-tests.sh - Run integration tests against deployed containers
#
# Tests are thin gRPC clients that connect to already-running services.
# Data isolation is achieved via unique UUID tenant IDs per test.
#
# Usage: ./scripts/integ-tests.sh [options]
#
# Options:
#   -p, --package <name>    Run tests for specific package (can be repeated)
#                           Valid: workflow-tests, payment-service
#   --all                   Run all tests (default if no -p specified)
#   --                      Pass remaining args to cargo test
#
# Requirements:
#   - All services running via ./scripts/dev-up.sh
#
# Examples:
#   ./scripts/integ-tests.sh                           # Run all tests
#   ./scripts/integ-tests.sh -p workflow-tests         # Run workflow + service tests
#   ./scripts/integ-tests.sh -p payment-service        # Run payment embedded tests

set -e

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

# Check if all services are running via health endpoints
check_services_running() {
    log_step "Checking if all services are running..."

    local services_healthy=true
    local health_endpoints=(
        "auth:http://localhost:9005/health"
        "billing:http://localhost:9012/health"
        "document:http://localhost:9007/health"
        "genai:http://localhost:9010/health"
        "invoicing:http://localhost:9014/health"
        "ledger:http://localhost:9011/health"
        "notification:http://localhost:9008/health"
        "payment:http://localhost:9009/health"
        "reconciliation:http://localhost:9013/health"
    )

    for entry in "${health_endpoints[@]}"; do
        local name="${entry%%:*}"
        local url="${entry#*:}"

        if curl -sf --max-time 2 "$url" > /dev/null 2>&1; then
            log_info "  ✓ ${name}-service is healthy"
        else
            log_warn "  ✗ ${name}-service is not responding at ${url}"
            services_healthy=false
        fi
    done

    if [ "$services_healthy" = true ]; then
        log_info "All services are healthy"
        return 0
    else
        log_error "Some services are not running. Start them with: ./scripts/dev-up.sh"
        return 1
    fi
}

# Export gRPC endpoints for all services
export_grpc_endpoints() {
    export AUTH_GRPC_ENDPOINT="http://localhost:50051"
    export BILLING_GRPC_ENDPOINT="http://localhost:50057"
    export DOCUMENT_GRPC_ENDPOINT="http://localhost:50052"
    export GENAI_GRPC_ENDPOINT="http://localhost:50055"
    export INVOICING_GRPC_ENDPOINT="http://localhost:50059"
    export LEDGER_GRPC_ENDPOINT="http://localhost:50056"
    export NOTIFICATION_GRPC_ENDPOINT="http://localhost:50053"
    export PAYMENT_GRPC_ENDPOINT="http://localhost:50054"
    export RECONCILIATION_GRPC_ENDPOINT="http://localhost:50058"

    log_info "gRPC endpoints exported"
}

# Export ADMIN_API_KEY from .env.dev
export_admin_api_key() {
    local env_file=".env.dev"
    if [ -f "$env_file" ]; then
        local key
        key=$(grep '^ADMIN_API_KEY=' "$env_file" | cut -d'=' -f2-)
        if [ -n "$key" ]; then
            export ADMIN_API_KEY="$key"
            log_info "ADMIN_API_KEY exported from ${env_file}"
            return 0
        fi
    fi
    log_warn "ADMIN_API_KEY not found in ${env_file} — auth-service admin tests may fail"
    return 0
}

# Main execution
main() {
    echo ""
    echo -e "${GREEN}═══════════════════════════════════════════════════════════════${NC}"
    echo -e "${GREEN}  Integration Tests (thin gRPC client mode)${NC}"
    echo -e "${GREEN}═══════════════════════════════════════════════════════════════${NC}"
    echo ""

    # Determine which packages to test
    local run_workflow=false
    local run_payment=false

    if [ "$RUN_ALL" = true ]; then
        run_workflow=true
        run_payment=true
    else
        for pkg in "${PACKAGES[@]}"; do
            case "$pkg" in
                workflow-tests) run_workflow=true ;;
                payment-service) run_payment=true ;;
                *) log_warn "Unknown package: $pkg (valid: workflow-tests, payment-service)" ;;
            esac
        done
    fi

    if [ "$RUN_ALL" = true ]; then
        log_info "Running: all tests"
    else
        log_info "Running: ${PACKAGES[*]}"
    fi
    echo ""

    # Verify services are running
    if ! check_services_running; then
        exit 1
    fi
    echo ""

    # Export endpoints and keys
    export_grpc_endpoints
    export_admin_api_key
    echo ""

    # Pre-compile all test binaries in one pass
    local compile_pkgs=()
    if [ "$run_workflow" = true ]; then
        compile_pkgs+=("-p" "workflow-tests")
    fi
    if [ "$run_payment" = true ]; then
        compile_pkgs+=("-p" "payment-service")
    fi
    if [ ${#compile_pkgs[@]} -gt 0 ]; then
        log_step "Compiling all test binaries..."
        cargo test --no-run "${compile_pkgs[@]}" 2>&1 | tail -3
        echo ""
    fi

    local has_failures=false

    # Workflow tests (all service integration tests + cross-service workflows)
    if [ "$run_workflow" = true ]; then
        log_step "Running workflow tests (all service integration tests)..."
        local extra_args=("--test-threads=1")
        if [ ${#CARGO_ARGS[@]} -gt 0 ]; then
            extra_args+=("${CARGO_ARGS[@]}")
        fi
        log_info "Running: cargo test -p workflow-tests -- ${extra_args[*]}"
        cargo test -p workflow-tests -- "${extra_args[@]}" || has_failures=true
        echo ""
    fi

    # Payment embedded tests (wiremock)
    if [ "$run_payment" = true ]; then
        log_step "Running payment-service embedded tests (wiremock)..."
        if [ ${#CARGO_ARGS[@]} -gt 0 ]; then
            log_info "Running: cargo test -p payment-service -- ${CARGO_ARGS[*]}"
            cargo test -p payment-service -- "${CARGO_ARGS[@]}" || has_failures=true
        else
            log_info "Running: cargo test -p payment-service"
            cargo test -p payment-service || has_failures=true
        fi
        echo ""
    fi

    if [ "$has_failures" = true ]; then
        log_error "Some tests failed"
        exit 1
    fi

    log_info "All tests completed successfully"
}

main
