//! gRPC server builder utilities.
//!
//! Provides a builder pattern for configuring gRPC servers with standard
//! middleware and services (health, reflection).

use std::net::SocketAddr;
use std::time::Duration;

use tonic::transport::Server;
use tonic_reflection::server::Builder as ReflectionBuilder;

/// Builder for configuring a gRPC server with standard middleware.
pub struct GrpcServerBuilder {
    service_name: String,
    enable_reflection: bool,
    enable_health: bool,
    http2_keepalive_interval: Option<Duration>,
    http2_keepalive_timeout: Option<Duration>,
    concurrency_limit: Option<usize>,
}

impl GrpcServerBuilder {
    /// Create a new server builder for the given service name.
    pub fn new(service_name: impl Into<String>) -> Self {
        Self {
            service_name: service_name.into(),
            enable_reflection: true,
            enable_health: true,
            http2_keepalive_interval: Some(Duration::from_secs(30)),
            http2_keepalive_timeout: Some(Duration::from_secs(10)),
            concurrency_limit: None,
        }
    }

    /// Enable or disable gRPC reflection (enabled by default).
    pub fn with_reflection(mut self, enable: bool) -> Self {
        self.enable_reflection = enable;
        self
    }

    /// Enable or disable health check service (enabled by default).
    pub fn with_health(mut self, enable: bool) -> Self {
        self.enable_health = enable;
        self
    }

    /// Set HTTP/2 keepalive interval.
    pub fn with_keepalive_interval(mut self, interval: Duration) -> Self {
        self.http2_keepalive_interval = Some(interval);
        self
    }

    /// Set HTTP/2 keepalive timeout.
    pub fn with_keepalive_timeout(mut self, timeout: Duration) -> Self {
        self.http2_keepalive_timeout = Some(timeout);
        self
    }

    /// Set concurrency limit for the server.
    pub fn with_concurrency_limit(mut self, limit: usize) -> Self {
        self.concurrency_limit = Some(limit);
        self
    }

    /// Get the service name.
    pub fn service_name(&self) -> &str {
        &self.service_name
    }

    /// Check if reflection is enabled.
    pub fn reflection_enabled(&self) -> bool {
        self.enable_reflection
    }

    /// Check if health is enabled.
    pub fn health_enabled(&self) -> bool {
        self.enable_health
    }

    /// Build a tonic Server with the configured settings.
    pub fn build_server(&self) -> tonic::transport::server::Server {
        let mut server = Server::builder();

        if let Some(interval) = self.http2_keepalive_interval {
            server = server.http2_keepalive_interval(Some(interval));
        }

        if let Some(timeout) = self.http2_keepalive_timeout {
            server = server.http2_keepalive_timeout(Some(timeout));
        }

        if let Some(limit) = self.concurrency_limit {
            server = server.concurrency_limit_per_connection(limit);
        }

        server
    }
}

/// Create a reflection service builder.
///
/// # Example
///
/// ```ignore
/// use service_core::grpc::server::create_reflection_service;
///
/// let reflection_service = create_reflection_service(&[
///     micros::auth::v1::FILE_DESCRIPTOR_SET,
/// ])?;
/// ```
pub fn create_reflection_service(
    file_descriptor_sets: &[&[u8]],
) -> Result<
    tonic_reflection::server::ServerReflectionServer<
        impl tonic_reflection::server::ServerReflection,
    >,
    tonic_reflection::server::Error,
> {
    let mut builder = ReflectionBuilder::configure();

    for fds in file_descriptor_sets {
        builder = builder.register_encoded_file_descriptor_set(fds);
    }

    builder.build_v1()
}

/// Start a minimal HTTP health check server for Docker/K8s probes.
///
/// This runs a simple HTTP server that responds with 200 OK to GET /health.
/// Use this alongside gRPC for container orchestration health probes.
///
/// # Example
///
/// ```ignore
/// use service_core::grpc::server::start_http_health_server;
///
/// // Start HTTP health server on port 8080
/// let health_handle = start_http_health_server(8080).await?;
///
/// // Start gRPC server on port 50051
/// Server::builder()
///     .add_service(my_service)
///     .serve("[::]:50051".parse()?)
///     .await?;
/// ```
pub async fn start_http_health_server(
    port: u16,
) -> Result<tokio::task::JoinHandle<()>, std::io::Error> {
    use axum::{Router, routing::get};

    let app = Router::new().route("/health", get(|| async { "OK" }));

    let addr: SocketAddr = format!("0.0.0.0:{}", port).parse().unwrap();

    tracing::info!(port = port, "Starting HTTP health server");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    let handle = tokio::spawn(async move {
        if let Err(e) = axum::serve(listener, app).await {
            tracing::error!(error = %e, "HTTP health server error");
        }
    });

    Ok(handle)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_defaults() {
        let builder = GrpcServerBuilder::new("test-service");
        assert!(builder.reflection_enabled());
        assert!(builder.health_enabled());
    }

    #[test]
    fn test_builder_configuration() {
        let builder = GrpcServerBuilder::new("test-service")
            .with_reflection(false)
            .with_health(false)
            .with_concurrency_limit(100);

        assert!(!builder.reflection_enabled());
        assert!(!builder.health_enabled());
        assert_eq!(builder.concurrency_limit, Some(100));
    }
}
