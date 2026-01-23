//! gRPC service implementations for auth-service.

pub mod assignment_service;
pub mod audit_service;
pub mod auth_service;
pub mod authz_service;
pub mod invitation_service;
pub mod org_service;
pub mod role_service;
pub mod visibility_service;

// Include the generated proto code
pub mod proto {
    pub mod auth {
        tonic::include_proto!("micros.auth.v1");

        // File descriptor set for gRPC reflection
        pub const FILE_DESCRIPTOR_SET: &[u8] =
            tonic::include_file_descriptor_set!("auth_service_descriptor");
    }
}

pub use assignment_service::AssignmentServiceImpl;
pub use audit_service::AuditServiceImpl;
pub use auth_service::AuthServiceImpl;
pub use authz_service::AuthzServiceImpl;
pub use invitation_service::InvitationServiceImpl;
pub use org_service::OrgServiceImpl;
pub use role_service::RoleServiceImpl;
pub use visibility_service::VisibilityServiceImpl;
