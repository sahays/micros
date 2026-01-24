#!/bin/bash
# Start development stack (PostgreSQL/MongoDB/Redis on host)

set -e

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

# Parse flags
REBUILD_FLAG=""
NO_CACHE_FLAG=""

while [[ $# -gt 0 ]]; do
    case $1 in
        --rebuild)
            REBUILD_FLAG="--build"
            shift
            ;;
        --no-cache)
            NO_CACHE_FLAG="--no-cache"
            shift
            ;;
        *)
            echo -e "${RED}Unknown option: $1${NC}"
            echo "Usage: $0 [--rebuild] [--no-cache]"
            echo "  --rebuild    Rebuild Docker images before starting"
            echo "  --no-cache   Rebuild without using cache (implies --rebuild)"
            exit 1
            ;;
    esac
done

# If --no-cache is set, also set --rebuild
if [ -n "$NO_CACHE_FLAG" ]; then
    REBUILD_FLAG="--build"
fi

echo -e "${GREEN}Starting Micros Development Stack${NC}"
echo "PostgreSQL, MongoDB, and Redis must be running on your host machine"
echo ""

if [ -n "$REBUILD_FLAG" ]; then
    if [ -n "$NO_CACHE_FLAG" ]; then
        echo -e "${YELLOW}Rebuilding all images without cache...${NC}"
    else
        echo -e "${YELLOW}Rebuilding images with cache...${NC}"
    fi
    echo ""
fi

# Check if .env.dev exists
if [ ! -f .env.dev ]; then
    echo -e "${YELLOW}Warning: .env.dev not found${NC}"
    echo "Creating from template..."
    cp .env.example .env.dev
    echo -e "${YELLOW}Please edit .env.dev and set your secrets before continuing${NC}"
    exit 1
fi

# Check if JWT keys exist
if [ ! -f auth-service/keys/private.pem ]; then
    echo -e "${YELLOW}JWT keys not found. Generating...${NC}"
    mkdir -p auth-service/keys
    openssl genrsa -out auth-service/keys/private.pem 2048
    openssl rsa -in auth-service/keys/private.pem -pubout -out auth-service/keys/public.pem
    echo -e "${GREEN}JWT keys generated${NC}"
fi

# Check if PostgreSQL is accessible (auth, ledger, billing, reconciliation, invoicing)
echo "Checking host PostgreSQL connection..."
if nc -z localhost 5432 2>/dev/null; then
    echo -e "${GREEN}✓ PostgreSQL is accessible on port 5432${NC}"
else
    echo -e "${RED}✗ PostgreSQL is not accessible on port 5432${NC}"
    echo "Please start PostgreSQL on your host machine first"
    echo "Required by: auth, ledger, billing, reconciliation, invoicing services"
    exit 1
fi

# Check if MongoDB is accessible (document, notification, payment, genai)
echo "Checking host MongoDB connection..."
if nc -z localhost 27017 2>/dev/null; then
    echo -e "${GREEN}✓ MongoDB is accessible on port 27017${NC}"
else
    echo -e "${RED}✗ MongoDB is not accessible on port 27017${NC}"
    echo "Please start MongoDB on your host machine first"
    echo "Required by: document, notification, payment, genai services"
    exit 1
fi

# Check if Redis is accessible
echo "Checking host Redis connection..."
if nc -z localhost 6379 2>/dev/null; then
    echo -e "${GREEN}✓ Redis is accessible on port 6379${NC}"
else
    echo -e "${RED}✗ Redis is not accessible on port 6379${NC}"
    echo "Please start Redis on your host machine first"
    exit 1
fi

echo ""

# Build the centralized builder image (compiles all binaries once)
BUILDER_EXISTS=$(docker images -q micros-builder 2>/dev/null)

if [ -n "$REBUILD_FLAG" ] || [ -z "$BUILDER_EXISTS" ]; then
    if [ -n "$NO_CACHE_FLAG" ]; then
        ./scripts/build-builder.sh --no-cache
    else
        ./scripts/build-builder.sh
    fi
fi

echo -e "${GREEN}Starting services with Docker Compose...${NC}"
docker-compose -f docker-compose.dev.yml --env-file .env.dev up -d $REBUILD_FLAG $NO_CACHE_FLAG

echo ""
echo -e "${GREEN}Services started!${NC}"
echo ""
echo "Access points (Dev: ports 9000-9014):"
echo "  Health Endpoints:"
echo "    - Auth Service:           http://localhost:9005/health"
echo "    - Document Service:       http://localhost:9007/health"
echo "    - Notification Service:   http://localhost:9008/health"
echo "    - Payment Service:        http://localhost:9009/health"
echo "    - GenAI Service:          http://localhost:9010/health"
echo "    - Ledger Service:         http://localhost:9011/health"
echo "    - Billing Service:        http://localhost:9012/health"
echo "    - Reconciliation Service: http://localhost:9013/health"
echo "    - Invoicing Service:      http://localhost:9014/health"
echo ""
echo "  gRPC Endpoints:"
echo "    - Auth Service:           localhost:50051"
echo "    - Document Service:       localhost:50052"
echo "    - Notification Service:   localhost:50053"
echo "    - Payment Service:        localhost:50054"
echo "    - GenAI Service:          localhost:50055"
echo "    - Ledger Service:         localhost:50056"
echo "    - Billing Service:        localhost:50057"
echo "    - Reconciliation Service: localhost:50058"
echo "    - Invoicing Service:      localhost:50059"
echo ""
echo "  Observability:"
echo "    - Prometheus:           http://localhost:9000"
echo "    - Loki:                 http://localhost:9001"
echo "    - Grafana:              http://localhost:9002 (admin/admin)"
echo "    - Tempo:                http://localhost:9003"
echo ""
echo "Databases (on host machine):"
echo "  - PostgreSQL: localhost:5432  (auth, ledger, billing, reconciliation, invoicing)"
echo "  - MongoDB:    localhost:27017 (document, notification, payment, genai)"
echo "  - Redis:      localhost:6379  (auth, session cache)"
echo ""
echo "View logs:"
echo "  docker-compose -f docker-compose.dev.yml logs -f"
echo "  docker-compose -f docker-compose.dev.yml logs -f reconciliation-service"
echo ""
echo "Stop services:"
echo "  ./scripts/dev-down.sh"
echo ""
echo "Rebuild images:"
echo "  ./scripts/dev-up.sh --rebuild              (use cache)"
echo "  ./scripts/dev-up.sh --rebuild --no-cache   (full rebuild)"
