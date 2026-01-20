//! Auth service gRPC client for service-to-service communication.

use std::time::Duration;
use tonic::Request;
use tonic::transport::{Channel, Endpoint};

use super::proto::auth::auth_service_client::AuthServiceClient;
use super::proto::auth::authz_service_client::AuthzServiceClient;
use super::proto::auth::{
    CheckCapabilityRequest, CheckCapabilityResponse, GetAuthContextRequest, GetAuthContextResponse,
    LoginRequest, LoginResponse, LogoutRequest, RefreshRequest, RefreshResponse, RegisterRequest,
    RegisterResponse, ValidateTokenRequest, ValidateTokenResponse,
};

/// Configuration for the auth service client.
#[derive(Clone, Debug)]
pub struct AuthClientConfig {
    /// The gRPC endpoint of the auth service (e.g., "http://auth-service:50051").
    pub endpoint: String,
    /// Connection timeout.
    pub connect_timeout: Duration,
    /// Request timeout.
    pub request_timeout: Duration,
}

impl Default for AuthClientConfig {
    fn default() -> Self {
        Self {
            endpoint: "http://localhost:50051".to_string(),
            connect_timeout: Duration::from_secs(5),
            request_timeout: Duration::from_secs(30),
        }
    }
}

/// Auth service client for calling auth-service via gRPC.
#[derive(Clone)]
pub struct AuthClient {
    auth_client: AuthServiceClient<Channel>,
    authz_client: AuthzServiceClient<Channel>,
}

impl AuthClient {
    /// Create a new auth client with the given configuration.
    pub async fn new(config: AuthClientConfig) -> Result<Self, tonic::transport::Error> {
        let channel = Endpoint::from_shared(config.endpoint)?
            .connect_timeout(config.connect_timeout)
            .timeout(config.request_timeout)
            .connect()
            .await?;

        Ok(Self {
            auth_client: AuthServiceClient::new(channel.clone()),
            authz_client: AuthzServiceClient::new(channel),
        })
    }

    /// Create a new auth client connecting to the specified endpoint.
    pub async fn connect(endpoint: &str) -> Result<Self, tonic::transport::Error> {
        Self::new(AuthClientConfig {
            endpoint: endpoint.to_string(),
            ..Default::default()
        })
        .await
    }

    // =========================================================================
    // AuthService methods
    // =========================================================================

    /// Register a new user.
    pub async fn register(
        &mut self,
        tenant_slug: String,
        email: String,
        password: String,
        display_name: Option<String>,
    ) -> Result<RegisterResponse, tonic::Status> {
        let request = Request::new(RegisterRequest {
            tenant_slug,
            email,
            password,
            display_name,
        });
        let response = self.auth_client.register(request).await?;
        Ok(response.into_inner())
    }

    /// Login with email and password.
    pub async fn login(
        &mut self,
        tenant_slug: String,
        email: String,
        password: String,
    ) -> Result<LoginResponse, tonic::Status> {
        let request = Request::new(LoginRequest {
            tenant_slug,
            email,
            password,
        });
        let response = self.auth_client.login(request).await?;
        Ok(response.into_inner())
    }

    /// Refresh tokens using a refresh token.
    pub async fn refresh(
        &mut self,
        refresh_token: String,
    ) -> Result<RefreshResponse, tonic::Status> {
        let request = Request::new(RefreshRequest { refresh_token });
        let response = self.auth_client.refresh(request).await?;
        Ok(response.into_inner())
    }

    /// Logout and invalidate the refresh token.
    pub async fn logout(&mut self, refresh_token: String) -> Result<(), tonic::Status> {
        let request = Request::new(LogoutRequest { refresh_token });
        self.auth_client.logout(request).await?;
        Ok(())
    }

    /// Validate an access token.
    pub async fn validate_token(
        &mut self,
        access_token: String,
    ) -> Result<ValidateTokenResponse, tonic::Status> {
        let request = Request::new(ValidateTokenRequest { access_token });
        let response = self.auth_client.validate_token(request).await?;
        Ok(response.into_inner())
    }

    // =========================================================================
    // AuthzService methods
    // =========================================================================

    /// Get authorization context for a user.
    ///
    /// The user_id and tenant_id should be passed via request metadata.
    pub async fn get_auth_context(
        &mut self,
        user_id: &str,
        tenant_id: &str,
        org_node_id: Option<String>,
    ) -> Result<GetAuthContextResponse, tonic::Status> {
        let mut request = Request::new(GetAuthContextRequest { org_node_id });
        request
            .metadata_mut()
            .insert("x-user-id", user_id.parse().unwrap());
        request
            .metadata_mut()
            .insert("x-tenant-id", tenant_id.parse().unwrap());
        let response = self.authz_client.get_auth_context(request).await?;
        Ok(response.into_inner())
    }

    /// Check if a user has a specific capability at an org node.
    pub async fn check_capability(
        &mut self,
        user_id: &str,
        tenant_id: &str,
        org_node_id: String,
        capability: String,
    ) -> Result<CheckCapabilityResponse, tonic::Status> {
        let mut request = Request::new(CheckCapabilityRequest {
            org_node_id,
            capability,
        });
        request
            .metadata_mut()
            .insert("x-user-id", user_id.parse().unwrap());
        request
            .metadata_mut()
            .insert("x-tenant-id", tenant_id.parse().unwrap());
        let response = self.authz_client.check_capability(request).await?;
        Ok(response.into_inner())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_client_config_default() {
        let config = AuthClientConfig::default();
        assert_eq!(config.endpoint, "http://localhost:50051");
        assert_eq!(config.connect_timeout, Duration::from_secs(5));
        assert_eq!(config.request_timeout, Duration::from_secs(30));
    }
}
