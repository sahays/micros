//! Data models for auth-service v2.
//!
//! PostgreSQL-backed models following the new schema with:
//! - Capability-based authorization
//! - Org node hierarchy with closure table
//! - Time-bounded immutable assignments
//! - Know-Your-Service (KYS) registry

pub mod audit_event;
pub mod capability;
pub mod invitation;
pub mod org_assignment;
pub mod org_node;
pub mod otp_code;
pub mod refresh_session;
pub mod role;
pub mod service;
pub mod tenant;
pub mod user;
pub mod user_identity;
pub mod visibility_grant;

// Re-export main types for convenience
pub use audit_event::{AuditEvent, AuditEventResponse, AuditEventType};
pub use capability::{Capability, CapabilityParts, CapabilityResponse, CreateCapabilityRequest};
pub use invitation::{
    AcceptInvitationRequest, CreateInvitationRequest, Invitation, InvitationResponse,
    InvitationState,
};
pub use org_assignment::{
    AssignmentDetail, AssignmentResponse, CreateAssignmentRequest, OrgAssignment,
};
pub use org_node::{
    CreateOrgNodeRequest, OrgNode, OrgNodePath, OrgNodeResponse, OrgTreeNode, UpdateOrgNodeRequest,
};
pub use otp_code::{OtpChannel, OtpCode, OtpPurpose};
pub use refresh_session::RefreshSession;
pub use role::{
    AssignCapabilityRequest, CreateRoleRequest, Role, RoleCapability, RoleResponse,
    RoleWithCapabilities,
};
pub use service::{
    RegisterServiceRequest, RegisterServiceResponse, RotateSecretResponse, Service, ServiceContext,
    ServicePermission, ServiceResponse, ServiceSecret, ServiceSession, ServiceState,
    ServiceTokenResponse,
};
pub use tenant::{CreateTenantRequest, Tenant, TenantResponse, TenantState};
pub use user::{
    AuthResponse, LoginRequest, RegisterUserRequest, TokenResponse, User, UserResponse, UserState,
};
pub use user_identity::{IdentProvider, UserIdentity};
pub use visibility_grant::{
    AccessScope, CreateVisibilityGrantRequest, VisibilityGrant, VisibilityGrantResponse,
};
