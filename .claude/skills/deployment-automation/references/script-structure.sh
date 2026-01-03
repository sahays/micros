#!/bin/bash
set -euo pipefail
IFS=$'\n\t'

MODE="code"  # Default to code-only

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --mode=*)
            MODE="${1#*=}"
            shift
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

# Validate mode
if [[ "$MODE" != "full" && "$MODE" != "code" ]]; then
    echo "Invalid mode: $MODE. Use --mode=full or --mode=code"
    exit 1
fi

# Logging
LOG_FILE="/app/logs/deploy-$(date +%Y%m%d).log"

log() {
    local msg="[$(date +'%Y-%m-%d %H:%M:%S')] $*"
    echo "$msg" | tee -a "$LOG_FILE"
}

# Error handling
trap 'log "ERROR at line $LINENO: $BASH_COMMAND"' ERR

fail() {
    echo "ERROR: $1" >&2
    exit 1
}

log "Starting deployment in $MODE mode"
