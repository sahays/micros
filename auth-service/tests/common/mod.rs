//! Test helper module for auth-service v2 integration tests.
//!
//! Provides common setup utilities for PostgreSQL-based gRPC tests.

#![allow(dead_code)]

use auth_service::{
    config::{
        AuthConfig, DatabaseConfig, Environment, GmailConfig, GoogleOAuthConfig, JwtConfig,
        NotificationServiceConfig, RateLimitConfig, RedisConfig, SecurityConfig, SwaggerConfig,
        SwaggerMode,
    },
    db,
    grpc::proto::auth::{
        admin_service_client::AdminServiceClient, auth_service_client::AuthServiceClient,
        authz_service_client::AuthzServiceClient, org_service_client::OrgServiceClient,
        role_service_client::RoleServiceClient,
    },
    services, AppState,
};
use sqlx::PgPool;
use std::io::Write;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tempfile::NamedTempFile;
use tokio::net::TcpListener;
use tonic::transport::Channel;

/// Test RSA private key for JWT signing
const TEST_PRIVATE_KEY: &str = r#"-----BEGIN PRIVATE KEY-----
MIIEvQIBADANBgkqhkiG9w0BAQEFAASCBKcwggSjAgEAAoIBAQCazAniq0OLiSsC
OhQ+HVyptrwMEaWD5YJzz2I+yjCFcLRWcQ30j9xnyZO9Rxt2lYveqlH0A73+w3St
+lzZmhs3HnrpdWUIPgFxB2EiP9Hf6ty2/e29CdxACUPx7aGh5M2ViASOdzkeFUPY
NOFkYuxZTGNGMTH2JzTwPpAavvcXmZ994OO/BJx25IBhDSK+sgPgh1NceigiakfL
6LwTwIeenkPVaus9Gi1Gi2UrmL3hr/o5MMv4NAcN+nAzIvZHVlykOn1ci6Pm939L
DSYWiVZUoj7W0dFe6klL9XsnWaUROsb5W9IQKlwJDMfCs7FHDjERPoNCVwRd9/VE
j4IPu1kdAgMBAAECggEAL3KLNSc5tPN+c1hKDCAD3yFb0nc2PI+ExOq0OnrPFJfP
Lw/IL0ZJUKbA2iuJh3efP8kFBb5/5i8S/KDZBPnvjZ2SHy0Uosoetv6ED3NwaSoc
LRr4XBFBqX8tjGJCQNVZDpR6kRCKOWZbPVI4JAUOXPDFHSbHIaQy3dDPauNN6bV6
zX0DiQ3zNtVJ/Cygd0ndiVjgILKhxC9VnN4HRA3usLkXpo7jGiCV1J7XHTQsmB3X
Kkbn3uqtjkyy7ngcLuSq6sdx/EFQhsl7rvcweeNMHNRE/paKupoeulXxbWM9EpN2
qmFDRtA8ih3EfeUK1PZGdTfLkQWt5f/4dD9w61z4IQKBgQDNUSqO58NfMqVampfb
NySa34WuXoVTNMwtHDqzFAykfg+nXo8ABGv6SvNcIHL8CicwPSYSrd5JvbSCTwVs
tJsaC836xOjrZ0kK+oy8l4sycp6tERHNi7rTv64YfbmPE0Z77M60c1/KueOYBcKn
srNZZLPrHpxyjmFlToYvj/MpHwKBgQDBAk2DJsINL79+dE2PqUTCX9dq9ixDDQEt
mH2OOQj7Too49tOjvZP/iG5kPQ/Qkfjx2JZeru2xKzxunYa3qvwuHDeJYDvkilxa
G3NEeVZahvdp+ZknmGZKxgaZKgZP04kgW97PAcfFrqjzB8EcajwcjHLue2Qg5162
ceihyBeqQwKBgEpu5X3fWb3Wb4nUR79KU3PuGtmnHLCYkHi+Ji2r1BWCOgyUREVe
VQLtTyKUBPuIdsKPOJFHBTI4mwsuuKm7JAuiQe9qmYJV9G4NfR4V1nnYgdv+NzUM
NhP0BpqMYcwT0da1eA6FUTH+iBsh43rGVyzOTEet1kvVgEuo1w7BIgdDAoGAQkcx
KO1hS7fu0VTM4Z1l0D2rMr7QWkIX+nlX/EPXsry4uHECIkNSlDhceC2DxcKqsxoG
IQN++gz31qBfh6i+qnLkG1ehmYxtxD+S6JumLLYWNh0RG8i4r8qqr2QAAN+KQkNq
ErnwyRB+Ud6C0OgmNkOAoCZdLvNk0c/x68RTZBMCgYEAxXsNZwPZQBeQIjLZQeiR
3N1PS33NB4HcQP8K+wYLbW0PvjxeXUpMit2RmkKi4fFLX0rO7Huwa0rwJLPksJdy
szbJbBstFz1BZ8nwpJp1m/Ntqja3n74mp4MwSr6au1Db1SVJAOisMRZ3oIXuYI6m
C+AKS63xSUuh0BRfCg6QHGA=
-----END PRIVATE KEY-----"#;

