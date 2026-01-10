#!/bin/bash
set -e

# API Test Runner using Newman (Postman CLI)
# Runs API tests against auth-service and document-service

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Default values
ENVIRONMENT="dev"
COLLECTION=""
VERBOSE=false
INSTALL_NEWMAN=false

# Function to check if Newman is installed
check_newman() {
    if ! command -v newman &> /dev/null; then
        echo -e "${YELLOW}Newman is not installed${NC}"
        echo ""
        echo "Newman is required to run API tests."
        echo "Install with: npm install -g newman newman-reporter-htmlextra"
        echo ""
        read -p "Would you like to install Newman now? (y/N) " -n 1 -r
        echo
        if [[ $REPLY =~ ^[Yy]$ ]]; then
            echo -e "${BLUE}Installing Newman...${NC}"
            npm install -g newman newman-reporter-htmlextra
            echo -e "${GREEN}Newman installed successfully${NC}"
        else
            echo -e "${RED}Newman is required to run API tests. Exiting.${NC}"
            exit 1
        fi
    fi
}

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        -e|--environment)
            ENVIRONMENT="$2"
            shift 2
            ;;
        -c|--collection)
            COLLECTION="$2"
            shift 2
            ;;
        -v|--verbose)
            VERBOSE=true
            shift
            ;;
        --install-newman)
            INSTALL_NEWMAN=true
            shift
            ;;
        -h|--help)
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  -e, --environment <env>   Environment to test (dev|prod, default: dev)"
            echo "  -c, --collection <name>   Specific collection to run (auth-service, document-service)"
            echo "  -v, --verbose             Show detailed Newman output"
            echo "  --install-newman          Install Newman if not present"
            echo "  -h, --help                Show this help message"
            echo ""
            echo "Examples:"
            echo "  $0                                  # Run all collections in dev environment"
            echo "  $0 -e prod                          # Run all collections in prod environment"
            echo "  $0 -c auth-service                  # Run auth-service collection only"
            echo "  $0 -c auth-service -e prod -v       # Run auth-service in prod with verbose output"
            echo ""
            echo "Prerequisites:"
            echo "  - Newman CLI: npm install -g newman newman-reporter-htmlextra"
            echo "  - Services must be running on configured ports"
            exit 0
            ;;
        *)
            echo -e "${RED}Unknown option: $1${NC}"
            echo "Use -h or --help for usage information"
            exit 1
            ;;
    esac
done

# Check Newman installation
check_newman

# Set paths
COLLECTIONS_DIR="tests/api/collections"
ENVIRONMENTS_DIR="tests/api/environments"
REPORTS_DIR="tests/api/reports"

# Create reports directory
mkdir -p "$REPORTS_DIR"

# Determine environment file
ENV_FILE="$ENVIRONMENTS_DIR/${ENVIRONMENT}.postman_environment.json"
if [ ! -f "$ENV_FILE" ]; then
    echo -e "${RED}Error: Environment file not found: $ENV_FILE${NC}"
    exit 1
fi

# Function to run a collection
run_collection() {
    local collection_name=$1
    local collection_file="$COLLECTIONS_DIR/${collection_name}.postman_collection.json"

    if [ ! -f "$collection_file" ]; then
        echo -e "${RED}Error: Collection not found: $collection_file${NC}"
        return 1
    fi

    echo -e "${BLUE}Running ${collection_name} tests in ${ENVIRONMENT} environment...${NC}"

    local newman_args=(
        run "$collection_file"
        -e "$ENV_FILE"
        --reporters cli,htmlextra
        --reporter-htmlextra-export "$REPORTS_DIR/${collection_name}-${ENVIRONMENT}-report.html"
    )

    if [ "$VERBOSE" = false ]; then
        newman_args+=(--reporter-cli-no-assertions --reporter-cli-no-console)
    fi

    if newman "${newman_args[@]}"; then
        echo -e "${GREEN}✓ ${collection_name} tests passed${NC}"
        echo -e "${BLUE}Report: $REPORTS_DIR/${collection_name}-${ENVIRONMENT}-report.html${NC}"
        return 0
    else
        echo -e "${RED}✗ ${collection_name} tests failed${NC}"
        echo -e "${BLUE}Report: $REPORTS_DIR/${collection_name}-${ENVIRONMENT}-report.html${NC}"
        return 1
    fi
}

# Main execution
echo -e "${GREEN}=== API Test Runner (Newman) ===${NC}"
echo -e "Environment: ${YELLOW}${ENVIRONMENT}${NC}"
echo ""

# Check if services are running
echo -e "${YELLOW}Checking if services are running...${NC}"
if [ "$ENVIRONMENT" = "dev" ]; then
    AUTH_PORT=9005
    DOC_PORT=9007
else
    AUTH_PORT=10005
    DOC_PORT=10007
fi

if ! curl -s "http://localhost:${AUTH_PORT}/health" > /dev/null 2>&1; then
    echo -e "${RED}Warning: auth-service not responding on port ${AUTH_PORT}${NC}"
    echo -e "${YELLOW}Make sure services are running before testing${NC}"
fi

echo ""

if [ -n "$COLLECTION" ]; then
    # Run specific collection
    run_collection "$COLLECTION"
    exit_code=$?
else
    # Run all collections
    FAILED=0

    for collection in auth-service; do
        run_collection "$collection" || FAILED=1
        echo ""
    done

    if [ $FAILED -eq 0 ]; then
        echo -e "${GREEN}=== All API tests passed ===${NC}"
        exit_code=0
    else
        echo -e "${RED}=== Some API tests failed ===${NC}"
        exit_code=1
    fi
fi

echo ""
echo -e "${BLUE}View detailed HTML reports in: $REPORTS_DIR${NC}"
exit $exit_code
