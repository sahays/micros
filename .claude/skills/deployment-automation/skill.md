---
name: deployment-automation
description:
  Create robust deployment scripts for Docker containers on self-managed Rocky Linux servers. Use when writing
  deployment automation, continuous delivery scripts, or infrastructure-as-code for containerized applications.
---

- Core Principles
  - Fail fast with clear errors: use set -euo pipefail
  - Atomic deployments: deploy to timestamped directories, use symlink swaps for zero-downtime
  - Keep 2-3 previous releases for instant rollback
  - Single source of truth for config: /app/config/prod.env with 600 permissions
  - Never commit config to git, validate before every deployment
  - Health-check driven: poll service health endpoints after deployment
  - Auto-rollback on failure, never leave broken deployments running

- Deployment Workflow
  - Validate prerequisites: check environment file exists with correct permissions, required variables set, Docker running, sufficient disk space
  - Prepare release: create timestamped directory, extract artifacts, copy validated config
  - Deploy containers: pull images, stop current containers gracefully, update symlink atomically, start new containers
  - Verify health: poll health endpoint with retries (30 attempts × 2s interval)
  - Handle failures: rollback to previous release if health checks fail
  - Cleanup: remove old releases after successful deployment, keep last 2-3

- Deployment Modes
  - Full deployment: install dependencies (docker-ce, docker-compose-plugin), create directory structure, setup Docker network, install systemd service, deploy containers
  - Code-only deployment: skip infrastructure setup, deploy containers only, fail if prerequisites missing
  - Control with --mode=full or --mode=code flag

- Directory Structure
  - /app/config/prod.env: protected environment file (600 permissions)
  - /app/current: symlink to active release
  - /app/data: persistent volumes
  - /app/logs: deployment and application logs
  - /app/releases/TIMESTAMP: timestamped release directories

- Best Practices
  - Trap errors with line numbers for debugging
  - Log all deployment actions to timestamped files
  - Use systemd for auto-start and graceful shutdown
  - Set clear exit codes: 0 (success), 1 (validation), 2 (deployment), 3 (health check)
  - Run as deploy user, never root
  - Structure scripts with functions for testability
  - Validate: check file existence, permissions, required env vars, disk space before deployment
  - Rollback: stop broken containers, revert symlink, restart previous version
  - Health checks: use Docker healthcheck in compose files, poll HTTP endpoints
  - Cleanup: remove old releases only after new deployment proven stable
