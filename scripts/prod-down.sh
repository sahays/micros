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

# Stop services
docker-compose -f docker-compose.prod.yml down

echo ""
echo -e "${GREEN}Production services stopped!${NC}"
echo ""
echo "All containers including MongoDB and Redis have been stopped."
echo ""
echo "To remove volumes (WARNING: deletes all data):"
echo "  docker-compose -f docker-compose.prod.yml down -v"
