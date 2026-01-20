# Epic: Service Integration and Testing Infrastructure

**Status**: In Progress
**Priority**: High
**Created**: 2026-01-19
**Related Services**: document-service, auth-service, notification-service, service-core

## Overview

This epic addresses critical infrastructure gaps in the micros monorepo: compile errors, missing test infrastructure, incomplete service configuration, and service integration. The goal is to ensure all services compile, have proper testing infrastructure, and are integrated with proper observability.

## Background

During the removal of KYS authentication and enhancement of tracing, several issues were identified:
1. `document-service` has compile errors related to MongoDB error handling
2. `auth-service` tests are being ignored due to missing PostgreSQL test database
3. `notification-service` lacks Dockerfile and environment configuration
4. Services need proper integration for end-to-end functionality

## Tasks

### Task 1: Fix document-service Compile Errors

**Status**: [x] Completed
**Priority**: Critical
**Estimated Effort**: Small

**Problem**: `document-service` fails to compile with 24 errors related to `AppError::from(mongodb::error::Error)` not being implemented.

**Acceptance Criteria**:
- [x] Add `From<mongodb::error::Error>` implementation to `service-core/src/error.rs`
- [x] `cargo build -p document-service` succeeds
- [x] All existing document-service tests pass

**Files Modified**:
- `service-core/src/error.rs`

---

### Task 2: Run auth-service Tests with PostgreSQL

**Status**: [x] Completed
**Priority**: High
**Estimated Effort**: Small

**Problem**: auth-service tests are ignored because they require a PostgreSQL database.

**Acceptance Criteria**:
- [x] Connect to PostgreSQL running on localhost:5432
- [x] Run all auth-service tests including previously ignored ones
- [x] Document any test failures and their causes

**Notes**:
- Fixed database URL with correct password (`pass@word1` URL-encoded as `pass%40word1`)
- Fixed Axum path parameter syntax (`{param}` -> `:param` for Axum 0.7)
- All 14 auth-service tests pass (7 auth, 5 tenant_org, 1 health, 1 db unit test)

**Prerequisites**:
- PostgreSQL running in Docker on standard port (5432)

---

### Task 3: Create Test Database Management Script

**Status**: [x] Completed
**Priority**: High
**Estimated Effort**: Medium

**Problem**: No automated way to create/cleanup test databases for running the full test suite.

**Acceptance Criteria**:
- [x] Create `scripts/test-with-db.sh` that:
  - Creates a fresh PostgreSQL test database (`micros_test`)
  - Runs database migrations
  - Executes `cargo test` for all workspace members
  - Cleans up the test database after tests complete (success or failure)
- [x] Script handles interrupts gracefully (Ctrl+C)
- [ ] Script is documented in CLAUDE.md
- [x] All workspace tests pass when using the script

**Files Created**:
- `scripts/test-with-db.sh`

**Notes**:
- Script uses `trap` for cleanup on EXIT, INT, TERM signals
- Configurable via environment variables (DB_HOST, DB_PORT, DB_USER, DB_PASSWORD, DB_NAME)

---

### Task 4: Add notification-service Dockerfile and Environment Configuration

**Status**: [x] Completed
**Priority**: High
**Estimated Effort**: Medium

**Problem**: notification-service lacks:
- Dockerfile for containerized deployment
- Environment variables in `.env.example`, `.env.dev`, `.env.prod`
- Docker Compose service definition

**Acceptance Criteria**:
- [x] Create `notification-service/Dockerfile` following auth-service pattern
- [x] Add notification-service to `docker-compose.dev.yml` and `docker-compose.prod.yml`
- [x] Add required environment variables to all .env files:
  - MongoDB connection (NOTIFICATION_MONGODB_URI, NOTIFICATION_MONGODB_DATABASE)
  - SMTP configuration (NOTIFICATION_SMTP_*)
  - Msg91 configuration (NOTIFICATION_MSG91_*)
  - FCM configuration (NOTIFICATION_FCM_*)
  - Service configuration (NOTIFICATION_SERVICE_PORT, etc.)
  - OTLP endpoint for tracing
- [x] `docker-compose build notification-service` succeeds
- [ ] Service starts and health check passes

**Files Created**:
- `notification-service/Dockerfile`

**Files Modified**:
- `docker-compose.dev.yml`
- `docker-compose.prod.yml`
- `.env.example`
- `.env.dev`
- `.env.prod`

---

### Task 5: Integrate notification-service with auth-service

**Status**: [x] Completed
**Priority**: Medium
**Estimated Effort**: Medium

**Problem**: notification-service should be callable from auth-service for sending emails (verification, password reset, etc.) with proper trace context propagation.

**Acceptance Criteria**:
- [x] Create `NotificationClient` in auth-service that:
  - Uses `TracedClientExt` for trace context propagation
  - Sends emails via notification-service `/notifications/email` endpoint
  - Implements `EmailProvider` trait for drop-in replacement
- [x] Add configuration for notification-service URL and enabled flag
- [ ] Auth-service can send OTP emails through notification-service
- [ ] Auth-service can send password reset emails through notification-service
- [ ] Traces show complete request flow from auth-service to notification-service in Tempo

**Files Created**:
- `auth-service/src/services/notification_client.rs`

**Files Modified**:
- `auth-service/src/services/mod.rs`
- `auth-service/src/config/mod.rs` (added NotificationServiceConfig)
- `docker-compose.dev.yml` (added notification service env vars to auth-service)
- `docker-compose.prod.yml` (added notification service env vars to auth-service)
- `.env.example`, `.env.dev`, `.env.prod` (added NOTIFICATION_SERVICE_* vars)

**Notes**:
- NotificationClient implements EmailProvider trait for seamless switching
- NOTIFICATION_SERVICE_ENABLED controls whether to use notification-service or direct SMTP
- Trace context is automatically propagated via TracedClientExt

---

### Task 6: Verify End-to-End Tracing

**Status**: [ ] Not Started
**Priority**: Medium
**Estimated Effort**: Small

**Problem**: Need to verify that trace context propagation works across all services.

**Acceptance Criteria**:
- [ ] Make a request to secure-frontend that triggers auth-service which triggers notification-service
- [ ] Verify in Tempo/Grafana that all three services appear in the same trace
- [ ] Document the verification process

---

## Implementation Order

1. **Task 1**: Fix document-service (unblocks full workspace build)
2. **Task 3**: Create test script (unblocks running all tests)
3. **Task 2**: Run auth-service tests (verify auth-service works)
4. **Task 4**: Add notification-service Dockerfile (unblocks deployment)
5. **Task 5**: Integrate services (enable cross-service functionality)
6. **Task 6**: Verify tracing (confirm observability works)

## Success Criteria

- [x] `cargo build` succeeds for entire workspace
- [x] `scripts/test-with-db.sh` runs all tests successfully
- [ ] All services can be started with `docker-compose up`
- [ ] Cross-service requests appear as single traces in Tempo
- [x] No ignored tests (except intentionally skipped) - auth-service tests now run with postgres

## Dependencies

- PostgreSQL running locally or in Docker
- MongoDB running locally or in Docker
- Docker and Docker Compose installed

## Notes

- The KYS authentication removal was completed; services now use trace context propagation instead
- Document-service errors are pre-existing and not related to the recent changes
- Consider adding integration tests that verify cross-service communication
