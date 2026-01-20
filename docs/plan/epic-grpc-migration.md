# Epic: Migrate Services to gRPC

**Status**: Not Started
**Priority**: Medium
**Created**: 2026-01-20
**Related Services**: auth-service, document-service, notification-service, payment-service, service-core

## Overview

Migrate all non-BFF microservices from REST/HTTP to gRPC for service-to-service communication. This improves type safety, enables better schema evolution through protobuf, and provides performance benefits through HTTP/2 and binary serialization.

## Motivation

### Why gRPC?

1. **Type Safety**: Protobuf schemas provide compile-time type checking across service boundaries
2. **Schema Evolution**: Protobuf's field numbering enables backward/forward compatible changes
3. **Performance**: Binary serialization and HTTP/2 multiplexing reduce latency and bandwidth
4. **Code Generation**: Auto-generated clients eliminate manual DTO maintenance
5. **Streaming**: Native support for bidirectional streaming (useful for real-time features)
6. **Standardized**: Well-defined patterns for errors, deadlines, and metadata propagation

### Current State

- All services use REST/HTTP with JSON serialization
- DTOs are manually defined in each service
- No compile-time validation of service contracts
- Schema changes require coordinated updates across services

### Target State

- Service-to-service calls use gRPC with protobuf
- REST endpoints maintained for external clients (mobile apps, third-party integrations)
- Shared proto definitions in a central location
- Generated Rust clients for type-safe service calls
- gRPC reflection enabled for debugging

## Architecture

```
┌─────────────────┐     REST/HTTP      ┌──────────────────┐
│  Mobile Apps    │◄──────────────────►│  auth-service    │
│  External APIs  │                    │  (REST + gRPC)   │
└─────────────────┘                    └────────┬─────────┘
                                                │ gRPC
┌─────────────────┐     REST/HTTP      ┌────────▼─────────┐
│  secure-frontend│◄──────────────────►│notification-svc  │
│  (BFF)          │                    │  (gRPC only)     │
└────────┬────────┘                    └──────────────────┘
         │ gRPC
         ▼
┌─────────────────┐     gRPC           ┌──────────────────┐
│  auth-service   │◄──────────────────►│  document-service│
│                 │                    │  (REST + gRPC)   │
└─────────────────┘                    └──────────────────┘
```

**Dual Protocol Strategy:**
- External-facing services (auth, document): REST + gRPC
- Internal-only services (notification): gRPC only
- BFF (secure-frontend): REST to clients, gRPC to backend services

## Proto Organization

```
proto/
├── common/
│   ├── error.proto          # Standard error types
│   ├── pagination.proto     # Pagination request/response
│   └── metadata.proto       # Common metadata types
├── auth/
│   ├── v1/
│   │   ├── auth.proto       # Auth service definition
│   │   ├── user.proto       # User messages
│   │   ├── tenant.proto     # Tenant messages
│   │   └── token.proto      # Token messages
│   └── v2/                  # Future versions
├── document/
│   └── v1/
│       ├── document.proto   # Document service definition
│       └── storage.proto    # Storage-related messages
├── notification/
│   └── v1/
│       ├── notification.proto
│       ├── email.proto
│       ├── sms.proto
│       └── push.proto
└── payment/
    └── v1/
        ├── payment.proto
        └── transaction.proto
```

## Tasks

### Phase 1: Foundation

#### Task 1.1: Set Up Proto Infrastructure

**Status**: [ ] Not Started
**Priority**: Critical
**Estimated Effort**: Medium

**Description**: Create the proto directory structure and build configuration.

**Acceptance Criteria**:
- [ ] Create `proto/` directory in repository root
- [ ] Add `buf.yaml` for proto linting and breaking change detection
- [ ] Add `buf.gen.yaml` for Rust code generation
- [ ] Create common proto files (error.proto, pagination.proto)
- [ ] Add proto compilation to CI pipeline
- [ ] Document proto development workflow in CLAUDE.md

**Files to Create**:
- `proto/buf.yaml`
- `proto/buf.gen.yaml`
- `proto/common/error.proto`
- `proto/common/pagination.proto`
- `proto/common/metadata.proto`

