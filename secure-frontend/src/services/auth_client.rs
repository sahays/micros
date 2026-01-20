//! Auth service client for secure-frontend BFF pattern.
//!
//! Uses gRPC for internal service calls (login, register, logout).
//! OAuth flows still use HTTP redirects via public_url.

use crate::config::AuthServiceSettings;
use anyhow::Result;
use reqwest::Client as HttpClient;
use service_core::grpc::{AuthClient as GrpcAuthClient, AuthClientConfig};
use service_core::observability::TracedClientExt;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Auth client wrapping gRPC client for auth-service communication.
/// Also includes HTTP client for OAuth flows that require browser redirects.
pub struct AuthClient {
    grpc_client: Arc<Mutex<GrpcAuthClient>>,
    http_client: HttpClient,
    settings: AuthServiceSettings,
}

impl AuthClient {
    /// Create a new auth client with gRPC connection and HTTP client for OAuth.
    pub async fn new(settings: AuthServiceSettings) -> Result<Self> {
        let config = AuthClientConfig {
            endpoint: settings.grpc_url.clone(),
            ..Default::default()
        };

        let grpc_client = GrpcAuthClient::new(config).await.map_err(|e| {
            tracing::error!("Failed to connect to auth-service gRPC: {}", e);
            anyhow::anyhow!("gRPC connection failed: {}", e)
        })?;

        tracing::info!(
            endpoint = %settings.grpc_url,
            "Connected to auth-service gRPC"
        );

        Ok(Self {
            grpc_client: Arc::new(Mutex::new(grpc_client)),
            http_client: HttpClient::new(),
            settings,
        })
    }

    /// Get the public URL for OAuth redirects (browser-accessible).
    pub fn public_url(&self) -> &str {
        &self.settings.public_url
    }

    /// Get the default tenant slug.
    pub fn tenant_slug(&self) -> &str {
        &self.settings.default_tenant_slug
    }

    /// Login with email and password via gRPC.
    ///
    /// Returns tokens (access_token, refresh_token) and user info on success.
    pub async fn login(&self, email: &str, password: &str) -> Result<LoginResult> {
        let mut client = self.grpc_client.lock().await;

        let response = client
            .login(
                self.settings.default_tenant_slug.clone(),
                email.to_string(),
                password.to_string(),
            )
            .await
            .map_err(|e| {
                tracing::warn!(email = %email, error = %e, "Login failed");
                anyhow::anyhow!("Login failed: {}", e.message())
            })?;

        Ok(LoginResult {
            access_token: response.access_token,
            refresh_token: response.refresh_token,
            expires_in: response.expires_in,
            user: response.user.map(|u| UserInfo {
                id: u.user_id,
                email: u.email,
                display_name: u.display_name,
            }),
        })
    }

    /// Register a new user via gRPC.
    ///
    /// Returns tokens and user info on success.
    pub async fn register(
        &self,
        email: &str,
        password: &str,
        display_name: Option<&str>,
    ) -> Result<LoginResult> {
        let mut client = self.grpc_client.lock().await;

        let response = client
            .register(
                self.settings.default_tenant_slug.clone(),
                email.to_string(),
                password.to_string(),
                display_name.map(String::from),
            )
            .await
            .map_err(|e| {
                tracing::warn!(email = %email, error = %e, "Registration failed");
                anyhow::anyhow!("Registration failed: {}", e.message())
            })?;

        Ok(LoginResult {
            access_token: response.access_token,
            refresh_token: response.refresh_token,
            expires_in: response.expires_in,
            user: response.user.map(|u| UserInfo {
                id: u.user_id,
                email: u.email,
                display_name: u.display_name,
            }),
        })
    }

    /// Logout and revoke refresh token via gRPC.
    pub async fn logout(&self, refresh_token: &str) -> Result<()> {
        let mut client = self.grpc_client.lock().await;

        client
            .logout(refresh_token.to_string())
            .await
            .map_err(|e| {
                tracing::warn!(error = %e, "Logout/token revocation failed");
                anyhow::anyhow!("Logout failed: {}", e.message())
            })?;

        tracing::info!("Token revoked successfully via gRPC");
        Ok(())
    }

