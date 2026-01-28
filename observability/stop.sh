#!/bin/bash
# Stop PLG+T Observability Stack

set -e

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# Default to dev environment
ENV="dev"
if [[ "$1" == "--prod" ]]; then
    ENV="prod"
fi

export COMPOSE_PROJECT_NAME="observability-${ENV}"

echo -e "${YELLOW}Stopping PLG+T Observability Stack (${ENV})${NC}"
echo ""

docker-compose down --remove-orphans

echo ""
echo -e "${GREEN}Observability stack stopped!${NC}"
echo ""
echo "To also remove volumes (WARNING: deletes all observability data):"
echo "  cd $SCRIPT_DIR && COMPOSE_PROJECT_NAME=observability-${ENV} docker-compose down -v"
