#!/bin/bash

# Pre-deployment validation
validate_preconditions() {
    # Prod env file must exist
    [ -f /app/config/prod.env ] || fail "prod.env not found"

    # Env file must be readable by deploy user only
    [ "$(stat -c %a /app/config/prod.env)" = "600" ] || fail "prod.env has wrong permissions"

    # Validate all required vars in prod.env
    source /app/config/prod.env
    : "${DATABASE_URL:?DATABASE_URL not set in prod.env}"
    : "${API_KEY:?API_KEY not set in prod.env}"

    # Docker must be running
    systemctl is-active docker || fail "Docker is not running"

    # Disk space check (require 5GB free)
    available=$(df /app --output=avail | tail -1)
    [ "$available" -gt 5242880 ] || fail "Insufficient disk space"
}

# Environment file validation
validate_env_file() {
    local env_file="/app/config/prod.env"

    # File must exist
    [ -f "$env_file" ] || fail "$env_file does not exist"

    # Must be owned by deploy user
    [ "$(stat -c %U "$env_file")" = "$USER" ] || fail "$env_file wrong owner"

    # Must be 600 permissions (owner read/write only)
    [ "$(stat -c %a "$env_file")" = "600" ] || fail "$env_file must be 600"

    # Source and check required variables
    set -a
    source "$env_file"
    set +a

    # Validate required vars exist and non-empty
    local required_vars=(
        "ENVIRONMENT"
        "DATABASE_URL"
        "API_KEY"
        "SERVICE_PORT"
    )

    for var in "${required_vars[@]}"; do
        if [ -z "${!var:-}" ]; then
            fail "Required variable $var not set in $env_file"
        fi
    done

    # Environment must be "prod"
    [ "$ENVIRONMENT" = "prod" ] || fail "ENVIRONMENT must be 'prod' in prod.env"
}