/// Test RSA public key for JWT verification
const TEST_PUBLIC_KEY: &str = r#"-----BEGIN PUBLIC KEY-----
MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAmswJ4qtDi4krAjoUPh1c
qba8DBGlg+WCc89iPsowhXC0VnEN9I/cZ8mTvUcbdpWL3qpR9AO9/sN0rfpc2Zob
Nx566XVlCD4BcQdhIj/R3+rctv3tvQncQAlD8e2hoeTNlYgEjnc5HhVD2DThZGLs
WUxjRjEx9ic08D6QGr73F5mffeDjvwScduSAYQ0ivrID4IdTXHooImpHy+i8E8CH
np5D1WrrPRotRotlK5i94a/6OTDL+DQHDfpwMyL2R1ZcpDp9XIuj5vd/Sw0mFolW
VKI+1tHRXupJS/V7J1mlETrG+VvSECpcCQzHwrOxRw4xET6DQlcEXff1RI+CD7tZ
HQIDAQAB
-----END PUBLIC KEY-----"#;

pub const TEST_ADMIN_API_KEY: &str = "test-admin-key-12345";

/// Test application with running gRPC server.
pub struct TestApp {
    pub grpc_port: u16,
    pub state: AppState,
    _key_files: (NamedTempFile, NamedTempFile),
}

impl TestApp {
    /// Spawn the test application with a fresh database.
    pub async fn spawn() -> Self {
        let (private_file, public_file) = create_test_keys().expect("Failed to create test keys");
        let pool = create_test_pool()
            .await
            .expect("Failed to create test pool");

        // Clean up any existing test data
        cleanup_test_data(&pool)
            .await
            .expect("Failed to cleanup test data");

        let config = create_test_config(
            private_file.path().to_str().unwrap(),
            public_file.path().to_str().unwrap(),
        );

        let database = services::Database::new(pool);
        let jwt = services::JwtService::new(&config.jwt).expect("Failed to create JWT service");
        let redis = Arc::new(services::MockBlacklist::new()) as Arc<dyn services::TokenBlacklist>;
        let email = Arc::new(services::MockEmailService) as Arc<dyn services::EmailProvider>;

        let state = AppState {
            config: config.clone(),
            db: database,
            email,
            jwt,
            redis,
        };

        // Find an available port for gRPC
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("Failed to bind to random port");
        let grpc_port = listener.local_addr().unwrap().port();
        drop(listener);

        let grpc_addr = SocketAddr::from(([127, 0, 0, 1], grpc_port));

        // Build gRPC services
        let admin_service = auth_service::grpc::AdminServiceImpl::new(state.clone());
        let auth_svc = auth_service::grpc::AuthServiceImpl::new(state.clone());
        let authz_service = auth_service::grpc::AuthzServiceImpl::new(state.clone());
        let org_service = auth_service::grpc::OrgServiceImpl::new(state.clone());
        let role_service = auth_service::grpc::RoleServiceImpl::new(state.clone());
        let assignment_service = auth_service::grpc::AssignmentServiceImpl::new(state.clone());
        let invitation_service = auth_service::grpc::InvitationServiceImpl::new(state.clone());
        let visibility_service = auth_service::grpc::VisibilityServiceImpl::new(state.clone());
        let audit_service = auth_service::grpc::AuditServiceImpl::new(state.clone());

        use auth_service::grpc::proto::auth::{
            admin_service_server::AdminServiceServer,
            assignment_service_server::AssignmentServiceServer,
            audit_service_server::AuditServiceServer, auth_service_server::AuthServiceServer,
            authz_service_server::AuthzServiceServer,
            invitation_service_server::InvitationServiceServer,
            org_service_server::OrgServiceServer, role_service_server::RoleServiceServer,
            visibility_service_server::VisibilityServiceServer,
        };

        let grpc_server = tonic::transport::Server::builder()
            .add_service(AdminServiceServer::new(admin_service))
            .add_service(AuthServiceServer::new(auth_svc))
            .add_service(AuthzServiceServer::new(authz_service))
            .add_service(OrgServiceServer::new(org_service))
            .add_service(RoleServiceServer::new(role_service))
            .add_service(AssignmentServiceServer::new(assignment_service))
            .add_service(InvitationServiceServer::new(invitation_service))
            .add_service(VisibilityServiceServer::new(visibility_service))
            .add_service(AuditServiceServer::new(audit_service))
            .serve(grpc_addr);

        // Spawn the server in the background
        tokio::spawn(async move {
            let _ = grpc_server.await;
        });

        // Wait for server to start
        tokio::time::sleep(Duration::from_millis(100)).await;

        TestApp {
            grpc_port,
            state,
            _key_files: (private_file, public_file),
        }
    }

