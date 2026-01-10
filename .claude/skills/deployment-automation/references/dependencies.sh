#!/bin/bash

# Check and install system dependencies (Rocky Linux)
check_dependencies() {
    local deps=(
        "docker-ce"
        "docker-compose-plugin"
        "git"
        "curl"
        "tar"
    )

    local missing=()

    for dep in "${deps[@]}"; do
        if ! rpm -q "$dep" &>/dev/null; then
            missing+=("$dep")
        fi
    done

    if [ ${#missing[@]} -gt 0 ]; then
        echo "Missing dependencies: ${missing[*]}"

        if [ "$MODE" = "full" ]; then
            install_dependencies "${missing[@]}"
        else
            fail "Missing dependencies. Run with --mode=full to install"
        fi
    fi
}

install_dependencies() {
    echo "Installing dependencies: $*"

    # Update package cache
    dnf makecache --refresh

    # Install missing packages
    dnf install -y "$@"
}

# Install Docker on Rocky Linux
install_docker() {
    # Add Docker repo
    dnf config-manager --add-repo https://download.docker.com/linux/centos/docker-ce.repo

    # Install Docker
    dnf install -y docker-ce docker-ce-cli containerd.io docker-compose-plugin

    # Start and enable Docker
    systemctl start docker
    systemctl enable docker
}