**Dependencies**:
- `prost` - Protobuf code generation
- `tonic` - gRPC implementation for Rust
- `tonic-build` - Build-time proto compilation
- `buf` - Proto linting and management (CLI tool)

---

#### Task 1.2: Add gRPC Dependencies to service-core

**Status**: [ ] Not Started
**Priority**: Critical
**Estimated Effort**: Small

**Description**: Add shared gRPC utilities to service-core for reuse across services.

**Acceptance Criteria**:
- [ ] Add tonic and prost dependencies to service-core
- [ ] Create gRPC server builder with standard interceptors
- [ ] Add trace context propagation for gRPC (grpc-metadata)
- [ ] Create gRPC health check service implementation
- [ ] Add gRPC reflection service wrapper
- [ ] Create error conversion utilities (AppError ↔ tonic::Status)

**Files to Create**:
- `service-core/src/grpc/mod.rs`
- `service-core/src/grpc/server.rs`
- `service-core/src/grpc/interceptors.rs`
- `service-core/src/grpc/health.rs`
- `service-core/src/grpc/error.rs`

**Files to Modify**:
- `service-core/Cargo.toml`
- `service-core/src/lib.rs`

---

### Phase 2: Auth Service Migration

#### Task 2.1: Define Auth Service Protos

**Status**: [ ] Not Started
**Priority**: High
**Estimated Effort**: Medium

**Description**: Create protobuf definitions for auth-service.

**Acceptance Criteria**:
- [ ] Define auth.proto with core RPC methods:
  - `Register`, `Login`, `Refresh`, `Logout`
  - `ValidateToken`, `GetAuthContext`
  - `SendOtp`, `VerifyOtp`
- [ ] Define user.proto with User, UserIdentity messages
- [ ] Define tenant.proto with Tenant, OrgNode messages
- [ ] Define token.proto with TokenPair, Claims messages
- [ ] Add appropriate field options for validation
- [ ] Run buf lint and fix any issues

**Files to Create**:
- `proto/auth/v1/auth.proto`
- `proto/auth/v1/user.proto`
- `proto/auth/v1/tenant.proto`
- `proto/auth/v1/token.proto`
- `proto/auth/v1/org.proto`
- `proto/auth/v1/role.proto`

---

#### Task 2.2: Implement Auth gRPC Server

**Status**: [ ] Not Started
**Priority**: High
**Estimated Effort**: Large

**Description**: Implement gRPC server for auth-service alongside existing REST API.

**Acceptance Criteria**:
- [ ] Generate Rust code from auth protos
- [ ] Implement `AuthService` trait from generated code
- [ ] Reuse existing business logic (handlers call same service layer)
- [ ] Add gRPC server to auth-service startup
- [ ] Configure separate port for gRPC (e.g., 3001)
- [ ] Add gRPC health check endpoint
- [ ] Add gRPC reflection for debugging
- [ ] Update Docker configuration to expose gRPC port

**Files to Create**:
- `auth-service/build.rs`
- `auth-service/src/grpc/mod.rs`
- `auth-service/src/grpc/auth_service.rs`
- `auth-service/src/grpc/user_service.rs`
- `auth-service/src/grpc/tenant_service.rs`

**Files to Modify**:
- `auth-service/Cargo.toml`
- `auth-service/src/lib.rs`
- `auth-service/src/main.rs`
- `auth-service/Dockerfile`
- `docker-compose.dev.yml`
- `docker-compose.prod.yml`

---

#### Task 2.3: Create Auth gRPC Client in service-core

**Status**: [ ] Not Started
**Priority**: High
**Estimated Effort**: Small

**Description**: Create reusable auth-service gRPC client for other services.

**Acceptance Criteria**:
- [ ] Generate client code from auth protos
- [ ] Create `AuthClient` wrapper with connection pooling
- [ ] Add automatic trace context propagation
- [ ] Add retry logic with exponential backoff
- [ ] Add circuit breaker for fault tolerance
- [ ] Create mock client for testing

**Files to Create**:
- `service-core/src/clients/mod.rs`
- `service-core/src/clients/auth_client.rs`

---

### Phase 3: Notification Service Migration

#### Task 3.1: Define Notification Service Protos

**Status**: [ ] Not Started
**Priority**: High
**Estimated Effort**: Medium