    /// Get the gRPC endpoint address.
    pub fn grpc_addr(&self) -> String {
        format!("http://127.0.0.1:{}", self.grpc_port)
    }

    /// Create an AdminService client.
    pub async fn admin_client(&self) -> AdminServiceClient<Channel> {
        create_admin_client(self.grpc_port).await
    }

    /// Create an AuthService client.
    pub async fn auth_client(&self) -> AuthServiceClient<Channel> {
        create_auth_client(self.grpc_port).await
    }

    /// Create an AuthzService client.
    pub async fn authz_client(&self) -> AuthzServiceClient<Channel> {
        create_authz_client(self.grpc_port).await
    }

    /// Create an OrgService client.
    pub async fn org_client(&self) -> OrgServiceClient<Channel> {
        create_org_client(self.grpc_port).await
    }

    /// Create a RoleService client.
    pub async fn role_client(&self) -> RoleServiceClient<Channel> {
        create_role_client(self.grpc_port).await
    }

    /// Clean up test data.
    pub async fn cleanup(&self) -> anyhow::Result<()> {
        cleanup_test_data(self.state.db.pool()).await
    }
}

/// Create temporary JWT key files for testing.
pub fn create_test_keys() -> anyhow::Result<(NamedTempFile, NamedTempFile)> {
    let mut private_file = NamedTempFile::new()?;
    private_file.write_all(TEST_PRIVATE_KEY.as_bytes())?;

    let mut public_file = NamedTempFile::new()?;
    public_file.write_all(TEST_PUBLIC_KEY.as_bytes())?;

    Ok((private_file, public_file))
}

/// Get the database URL for testing from environment or use default.
pub fn get_test_database_url() -> String {
    std::env::var("TEST_DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:pass%40word1@localhost:5432/auth_test".to_string())
}

/// Create a test database pool.
pub async fn create_test_pool() -> anyhow::Result<PgPool> {
    let config = DatabaseConfig {
        url: get_test_database_url(),
        max_connections: 5,
        min_connections: 1,
    };

    let pool = db::create_pool(&config).await?;
    db::run_migrations(&pool).await?;

    Ok(pool)
}

/// Create a test configuration.
pub fn create_test_config(private_key_path: &str, public_key_path: &str) -> AuthConfig {
    AuthConfig {
        common: service_core::config::Config { port: 0 },
        environment: Environment::Dev,
        service_name: "auth-service-test".to_string(),
        service_version: "0.1.0".to_string(),
        log_level: "debug".to_string(),
        otlp_endpoint: None,
        database: DatabaseConfig {
            url: get_test_database_url(),
            max_connections: 5,
            min_connections: 1,
        },
        redis: RedisConfig {
            url: "redis://localhost:6379".to_string(),
        },
        jwt: JwtConfig {
            private_key_path: private_key_path.to_string(),
            public_key_path: public_key_path.to_string(),
            access_token_expiry_minutes: 15,
            refresh_token_expiry_days: 7,
            app_token_expiry_minutes: 60,
        },
        google: GoogleOAuthConfig {
            client_id: "test-client-id".to_string(),
            client_secret: "test-client-secret".to_string(),
            redirect_uri: "http://localhost:3000/auth/callback".to_string(),
            frontend_url: "http://localhost:3000".to_string(),
        },
        gmail: GmailConfig {
            user: "test@gmail.com".to_string(),
            app_password: "test-password".to_string(),
        },
        notification: NotificationServiceConfig {
            url: "http://localhost:8080".to_string(),
            enabled: false,
            timeout_seconds: 30,
        },
        security: SecurityConfig {
            allowed_origins: vec!["http://localhost:3000".to_string()],
            require_signatures: false,
            admin_api_key: TEST_ADMIN_API_KEY.to_string(),
            signature_config: service_core::middleware::signature::SignatureConfig {
                require_signatures: false,
                excluded_paths: vec!["/health".to_string()],
            },
        },
        swagger: SwaggerConfig {
            enabled: SwaggerMode::Disabled,
        },
        rate_limit: RateLimitConfig {
            login_attempts: 100,
            login_window_seconds: 60,
            register_attempts: 100,
            register_window_seconds: 60,
            password_reset_attempts: 100,
            password_reset_window_seconds: 60,
            global_ip_limit: 1000,
            global_ip_window_seconds: 60,
            app_token_limit: 100,
            app_token_window_seconds: 60,
        },
    }
}

