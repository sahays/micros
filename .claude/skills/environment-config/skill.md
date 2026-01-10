---
name: environment-config
description:
  Configure and manage dev and prod environments for safe, error-free deployments. Use when setting up environment
  configuration, managing secrets, or defining deployment practices. Focuses on two-environment strategy.
---

- Two-Environment Strategy
  - Dev: local development and testing, permissive, verbose, fast iteration
  - Prod: production serving real users, strict, optimized, safe
  - No staging: use feature flags and gradual rollouts for safety

- Configuration Principles
  - Environment variables for all config, never hardcode environment-specific values
  - Twelve-factor config: store config in environment, not code
  - Fail fast on startup: validate all required config exists and is valid before accepting traffic
  - Immutable deployments: same artifact runs in both environments with different config
  - Config as code: version control configs (except secrets)

- Required Configuration
  - ENVIRONMENT: dev or prod
  - SERVICE_NAME: service identifier
  - SERVICE_VERSION: deployment version
  - LOG_LEVEL: debug for dev, info for prod
  - PORT: service listening port
  - Database connection: URL, credentials
  - External service endpoints
  - Feature flags

- Configuration Sources (Priority Order)
  - Environment variables (runtime config)
  - Config file for environment (config/prod.yml, config/dev.yml)
  - Default values (dev only, never use defaults in prod)

- Dev Environment
  - DEBUG log level
  - Verbose error messages with stack traces
  - Auto-reload on code changes
  - Permissive CORS
  - Longer timeouts
  - Seed data and fixtures
  - Local database
  - Mock external services when possible

- Prod Environment
  - INFO log level
  - Sanitized error messages
  - Strict CORS
  - Production timeouts
  - Real database with backups
  - Real external services
  - Health checks enabled
  - Metrics and monitoring enabled

- Secrets Management
  - Never commit secrets to version control (not even encrypted)
  - Use Vault, AWS Secrets Manager, GCP Secret Manager, Kubernetes secrets
  - Dev secrets: can use .env file (gitignored), document in .env.example
  - Prod secrets: always from secrets manager, rotate regularly
  - Design for zero-downtime secret rotation

- Configuration Validation
  - Validate on startup: all required variables set, correct type and format, database connection works, external services reachable
  - Fail immediately: crash with clear error if validation fails
  - Check: required variables present, port numbers valid, URLs well-formed, timeouts positive, enums match allowed values

- Environment Parity
  - Keep dev and prod similar: same language/framework versions, same database type, same runtime environment (containers)
  - Acceptable differences: scale, log levels, debug features, mock vs real external services

- Deployment Safety
  - Pre-deployment: all tests pass, config validation passes, database migrations tested, rollback plan ready
  - Deployment: blue-green deploy, health checks on new version, gradual traffic shift (canary), monitor error rates, rollback if issues
  - Rollback plan: always deployable to previous version, keep N-1 version ready
  - Database migrations: backward compatible, deploy code before schema changes that break old code

- Feature Flags
  - Use for risky changes: new features, algorithm changes, external integrations
  - Types: environment flags (enabled in dev, disabled in prod), gradual rollout (1%, 10%, 50%, 100%), kill switches
  - Implementation: config value or feature flag service (LaunchDarkly, Unleash, custom)
  - Cleanup: remove flags after full rollout

- Configuration Drift Prevention
  - Single source of truth: config repo or secrets manager
  - Automated deployment: no manual config changes in prod
  - Audit logging: log all config changes with who, what, when
  - Config review: treat config changes like code changes

- Best Practices
  - Validate config on startup
  - Document all required config
  - Keep environments similar
  - Automate deployments
  - Version control config (not secrets)
  - Test config changes in dev first
  - Never hardcode environment-specific values
  - Never use different datastores in dev vs prod
  - Never commit secrets to git
  - Never deploy without validation
  - Never make manual config changes in prod
  - Never use default values in prod
  - Explicit environment detection: set ENVIRONMENT variable explicitly, never infer from hostname or IP
  - Document in code: required vs optional variables, default values, format and validation rules, example values
  - Maintain .env.example: template showing all required config
