#!/bin/bash
# Stop production stack

set -e

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

echo -e "${YELLOW}Stopping Micros Production Stack${NC}"
echo ""

# Stop and remove services
docker-compose -f docker-compose.prod.yml --env-file .env.prod down --remove-orphans

echo ""
echo -e "${GREEN}Production services stopped and removed!${NC}"
echo ""
echo "All containers including MongoDB and Redis have been removed."
echo ""
echo "To also remove volumes (WARNING: deletes all data):"
echo "  docker-compose -f docker-compose.prod.yml --env-file .env.prod down -v"
