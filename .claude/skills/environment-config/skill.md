---
name: environment-config
description:
  Configure and manage dev and prod environments for safe, error-free deployments. Use when setting up environment
  configuration, managing secrets, or defining deployment practices. Focuses on two-environment strategy.
---

# Environment Configuration

## Two-Environment Strategy

**Dev**: Local development and testing. Permissive, verbose, fast iteration.

**Prod**: Production serving real users. Strict, optimized, safe.

**No staging**: Simplifies workflow. Use feature flags and gradual rollouts for safety.

## Configuration Principles

**Environment variables for all config**: Never hardcode environment-specific values.

**Twelve-factor config**: Store config in environment, not code.

**Fail fast on startup**: Validate all required config exists and is valid before accepting traffic.

**Immutable deployments**: Same artifact (container, binary) runs in both environments with different config.

**Config as code**: Store environment configs in version control (except secrets).

## Required Configuration

**Every service must configure**:

- `ENVIRONMENT`: "dev" or "prod"
- `SERVICE_NAME`: Service identifier
- `SERVICE_VERSION`: Deployment version
- `LOG_LEVEL`: "debug" for dev, "info" for prod
- `PORT`: Service listening port
- Database connection (URL, credentials)
- External service endpoints
- Feature flags

## Configuration Sources

**Priority order (highest to lowest)**:

1. Environment variables (runtime config)
2. Config file for environment (config/prod.yml, config/dev.yml)
3. Default values (for dev only)

**Never use defaults in prod**: All prod config must be explicit.

## Dev Environment

**Characteristics**:

- DEBUG log level
- Verbose error messages with stack traces
- Auto-reload on code changes
- Permissive CORS
- Longer timeouts
- Seed data and fixtures
- Local database
- Mock external services when possible

**Safety**: Dev can break. Optimize for developer productivity.

## Prod Environment

**Characteristics**:

- INFO log level
- Sanitized error messages
- Strict CORS
- Production timeouts
- Real database with backups
- Real external services
- Health checks enabled
- Metrics and monitoring enabled

**Safety**: Prod must never break. Optimize for reliability.

## Secrets Management

**Never commit secrets to version control**: Not even encrypted ones.

**Use secret management tools**:

- Environment variables from secrets manager
- Vault, AWS Secrets Manager, GCP Secret Manager
- Kubernetes secrets

**Dev secrets**: Can use .env file (gitignored). Document required secrets in .env.example.

**Prod secrets**: Always from secrets manager. Rotate regularly.

**Secret rotation**: Design for zero-downtime secret rotation.

## Configuration Validation

**Validate on startup**:

- All required variables are set
- Values are correct type and format
- Database connection works
- Required external services are reachable

**Fail immediately**: If validation fails, crash with clear error message. Don't start partially configured.

**Validation checks**:

- Required variables present
- Port numbers are valid
- URLs are well-formed
- Timeouts are positive numbers
- Enums match allowed values

## Environment Parity

**Keep dev and prod as similar as possible**:

- Same language/framework versions
- Same database type (PostgreSQL in both, not SQLite in dev)
- Same message queue, cache, etc.
- Same runtime environment (containers)

**Acceptable differences**:

- Scale (single instance dev, multiple prod)
- Log levels
- Debug features
- Mock vs real external services

## Deployment Safety

**Pre-deployment checks**:

- All tests pass
- Config validation passes
- Database migrations tested
- Rollback plan ready

**Deployment process**:

1. Deploy new version alongside old (blue-green)
2. Run health checks on new version
3. Gradually shift traffic (canary)
4. Monitor error rates and key metrics
5. Full rollout if healthy, rollback if issues

**Rollback plan**: Always deployable to previous version. Keep N-1 version ready.

**Database migrations**: Backward compatible. Deploy code before schema changes that break old code.

## Feature Flags

**Use for risky changes**: New features, algorithm changes, external integrations.

**Flag types**:

- Environment flags: Enabled in dev, disabled in prod until ready
- Gradual rollout: 1%, 10%, 50%, 100% of users
- Kill switches: Instant disable if issues detected

**Implementation**: Config value or feature flag service (LaunchDarkly, Unleash, custom).

**Cleanup**: Remove flags after full rollout. Don't accumulate flag debt.

## Configuration Drift Prevention

**Single source of truth**: Config repo or secrets manager, not local files.

**Automated deployment**: No manual config changes in prod.

**Audit logging**: Log all config changes with who, what, when.

**Config review**: Treat config changes like code changes. Review and approve.

## Common Pitfalls

**Don't**:

- Hardcode environment-specific values
- Use different datastores in dev vs prod
- Commit secrets to git
- Deploy without validation
- Make manual config changes in prod
- Use default values in prod
- Mix dev and prod credentials

**Do**:

- Validate config on startup
- Document all required config
- Keep environments similar
- Automate deployments
- Version control config (not secrets)
- Test config changes in dev first

## Environment Detection

**Explicit, never implicit**: Set ENVIRONMENT variable explicitly.

**Never infer from**: Hostname, IP address, or other heuristics.

**Single source**: One variable determines environment-specific behavior.

**Type-safe**: Use enum or constant, not string comparison everywhere.

## Configuration Documentation

**Document in code**:

- Required vs optional variables
- Default values (dev only)
- Format and validation rules
- Example values

**Maintain .env.example**: Template showing all required config. Keep updated.

**README**: Document how to configure locally and deploy to prod.
