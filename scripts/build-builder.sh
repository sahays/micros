#!/bin/bash
# Build the centralized builder image for all microservices
#
# This compiles all service binaries in a single Docker build,
# avoiding memory exhaustion from parallel builds of the workspace.
#
# Usage:
#   ./scripts/build-builder.sh           # Build with cache
#   ./scripts/build-builder.sh --no-cache # Full rebuild

set -e

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

# Parse flags
NO_CACHE_FLAG=""

while [[ $# -gt 0 ]]; do
    case $1 in
        --no-cache)
            NO_CACHE_FLAG="--no-cache"
            shift
            ;;
        *)
            echo "Unknown option: $1"
            echo "Usage: $0 [--no-cache]"
            exit 1
            ;;
    esac
done

echo -e "${GREEN}Building micros-builder image${NC}"
echo "This compiles all 9 service binaries in a single build."
echo ""

if [ -n "$NO_CACHE_FLAG" ]; then
    echo -e "${YELLOW}Building without cache (full rebuild)...${NC}"
    docker build --no-cache -f Dockerfile.builder -t micros-builder .
else
    echo "Building with cache..."
    docker build -f Dockerfile.builder -t micros-builder .
fi

echo ""
echo -e "${GREEN}✓ micros-builder image ready${NC}"
echo ""
echo "Next steps:"
echo "  - Run ./scripts/dev-up.sh to start development stack"
echo "  - Run ./scripts/prod-up.sh to start production stack"
echo ""
echo "The service Dockerfiles will COPY binaries from micros-builder."
