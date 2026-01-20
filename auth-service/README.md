# Auth Service v2

Multi-tenant authentication and authorization service with capability-based access control.

## Architecture

**gRPC-only** internal service. External clients access via BFF (secure-frontend).

- **gRPC**: Port 50051 (all business logic)
- **HTTP**: Port 3000 (health checks only)

## gRPC Services

| Service | Description |
|---------|-------------|
| `AuthService` | Register, Login, Refresh, Logout, OTP |
| `OrgService` | Organization hierarchy (closure table) |
| `RoleService` | Roles and capabilities |
| `AssignmentService` | Time-bounded role assignments |
| `InvitationService` | User onboarding |
| `VisibilityService` | Cross-org visibility grants |
| `AuditService` | Audit log queries |
| `ServiceRegistryService` | KYS (Know Your Service) |

## RPC Methods

### AuthService
```protobuf
rpc Register(RegisterRequest) returns (RegisterResponse)
rpc Login(LoginRequest) returns (LoginResponse)
rpc Refresh(RefreshRequest) returns (RefreshResponse)
rpc Logout(LogoutRequest) returns (Empty)
rpc ValidateToken(ValidateTokenRequest) returns (ValidateTokenResponse)
rpc SendOtp(SendOtpRequest) returns (SendOtpResponse)
rpc VerifyOtp(VerifyOtpRequest) returns (VerifyOtpResponse)
```

### OrgService
```protobuf
rpc CreateOrgNode(CreateOrgNodeRequest) returns (CreateOrgNodeResponse)
rpc GetOrgNode(GetOrgNodeRequest) returns (GetOrgNodeResponse)
rpc GetOrgNodeDescendants(GetOrgNodeDescendantsRequest) returns (GetOrgNodeDescendantsResponse)
rpc ListTenantOrgNodes(ListTenantOrgNodesRequest) returns (ListTenantOrgNodesResponse)
rpc GetTenantOrgTree(GetTenantOrgTreeRequest) returns (GetTenantOrgTreeResponse)
```

### RoleService
```protobuf
rpc CreateRole(CreateRoleRequest) returns (CreateRoleResponse)
rpc GetRole(GetRoleRequest) returns (GetRoleResponse)
rpc ListTenantRoles(ListTenantRolesRequest) returns (ListTenantRolesResponse)
rpc GetRoleCapabilities(GetRoleCapabilitiesRequest) returns (GetRoleCapabilitiesResponse)
rpc AssignCapability(AssignCapabilityRequest) returns (AssignCapabilityResponse)
rpc ListCapabilities(ListCapabilitiesRequest) returns (ListCapabilitiesResponse)
rpc GetCapability(GetCapabilityRequest) returns (GetCapabilityResponse)
```

### AssignmentService
```protobuf
rpc CreateAssignment(CreateAssignmentRequest) returns (CreateAssignmentResponse)
rpc EndAssignment(EndAssignmentRequest) returns (EndAssignmentResponse)
rpc ListUserAssignments(ListUserAssignmentsRequest) returns (ListUserAssignmentsResponse)
```

### InvitationService
```protobuf
rpc CreateInvitation(CreateInvitationRequest) returns (CreateInvitationResponse)
rpc GetInvitation(GetInvitationRequest) returns (GetInvitationResponse)
rpc AcceptInvitation(AcceptInvitationRequest) returns (AcceptInvitationResponse)
```

### VisibilityService
```protobuf
rpc CreateVisibilityGrant(CreateVisibilityGrantRequest) returns (CreateVisibilityGrantResponse)
rpc RevokeVisibilityGrant(RevokeVisibilityGrantRequest) returns (RevokeVisibilityGrantResponse)
rpc ListUserVisibilityGrants(ListUserVisibilityGrantsRequest) returns (ListUserVisibilityGrantsResponse)
```

### AuditService
```protobuf
rpc QueryAuditEvents(QueryAuditEventsRequest) returns (QueryAuditEventsResponse)
rpc GetAuditEvent(GetAuditEventRequest) returns (GetAuditEventResponse)
```

### ServiceRegistryService
```protobuf
rpc RegisterService(RegisterServiceRequest) returns (RegisterServiceResponse)
rpc GetServiceToken(GetServiceTokenRequest) returns (GetServiceTokenResponse)
rpc GetService(GetServiceRequest) returns (GetServiceResponse)
rpc RotateSecret(RotateSecretRequest) returns (RotateSecretResponse)
rpc GetServicePermissions(GetServicePermissionsRequest) returns (GetServicePermissionsResponse)
rpc GrantPermission(GrantPermissionRequest) returns (GrantPermissionResponse)
```

## Usage (grpcurl)

```bash
# List services
grpcurl -plaintext localhost:50051 list

# Login
grpcurl -plaintext -d '{
  "tenant_slug": "acme",
  "email": "user@example.com",
  "password": "secret123"
}' localhost:50051 micros.auth.v1.AuthService/Login

# Validate token
grpcurl -plaintext -d '{
  "access_token": "eyJ..."
}' localhost:50051 micros.auth.v1.AuthService/ValidateToken
```

## Configuration

| Variable | Description |
|----------|-------------|
| `DATABASE_URL` | PostgreSQL connection |
| `REDIS_URL` | Redis connection |
| `JWT_PRIVATE_KEY_PATH` | RS256 private key |
| `JWT_PUBLIC_KEY_PATH` | RS256 public key |
| `GRPC_PORT` | gRPC port (default: 50051) |
| `HTTP_PORT` | Health check port (default: 3000) |

## Health Checks

```bash
# HTTP health endpoint
curl http://localhost:3000/health

# gRPC health check
grpcurl -plaintext localhost:50051 grpc.health.v1.Health/Check
```

## Proto Definitions

See `proto/micros/auth/v1/`:
- `auth.proto` - Authentication RPCs
- `org.proto` - Organization hierarchy
- `role.proto` - Roles and capabilities
- `assignment.proto` - Role assignments
- `invitation.proto` - User invitations
- `visibility.proto` - Visibility grants
- `audit.proto` - Audit logs
- `service_registry.proto` - KYS service registry
