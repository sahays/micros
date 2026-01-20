#!/bin/bash
# Start production stack (everything containerized)

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

echo -e "${GREEN}Starting Micros Production Stack${NC}"
echo "All services including PostgreSQL, MongoDB, and Redis will run in Docker"
echo ""

if [ -n "$REBUILD_FLAG" ]; then
    if [ -n "$NO_CACHE_FLAG" ]; then
        echo -e "${YELLOW}Rebuilding all images without cache...${NC}"
    else
        echo -e "${YELLOW}Rebuilding images with cache...${NC}"
    fi
    echo ""
fi

# Check if .env.prod exists
if [ ! -f .env.prod ]; then
    echo -e "${YELLOW}Warning: .env.prod not found${NC}"
    echo "Creating from template..."
    cp .env.example .env.prod
    echo -e "${YELLOW}Please edit .env.prod and set your secrets before continuing${NC}"
    echo "Important: Update all CHANGE_THIS values in .env.prod"
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

echo ""
echo -e "${GREEN}Starting services with Docker Compose...${NC}"
docker-compose -f docker-compose.prod.yml --env-file .env.prod up -d $REBUILD_FLAG $NO_CACHE_FLAG

echo ""
echo -e "${GREEN}Services started!${NC}"
echo ""
echo "Access points (Prod: ports 10000-10010):"
echo "  Health Endpoints:"
echo "    - Auth Service:         http://localhost:10005/health"
echo "    - Secure Frontend:      http://localhost:10006"
echo "    - Document Service:     http://localhost:10007/health"
echo "    - Notification Service: http://localhost:10008/health"
echo "    - Payment Service:      http://localhost:10009/health"
echo ""
echo "  gRPC Endpoints:"
echo "    - Auth Service:         localhost:50051"
echo "    - Document Service:     localhost:50052"
echo "    - Notification Service: localhost:50053"
echo "    - Payment Service:      localhost:50054"
echo ""
echo "  Observability:"
echo "    - Prometheus:           http://localhost:10000"
echo "    - Loki:                 http://localhost:10001"
echo "    - Grafana:              http://localhost:10002"
echo "    - Tempo:                http://localhost:10003"
echo ""
echo "Databases (containerized):"
echo "  - PostgreSQL:          localhost:10010 (auth-service)"
echo "  - MongoDB:             localhost:10008 (document/notification/payment)"
echo "  - Redis:               localhost:10009 (all services)"
echo ""
echo "View logs:"
echo "  docker-compose -f docker-compose.prod.yml logs -f"
echo ""
echo "Stop services:"
echo "  ./scripts/prod-down.sh"
echo ""
echo "Rebuild images:"
echo "  ./scripts/prod-up.sh --rebuild         (use cache)"
echo "  ./scripts/prod-up.sh --rebuild --no-cache  (full rebuild)"