**Description**: Create protobuf definitions for notification-service.

**Acceptance Criteria**:
- [ ] Define notification.proto with core RPC methods:
  - `SendEmail`, `SendSms`, `SendPush`
  - `SendBatch`
  - `GetNotificationStatus`, `ListNotifications`
- [ ] Define email.proto, sms.proto, push.proto messages
- [ ] Add streaming RPC for batch notifications
- [ ] Run buf lint and fix any issues

**Files to Create**:
- `proto/notification/v1/notification.proto`
- `proto/notification/v1/email.proto`
- `proto/notification/v1/sms.proto`
- `proto/notification/v1/push.proto`

---

#### Task 3.2: Implement Notification gRPC Server

**Status**: [ ] Not Started
**Priority**: High
**Estimated Effort**: Medium

**Description**: Replace REST API with gRPC-only server for notification-service.

**Acceptance Criteria**:
- [ ] Generate Rust code from notification protos
- [ ] Implement `NotificationService` trait
- [ ] Remove REST handlers (internal service only)
- [ ] Keep health check endpoint on HTTP for Docker
- [ ] Add gRPC reflection
- [ ] Update all configuration

**Files to Create**:
- `notification-service/build.rs`
- `notification-service/src/grpc/mod.rs`
- `notification-service/src/grpc/notification_service.rs`

**Files to Modify**:
- `notification-service/Cargo.toml`
- `notification-service/src/lib.rs`
- `notification-service/src/main.rs`

---

#### Task 3.3: Update Auth Service to Use Notification gRPC Client

**Status**: [ ] Not Started
**Priority**: High
**Estimated Effort**: Small

**Description**: Replace NotificationClient HTTP calls with gRPC.

**Acceptance Criteria**:
- [ ] Generate notification client code
- [ ] Update `NotificationClient` to use gRPC
- [ ] Remove HTTP-based implementation
- [ ] Update configuration (URL → gRPC address)
- [ ] Verify trace context propagation works

**Files to Modify**:
- `auth-service/src/services/notification_client.rs`
- `auth-service/src/config/mod.rs`

---

### Phase 4: Document Service Migration

#### Task 4.1: Define Document Service Protos

**Status**: [ ] Not Started
**Priority**: Medium
**Estimated Effort**: Medium

**Description**: Create protobuf definitions for document-service.

**Acceptance Criteria**:
- [ ] Define document.proto with RPC methods:
  - `UploadDocument`, `DownloadDocument`, `DeleteDocument`
  - `GetDocumentMetadata`, `ListDocuments`
  - `GenerateSignedUrl`
- [ ] Use streaming for upload/download of large files
- [ ] Define storage.proto for storage-related types

**Files to Create**:
- `proto/document/v1/document.proto`
- `proto/document/v1/storage.proto`

---

#### Task 4.2: Implement Document gRPC Server

**Status**: [ ] Not Started
**Priority**: Medium
**Estimated Effort**: Large

**Description**: Add gRPC server to document-service alongside REST.

**Acceptance Criteria**:
- [ ] Implement streaming upload/download
- [ ] Maintain REST for direct browser uploads
- [ ] Add gRPC endpoints for service-to-service calls

---

### Phase 5: BFF Migration

#### Task 5.1: Update secure-frontend to Use gRPC Clients

**Status**: [ ] Not Started
**Priority**: Medium
**Estimated Effort**: Medium

**Description**: Replace REST clients in BFF with gRPC clients.

**Acceptance Criteria**:
- [ ] Replace `AuthClient` HTTP calls with gRPC
- [ ] Replace `DocumentClient` HTTP calls with gRPC
- [ ] Replace `NotificationClient` HTTP calls with gRPC
- [ ] Maintain REST API for browser clients
- [ ] Verify all trace context propagation works

**Files to Modify**:
- `secure-frontend/src/services/auth_client.rs`
- `secure-frontend/src/services/document_client.rs`
- `secure-frontend/src/config/mod.rs`

---

### Phase 6: Payment Service Migration

#### Task 6.1: Define Payment Service Protos

**Status**: [ ] Not Started
**Priority**: Low
**Estimated Effort**: Medium

