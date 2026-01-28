#!/bin/bash
# Stop development stack

set -e

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

echo -e "${YELLOW}Stopping Micros Development Stack${NC}"
echo ""

# Stop and remove services
docker-compose -f docker-compose.dev.yml --env-file .env.dev down --remove-orphans

echo ""
echo -e "${GREEN}Development services stopped and removed!${NC}"
echo ""
echo "Still running on host machine:"
echo "  - PostgreSQL (port 5432)"
echo "  - MongoDB (port 27017)"
echo "  - Redis (port 6379)"
echo "  - Observability stack (if started): cd observability && ./stop.sh"