    /// Refresh tokens using refresh token via gRPC.
    pub async fn refresh(&self, refresh_token: &str) -> Result<LoginResult> {
        let mut client = self.grpc_client.lock().await;

        let response = client
            .refresh(refresh_token.to_string())
            .await
            .map_err(|e| {
                tracing::warn!(error = %e, "Token refresh failed");
                anyhow::anyhow!("Refresh failed: {}", e.message())
            })?;

        Ok(LoginResult {
            access_token: response.access_token,
            refresh_token: response.refresh_token,
            expires_in: response.expires_in,
            user: response.user.map(|u| UserInfo {
                id: u.user_id,
                email: u.email,
                display_name: u.display_name,
            }),
        })
    }

    /// Validate an access token via gRPC.
    pub async fn validate_token(&self, access_token: &str) -> Result<TokenValidation> {
        let mut client = self.grpc_client.lock().await;

        let response = client
            .validate_token(access_token.to_string())
            .await
            .map_err(|e| {
                tracing::warn!(error = %e, "Token validation failed");
                anyhow::anyhow!("Validation failed: {}", e.message())
            })?;

        Ok(TokenValidation {
            valid: response.valid,
            claims: response.claims.map(|c| TokenClaims {
                sub: c.sub,
                email: c.email,
                app_id: c.app_id,
                exp: c.exp.map(|t| t.seconds).unwrap_or(0),
            }),
        })
    }

    // =========================================================================
    // OAuth methods (still use HTTP for browser redirect flows)
    // =========================================================================

    /// Exchange OAuth authorization code for tokens via HTTP.
    ///
    /// OAuth flows require HTTP because they involve browser redirects.
    /// This method is used for the callback after user authorizes with Google.
    pub async fn oauth_callback(&self, code: &str, state: Option<&str>) -> Result<OAuthResult> {
        let url = format!("{}/auth/social/google/callback", self.settings.url);

        let mut body = serde_json::json!({
            "code": code,
        });

        if let Some(s) = state {
            body["state"] = serde_json::json!(s);
        }

        let response = self
            .http_client
            .traced_post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                tracing::error!("OAuth callback request failed: {}", e);
                anyhow::anyhow!("OAuth callback failed: {}", e)
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            tracing::error!(status = %status, error = %error_text, "OAuth callback failed");
            return Err(anyhow::anyhow!(
                "OAuth callback failed with status {}",
                status
            ));
        }

        let tokens: serde_json::Value = response.json().await.map_err(|e| {
            tracing::error!("Failed to parse OAuth response: {}", e);
            anyhow::anyhow!("Failed to parse OAuth response: {}", e)
        })?;

        Ok(OAuthResult {
            access_token: tokens["access_token"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
            refresh_token: tokens["refresh_token"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
            name: tokens["name"].as_str().map(String::from),
            picture: tokens["picture"].as_str().map(String::from),
        })
    }
}

/// Result from OAuth callback.
#[derive(Debug)]
pub struct OAuthResult {
    pub access_token: String,
    pub refresh_token: String,
    pub name: Option<String>,
    pub picture: Option<String>,
}

/// Result from login/register operations.
#[derive(Debug)]
pub struct LoginResult {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: i64,
    pub user: Option<UserInfo>,
}

/// User information from auth response.
#[derive(Debug)]
pub struct UserInfo {
    pub id: String,
    pub email: String,
    pub display_name: Option<String>,
}

/// Result from token validation.
#[derive(Debug)]
pub struct TokenValidation {
    pub valid: bool,
    pub claims: Option<TokenClaims>,
}

/// Token claims from validation response.
#[derive(Debug)]
pub struct TokenClaims {
    pub sub: String,
    pub email: String,
    pub app_id: String,
    pub exp: i64,
}