**Files to Create**:
- `proto/payment/v1/payment.proto`
- `proto/payment/v1/transaction.proto`

---

#### Task 6.2: Implement Payment gRPC Server

**Status**: [ ] Not Started
**Priority**: Low
**Estimated Effort**: Medium

---

### Phase 7: Cleanup and Documentation

#### Task 7.1: Remove Unused REST Code

**Status**: [ ] Not Started
**Priority**: Low
**Estimated Effort**: Small

**Description**: Remove REST handlers that are no longer needed after gRPC migration.

---

#### Task 7.2: Update Documentation

**Status**: [ ] Not Started
**Priority**: Medium
**Estimated Effort**: Small

**Acceptance Criteria**:
- [ ] Update CLAUDE.md with gRPC development workflow
- [ ] Document proto style guide
- [ ] Add gRPC debugging guide (using grpcurl, reflection)
- [ ] Update API documentation

---

## Implementation Order

1. **Phase 1**: Foundation (proto infrastructure, service-core utilities)
2. **Phase 2**: Auth Service (most critical, used by all services)
3. **Phase 3**: Notification Service (simple, internal-only)
4. **Phase 4**: Document Service (streaming complexity)
5. **Phase 5**: BFF Migration (depends on backend services)
6. **Phase 6**: Payment Service (lowest priority)
7. **Phase 7**: Cleanup

## Success Criteria

- [ ] All service-to-service calls use gRPC
- [ ] Proto definitions pass buf lint with no warnings
- [ ] No breaking changes detected by buf breaking
- [ ] gRPC reflection enabled on all services
- [ ] Trace context propagates through gRPC calls (visible in Tempo)
- [ ] All existing tests pass
- [ ] gRPC endpoints have equivalent test coverage
- [ ] Documentation updated

## Technical Decisions

### Proto Style Guide

1. **Package naming**: `micros.<service>.v<version>` (e.g., `micros.auth.v1`)
2. **File naming**: lowercase with underscores (e.g., `user_service.proto`)
3. **Message naming**: PascalCase (e.g., `CreateUserRequest`)
4. **Field naming**: snake_case (e.g., `user_id`)
5. **Enum naming**: SCREAMING_SNAKE_CASE (e.g., `USER_STATUS_ACTIVE`)
6. **RPC naming**: PascalCase verbs (e.g., `CreateUser`, `GetUser`)

### Error Handling

Map `AppError` variants to gRPC status codes:

| AppError | gRPC Status |
|----------|-------------|
| `ValidationError` | `INVALID_ARGUMENT` |
| `NotFound` | `NOT_FOUND` |
| `Unauthorized` | `UNAUTHENTICATED` |
| `Forbidden` | `PERMISSION_DENIED` |
| `Conflict` | `ALREADY_EXISTS` |
| `TooManyRequests` | `RESOURCE_EXHAUSTED` |
| `InternalError` | `INTERNAL` |
| `ServiceUnavailable` | `UNAVAILABLE` |

### Port Configuration

| Service | REST Port | gRPC Port |
|---------|-----------|-----------|
| auth-service | 3000 | 3001 |
| document-service | 8080 | 8081 |
| notification-service | - | 8080 |
| payment-service | 3003 | 3004 |

### Dependencies

```toml
# Cargo.toml additions
[dependencies]
tonic = "0.12"
prost = "0.13"
prost-types = "0.13"
tonic-reflection = "0.12"
tonic-health = "0.12"

[build-dependencies]
tonic-build = "0.12"
```

## Risks and Mitigations

| Risk | Mitigation |
|------|------------|
| Breaking changes during migration | Keep REST endpoints until gRPC stable |
| Proto schema drift | Use buf breaking to detect incompatibilities |
| Debugging difficulty | Enable gRPC reflection, use grpcurl |
| Performance regression | Benchmark before/after, use connection pooling |
| Team learning curve | Document patterns, create examples |

## References

- [tonic documentation](https://docs.rs/tonic/latest/tonic/)
- [prost documentation](https://docs.rs/prost/latest/prost/)
- [buf documentation](https://buf.build/docs/)
- [gRPC Rust tutorial](https://grpc.io/docs/languages/rust/)
- [Protocol Buffers style guide](https://protobuf.dev/programming-guides/style/)
