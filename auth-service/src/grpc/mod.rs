//! gRPC service implementations for auth-service.

pub mod auth_service;
pub mod authz_service;

// Include the generated proto code
pub mod proto {
    pub mod auth {
        tonic::include_proto!("micros.auth.v1");

        // File descriptor set for gRPC reflection
        pub const FILE_DESCRIPTOR_SET: &[u8] =
            tonic::include_file_descriptor_set!("auth_service_descriptor");
    }
}

pub use auth_service::AuthServiceImpl;
pub use authz_service::AuthzServiceImpl;
