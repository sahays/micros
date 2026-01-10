#!/bin/bash
# Start production stack (everything containerized)

set -e

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

echo -e "${GREEN}Starting Micros Production Stack${NC}"
echo "All services including MongoDB and Redis will run in Docker"
echo ""

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
docker-compose -f docker-compose.prod.yml --env-file .env.prod up -d

echo ""
echo -e "${GREEN}Services started!${NC}"
echo ""
echo "Access points (Prod: ports 10000-10009):"
echo "  - Auth Service:      http://localhost:10005"
echo "  - Secure Frontend:   http://localhost:10006"
echo "  - Document Service:  http://localhost:10007"
echo "  - Prometheus:        http://localhost:10000"
echo "  - Loki:              http://localhost:10001"
echo "  - Grafana:           http://localhost:10002"
echo "  - Tempo:             http://localhost:10003"
echo ""
echo "Databases (containerized):"
echo "  - MongoDB:           localhost:10008"
echo "  - Redis:             localhost:10009"
echo ""
echo "View logs:"
echo "  docker-compose -f docker-compose.prod.yml logs -f"
echo ""
echo "Stop services:"
echo "  ./scripts/prod-down.sh"
