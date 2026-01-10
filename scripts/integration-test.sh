#!/bin/bash
set -e

# Integration Test Runner
# Runs Rust integration tests with proper environment configuration

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

# Ensure cargo is in PATH
export PATH="$HOME/.cargo/bin:$PATH"

# Load environment variables from .env.test for tests
if [ -f ".env.test" ]; then
    echo -e "${YELLOW}Loading environment variables from .env.test...${NC}"
    export $(grep -v '^#' .env.test | xargs)
else
    echo -e "${RED}Error: .env.test not found${NC}"
    echo "Please create .env.test from .env.example"
    exit 1
fi

# Parse arguments
SERVICE=""
JOBS=2
VERBOSE=false

while [[ $# -gt 0 ]]; do
    case $1 in
        -s|--service)
            SERVICE="$2"
            shift 2
            ;;
        -j|--jobs)
            JOBS="$2"
            shift 2
            ;;
        -v|--verbose)
            VERBOSE=true
            shift
            ;;
        -h|--help)
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  -s, --service <name>   Run tests for specific service (auth-service, document-service, service-core)"
            echo "  -j, --jobs <num>       Number of parallel jobs (default: 2)"
            echo "  -v, --verbose          Show test output (--nocapture)"
            echo "  -h, --help            Show this help message"
            echo ""
            echo "Examples:"
            echo "  $0                              # Run all integration tests"
            echo "  $0 -s auth-service              # Run auth-service tests only"
            echo "  $0 -s document-service -v       # Run document-service tests with output"
            exit 0
            ;;
        *)
            echo -e "${RED}Unknown option: $1${NC}"
            echo "Use -h or --help for usage information"
            exit 1
            ;;
    esac
done

# Function to run tests for a service
run_service_tests() {
    local service=$1
    echo -e "${YELLOW}Running integration tests for ${service}...${NC}"

    cd "$service"

    if [ "$VERBOSE" = true ]; then
        cargo test --jobs "$JOBS" -- --nocapture
    else
        cargo test --jobs "$JOBS"
    fi

    local exit_code=$?
    cd ..

    if [ $exit_code -eq 0 ]; then
        echo -e "${GREEN}✓ ${service} tests passed${NC}"
    else
        echo -e "${RED}✗ ${service} tests failed${NC}"
        return $exit_code
    fi
}

# Main execution
echo -e "${GREEN}=== Integration Test Runner ===${NC}"
echo ""

if [ -n "$SERVICE" ]; then
    # Run tests for specific service
    if [ ! -d "$SERVICE" ]; then
        echo -e "${RED}Error: Service directory '${SERVICE}' not found${NC}"
        exit 1
    fi
    run_service_tests "$SERVICE"
else
    # Run tests for all services
    FAILED=0

    for service in auth-service document-service service-core; do
        if [ -d "$service" ]; then
            run_service_tests "$service" || FAILED=1
            echo ""
        fi
    done

    if [ $FAILED -eq 0 ]; then
        echo -e "${GREEN}=== All integration tests passed ===${NC}"
    else
        echo -e "${RED}=== Some integration tests failed ===${NC}"
        exit 1
    fi
fi
