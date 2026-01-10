#!/bin/bash

# Full deployment - infrastructure setup
full_deployment() {
    echo "Running full deployment"

    # Install dependencies
    check_dependencies

    # Create directory structure
    mkdir -p /app/{config,data,logs,releases}

    # Set ownership
    chown -R deploy:deploy /app

    # Setup Docker network if not exists
    docker network inspect app-network &>/dev/null || \
        docker network create app-network

    # Setup systemd service for Docker Compose
    install_systemd_service

    # Continue with code deployment
    code_deployment
}

# Code-only deployment - Docker containers
code_deployment() {
    echo "Deploying code"

    # Create release directory with timestamp
    RELEASE_DIR="/app/releases/$(date +%Y%m%d-%H%M%S)"
    mkdir -p "$RELEASE_DIR"

    # Extract release artifact
    tar -xzf /tmp/release.tar.gz -C "$RELEASE_DIR"

    # Copy prod.env into release
    cp /app/config/prod.env "$RELEASE_DIR/.env"

    # Pull Docker images
    cd "$RELEASE_DIR"
    docker compose pull

    # Stop current containers gracefully
    if [ -L /app/current ]; then
        echo "Stopping current version"
        cd /app/current
        docker compose down --timeout 30 || true
    fi

    # Update current symlink
    ln -sfn "$RELEASE_DIR" /app/current

    # Start new containers
    cd /app/current
    docker compose up -d

    # Wait for health check
    wait_for_health

    # Cleanup old releases (keep last 3)
    cleanup_old_releases
}

# Cleanup old releases
cleanup_old_releases() {
    echo "Cleaning up old releases"

    # Keep last 3 releases
    ls -1dt /app/releases/* | tail -n +4 | while read -r dir; do
        echo "Removing old release: $dir"
        rm -rf "$dir"
    done
}
