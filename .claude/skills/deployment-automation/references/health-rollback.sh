#!/bin/bash

# Wait for service to be healthy
wait_for_health() {
    local max_attempts=30
    local attempt=1
    local health_url="http://localhost:${SERVICE_PORT}/health"

    echo "Waiting for service health check..."

    while [ $attempt -le $max_attempts ]; do
        if curl -sf "$health_url" >/dev/null 2>&1; then
            echo "Service is healthy"
            return 0
        fi

        echo "Attempt $attempt/$max_attempts failed, waiting..."
        sleep 2
        attempt=$((attempt + 1))
    done

    echo "Health check failed, rolling back"
    rollback
    exit 1
}

# Simple rollback to previous version
rollback() {
    echo "Rolling back to previous version"

    # Stop broken version
    cd /app/current
    docker compose down || true

    # Find previous release
    PREVIOUS=$(ls -1dt /app/releases/* | sed -n 2p)

    if [ -z "$PREVIOUS" ]; then
        fail "No previous version to rollback to"
    fi

    # Revert symlink
    ln -sfn "$PREVIOUS" /app/current

    # Start previous version
    cd /app/current
    docker compose up -d

    echo "Rollback complete"
}
