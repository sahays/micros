#![allow(dead_code)]

use crate::proto::auth::{
    admin_service_client::AdminServiceClient, auth_service_client::AuthServiceClient,
    authz_service_client::AuthzServiceClient, invitation_service_client::InvitationServiceClient,
    org_service_client::OrgServiceClient, role_service_client::RoleServiceClient,
};
use tonic::transport::Channel;

fn grpc_endpoint() -> String {
    std::env::var("AUTH_GRPC_ENDPOINT").unwrap_or_else(|_| "http://localhost:50051".to_string())
}

async fn connect(addr: &str) -> Channel {
    Channel::from_shared(addr.to_string())
        .unwrap()
        .connect()
        .await
        .unwrap_or_else(|e| panic!("Failed to connect to gRPC at {}: {}", addr, e))
}

/// Test application with connections to the deployed auth-service.
pub struct TestApp {
    grpc_addr: String,
}

impl TestApp {
    /// Connect to the deployed auth-service.
    pub async fn spawn() -> Self {
        TestApp {
            grpc_addr: grpc_endpoint(),
        }
    }

    /// Get the gRPC endpoint address.
    pub fn grpc_addr(&self) -> String {
        self.grpc_addr.clone()
    }

    /// Create an AdminService client.
    pub async fn admin_client(&self) -> AdminServiceClient<Channel> {
        let channel = connect(&self.grpc_addr).await;
        AdminServiceClient::new(channel)
    }

    /// Create an AuthService client.
    pub async fn auth_client(&self) -> AuthServiceClient<Channel> {
        let channel = connect(&self.grpc_addr).await;
        AuthServiceClient::new(channel)
    }

    /// Create an AuthzService client.
    pub async fn authz_client(&self) -> AuthzServiceClient<Channel> {
        let channel = connect(&self.grpc_addr).await;
        AuthzServiceClient::new(channel)
    }

    /// Create an OrgService client.
    pub async fn org_client(&self) -> OrgServiceClient<Channel> {
        let channel = connect(&self.grpc_addr).await;
        OrgServiceClient::new(channel)
    }

    /// Create a RoleService client.
    pub async fn role_client(&self) -> RoleServiceClient<Channel> {
        let channel = connect(&self.grpc_addr).await;
        RoleServiceClient::new(channel)
    }

    /// Create an InvitationService client.
    pub async fn invitation_client(&self) -> InvitationServiceClient<Channel> {
        let channel = connect(&self.grpc_addr).await;
        InvitationServiceClient::new(channel)
    }
}

/// Add authorization header to a request.
pub fn with_auth<T>(mut request: tonic::Request<T>, token: &str) -> tonic::Request<T> {
    request.metadata_mut().insert(
        "authorization",
        format!("Bearer {}", token).parse().unwrap(),
    );
    request
}
