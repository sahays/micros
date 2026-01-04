#!/bin/bash
set -euo pipefail

# Configuration
APP_NAME="auth-service"
BASE_DIR="/app"
CONFIG_DIR="${BASE_DIR}/config"
RELEASES_DIR="${BASE_DIR}/releases"
LOGS_DIR="${BASE_DIR}/logs"
CURRENT_SYMLINK="${BASE_DIR}/current"
PROD_ENV="${CONFIG_DIR}/prod.env"
TIMESTAMP=$(date +%Y%m%d-%H%M%S)
NEW_RELEASE_DIR="${RELEASES_DIR}/${TIMESTAMP}"

# Logging
LOG_FILE="${LOGS_DIR}/deploy-${TIMESTAMP}.log"
exec > >(tee -a "${LOG_FILE}") 2>&1

log() {
    echo "[$(date +'%Y-%m-%d %H:%M:%S')] $1"
}

error() {
    echo "[$(date +'%Y-%m-%d %H:%M:%S')] ERROR: $1" >&2
    exit 1
}

# 1. Validate prerequisites
log "Validating prerequisites..."

if [[ ! -f "${PROD_ENV}" ]]; then
    error "Production environment file ${PROD_ENV} not found."
fi

# Check permissions
PERMS=$(stat -c "%a" "${PROD_ENV}")
if [[ "${PERMS}" != "600" ]]; then
    log "Fixing permissions on ${PROD_ENV} to 600..."
    chmod 600 "${PROD_ENV}"
fi

# Basic validation of required env vars in prod.env
REQUIRED_VARS=(
    "ENVIRONMENT"
    "MONGODB_URI"
    "MONGODB_DATABASE"
    "REDIS_URL"
    "JWT_PRIVATE_KEY_PATH"
    "JWT_PUBLIC_KEY_PATH"
    "GOOGLE_CLIENT_ID"
    "GOOGLE_CLIENT_SECRET"
    "GMAIL_USER"
    "GMAIL_APP_PASSWORD"
    "ADMIN_API_KEY"
)

for VAR in "${REQUIRED_VARS[@]}"; do
    if ! grep -q "^${VAR}=" "${PROD_ENV}"; then
        error "Required environment variable ${VAR} is missing from ${PROD_ENV}."
    fi
done

# Ensure directories exist
mkdir -p "${RELEASES_DIR}" "${LOGS_DIR}" "${BASE_DIR}/data"

# 2. Prepare release
log "Preparing release ${TIMESTAMP}..."
mkdir -p "${NEW_RELEASE_DIR}"

# 3. Build/Pull image
log "Building Docker image..."
# In a real CI/CD, we might pull here. For this script, we build.
docker build -t "${APP_NAME}:latest" -t "${APP_NAME}:${TIMESTAMP}" ./auth-service

# 4. Stop current container
log "Stopping current container (if any)..."
docker stop "${APP_NAME}" 2>/dev/null || true
docker rm "${APP_NAME}" 2>/dev/null || true

# 5. Atomic symlink swap
log "Updating symlink..."
ln -sfn "releases/${TIMESTAMP}" "${CURRENT_SYMLINK}"

# 6. Start new container
log "Starting new container..."
docker run -d \
    --name "${APP_NAME}" \
    --restart unless-stopped \
    --env-file "${PROD_ENV}" \
    -p 3000:3000 \
    -v "${BASE_DIR}/data:/app/data" \
    "${APP_NAME}:${TIMESTAMP}"

# 7. Verify health
log "Verifying health..."
MAX_ATTEMPTS=30
INTERVAL=2
HEALTHY=false

for ((i=1; i<=MAX_ATTEMPTS; i++)); do
    if curl -s -f http://localhost:3000/health > /dev/null; then
        log "Service is healthy!"
        HEALTHY=true
        break
    fi
    log "Attempt $i/$MAX_ATTEMPTS: Service not healthy yet, waiting ${INTERVAL}s..."
    sleep ${INTERVAL}
done

if [[ "${HEALTHY}" == "false" ]]; then
    log "Health check failed! Rolling back..."
    
    # 8. Rollback
    if [[ -L "${CURRENT_SYMLINK}" ]]; then
        # Find previous release (second most recent in releases dir)
        PREV_RELEASE=$(ls -1dt "${RELEASES_DIR}"/*/ | sed -n '2p' | xargs basename 2>/dev/null || true)
        
        if [[ -n "${PREV_RELEASE}" ]]; then
            log "Rolling back to release ${PREV_RELEASE}..."
            docker stop "${APP_NAME}" || true
            docker rm "${APP_NAME}" || true
            
            ln -sfn "releases/${PREV_RELEASE}" "${CURRENT_SYMLINK}"
            
            # Start previous image (we tagged it with timestamp)
            docker run -d \
                --name "${APP_NAME}" \
                --restart unless-stopped \
                --env-file "${PROD_ENV}" \
                -p 3000:3000 \
                -v "${BASE_DIR}/data:/app/data" \
                "${APP_NAME}:${PREV_RELEASE}"
            
            log "Rollback complete."
        else
            error "No previous release found to rollback to."
        fi
    fi
    error "Deployment failed health checks."
fi

# 9. Cleanup old releases (keep last 3)
log "Cleaning up old releases..."
ls -1dt "${RELEASES_DIR}"/*/ | tail -n +4 | xargs rm -rf

log "Deployment successful!"
