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

# Stop services
docker-compose -f docker-compose.dev.yml down

echo ""
echo -e "${GREEN}Development services stopped!${NC}"
echo ""
echo "MongoDB and Redis on your host machine are still running."
echo "To stop them manually:"
echo "  - MongoDB: Use your system's MongoDB service manager"
echo "  - Redis: Use your system's Redis service manager"
