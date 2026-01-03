---
name: deployment-automation
description:
  Create robust deployment scripts for Docker containers on self-managed Rocky Linux servers. Use when writing
  deployment automation, continuous delivery scripts, or infrastructure-as-code for containerized applications.
---

# Deployment Automation

## Core Principles

**Fail fast with clear errors**: Use `set -euo pipefail`. Exit immediately on validation or deployment failures. Never
continue with partial state.

**Atomic deployments**: Deploy to timestamped directories, use symlink swaps for zero-downtime releases. Keep 2-3
previous releases for instant rollback.

**Single source of truth for config**: One production environment file (`/app/config/prod.env`) with strict permissions
(600). Never commit to git. Validate before every deployment.

**Health-check driven**: Poll service health endpoints after deployment. Auto-rollback on failure. Never leave broken
deployments running.

## Deployment Workflow

1. **Validate prerequisites**: Check environment file exists with correct permissions, required variables are set,
   Docker is running, sufficient disk space available
2. **Prepare release**: Create timestamped directory, extract artifacts, copy validated config into release
3. **Deploy containers**: Pull images, stop current containers gracefully, update symlink atomically, start new
   containers
4. **Verify health**: Poll health endpoint with retries (e.g., 30 attempts × 2s interval)
5. **Handle failures**: Rollback to previous release if health checks fail
6. **Cleanup**: Remove old releases after successful deployment, keeping last 2-3 for rollback

## Deployment Modes

**Full deployment** (first-time setup): Install dependencies (docker-ce, docker-compose-plugin), create directory
structure (`/app/{config,data,logs,releases}`), setup Docker network, install systemd service, then deploy containers.

**Code-only deployment** (updates): Skip infrastructure setup, deploy containers only. Fail if prerequisites missing.

Control behavior with `--mode=full` or `--mode=code` flag.

## Key Practices

- Trap errors with line numbers for debugging
- Log all deployment actions to timestamped files
- Use systemd for auto-start and graceful shutdown
- Set clear exit codes: 0 (success), 1 (validation), 2 (deployment), 3 (health check)
- Run as deploy user, never root
- Structure scripts with functions over inline code for testability

## Directory Structure

```
/app/
├── config/prod.env              # Protected environment file (600 perms)
├── current -> releases/TIMESTAMP/  # Symlink to active release
├── data/                        # Persistent volumes
├── logs/                        # Deployment and application logs
└── releases/
    ├── 20231215-143022/         # Previous (for rollback)
    └── 20231215-150033/         # Current active
```

## Common Patterns

**Validation**: Check file existence, permissions, required env vars, disk space before any deployment

**Rollback**: Stop broken containers, revert symlink to previous release, restart previous version

**Health checks**: Use Docker healthcheck in compose files, poll HTTP endpoints in deployment scripts

**Cleanup**: Remove old releases only after new deployment proven stable via health checks
