#!/bin/bash
# Start PLG+T Observability Stack

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

echo -e "${GREEN}Starting PLG+T Observability Stack (${ENV})${NC}"
echo ""

docker-compose up -d

echo ""
echo -e "${GREEN}Observability stack started!${NC}"
echo ""
echo "Access points (standard ports):"
echo "  - Prometheus: http://localhost:9090"
echo "  - Loki:       http://localhost:3100"
echo "  - Grafana:    http://localhost:3000 (admin/admin)"
echo "  - Tempo:      http://localhost:3200"
echo ""
echo "Health checks:"
echo "  curl http://localhost:9090/-/healthy  # Prometheus"
echo "  curl http://localhost:3100/ready      # Loki"
echo "  curl http://localhost:3200/ready      # Tempo"
echo "  curl http://localhost:3000/api/health # Grafana"
echo ""
echo "OTLP endpoints for services:"
echo "  - gRPC: http://host.docker.internal:4317"
echo "  - HTTP: http://host.docker.internal:4318"
echo ""
echo "View logs:"
echo "  docker-compose logs -f"
echo ""
echo "Stop stack:"
echo "  ./stop.sh"