/// Create a test application state with mock services.
pub async fn create_test_state(pool: PgPool) -> anyhow::Result<AppState> {
    let (private_file, public_file) = create_test_keys()?;
    let config = create_test_config(
        private_file.path().to_str().unwrap(),
        public_file.path().to_str().unwrap(),
    );

    let database = services::Database::new(pool);
    let jwt = services::JwtService::new(&config.jwt)?;
    let redis = Arc::new(services::MockBlacklist::new()) as Arc<dyn services::TokenBlacklist>;
    let email = Arc::new(services::MockEmailService) as Arc<dyn services::EmailProvider>;

    // Keep the temp files alive by leaking them (they'll be cleaned up when the process exits)
    std::mem::forget(private_file);
    std::mem::forget(public_file);

    Ok(AppState {
        config,
        db: database,
        email,
        jwt,
        redis,
    })
}

/// Clean up test data from the database.
pub async fn cleanup_test_data(pool: &PgPool) -> anyhow::Result<()> {
    // Delete in order respecting foreign key constraints
    sqlx::query("DELETE FROM audit_events")
        .execute(pool)
        .await?;
    sqlx::query("DELETE FROM invitations").execute(pool).await?;
    sqlx::query("DELETE FROM visibility_grants")
        .execute(pool)
        .await?;
    sqlx::query("DELETE FROM org_assignments")
        .execute(pool)
        .await?;
    sqlx::query("DELETE FROM role_capabilities")
        .execute(pool)
        .await?;
    sqlx::query("DELETE FROM roles").execute(pool).await?;
    sqlx::query("DELETE FROM org_node_paths")
        .execute(pool)
        .await?;
    sqlx::query("DELETE FROM org_nodes").execute(pool).await?;
    sqlx::query("DELETE FROM otp_codes").execute(pool).await?;
    sqlx::query("DELETE FROM refresh_sessions")
        .execute(pool)
        .await?;
    sqlx::query("DELETE FROM user_identities")
        .execute(pool)
        .await?;
    sqlx::query("DELETE FROM users").execute(pool).await?;
    sqlx::query("DELETE FROM tenants").execute(pool).await?;

    Ok(())
}

// ============================================================================
// gRPC Client Helpers
// ============================================================================

async fn connect_with_retry(addr: String) -> Channel {
    for _ in 0..10 {
        match Channel::from_shared(addr.clone()).unwrap().connect().await {
            Ok(channel) => return channel,
            Err(_) => {
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        }
    }
    panic!("Failed to connect to gRPC server at {}", addr);
}

pub async fn create_admin_client(port: u16) -> AdminServiceClient<Channel> {
    let addr = format!("http://127.0.0.1:{}", port);
    let channel = connect_with_retry(addr).await;
    AdminServiceClient::new(channel)
}

pub async fn create_auth_client(port: u16) -> AuthServiceClient<Channel> {
    let addr = format!("http://127.0.0.1:{}", port);
    let channel = connect_with_retry(addr).await;
    AuthServiceClient::new(channel)
}

pub async fn create_authz_client(port: u16) -> AuthzServiceClient<Channel> {
    let addr = format!("http://127.0.0.1:{}", port);
    let channel = connect_with_retry(addr).await;
    AuthzServiceClient::new(channel)
}

pub async fn create_org_client(port: u16) -> OrgServiceClient<Channel> {
    let addr = format!("http://127.0.0.1:{}", port);
    let channel = connect_with_retry(addr).await;
    OrgServiceClient::new(channel)
}

pub async fn create_role_client(port: u16) -> RoleServiceClient<Channel> {
    let addr = format!("http://127.0.0.1:{}", port);
    let channel = connect_with_retry(addr).await;
    RoleServiceClient::new(channel)
}

/// Add authorization header to a request.
pub fn with_auth<T>(mut request: tonic::Request<T>, token: &str) -> tonic::Request<T> {
    request.metadata_mut().insert(
        "authorization",
        format!("Bearer {}", token).parse().unwrap(),
    );
    request
}

/// Add admin API key header to a request.
pub fn with_admin_key<T>(mut request: tonic::Request<T>) -> tonic::Request<T> {
    request
        .metadata_mut()
        .insert("x-admin-api-key", TEST_ADMIN_API_KEY.parse().unwrap());
    request
}
