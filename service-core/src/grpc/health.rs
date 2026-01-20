//! gRPC health check service utilities.
//!
//! Wraps `tonic-health` to provide a simple interface for managing service health status.

use std::sync::Arc;
use tokio::sync::RwLock;
use tonic_health::server::HealthReporter as TonicHealthReporter;

/// Health status for a service.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthStatus {
    /// Service is healthy and ready to accept requests.
    Serving,
    /// Service is not ready to accept requests.
    NotServing,
    /// Health status is unknown.
    Unknown,
}

impl From<HealthStatus> for tonic_health::ServingStatus {
    fn from(status: HealthStatus) -> Self {
        match status {
            HealthStatus::Serving => tonic_health::ServingStatus::Serving,
            HealthStatus::NotServing => tonic_health::ServingStatus::NotServing,
            HealthStatus::Unknown => tonic_health::ServingStatus::Unknown,
        }
    }
}

/// Reporter for updating service health status.
///
/// This wraps `tonic-health`'s `HealthReporter` to provide a simpler interface.
#[derive(Clone)]
pub struct HealthReporter {
    inner: Arc<RwLock<TonicHealthReporter>>,
    service_name: String,
}

impl HealthReporter {
    /// Create a new health reporter for the given service.
    pub fn new(reporter: TonicHealthReporter, service_name: impl Into<String>) -> Self {
        Self {
            inner: Arc::new(RwLock::new(reporter)),
            service_name: service_name.into(),
        }
    }

    /// Set the health status for this service.
    pub async fn set_status(&self, status: HealthStatus) {
        let mut reporter = self.inner.write().await;
        reporter
            .set_service_status(&self.service_name, status.into())
            .await;
    }

    /// Mark the service as serving (healthy).
    pub async fn set_serving(&self) {
        self.set_status(HealthStatus::Serving).await;
    }

    /// Mark the service as not serving (unhealthy).
    pub async fn set_not_serving(&self) {
        self.set_status(HealthStatus::NotServing).await;
    }
}

/// Health service components returned by `create_health_service`.
pub struct HealthComponents<S> {
    /// The health server to add to the gRPC router.
    pub server: tonic_health::pb::health_server::HealthServer<S>,
    /// The reporter for updating health status.
    pub reporter: HealthReporter,
}

/// Create a health service and reporter.
///
/// Returns `HealthComponents` containing:
/// - The `HealthServer` to add to the gRPC server
/// - A `HealthReporter` to update health status
///
/// # Example
///
/// ```ignore
/// use service_core::grpc::health::create_health_service;
///
/// let health = create_health_service("my-service").await;
///
/// // Add health.server to your gRPC server
/// Server::builder()
///     .add_service(health.server)
///     .add_service(my_service)
///     .serve(addr)
///     .await?;
///
/// // Update health status
/// health.reporter.set_serving().await;
/// ```
pub async fn create_health_service(
    service_name: impl Into<String>,
) -> HealthComponents<impl tonic_health::pb::health_server::Health> {
    let service_name = service_name.into();
    let (mut reporter, health_server) = tonic_health::server::health_reporter();

    // Set initial status to serving
    reporter
        .set_service_status(&service_name, tonic_health::ServingStatus::Serving)
        .await;

    let health_reporter = HealthReporter::new(reporter, service_name);
    HealthComponents {
        server: health_server,
        reporter: health_reporter,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_status_conversion() {
        assert_eq!(
            tonic_health::ServingStatus::from(HealthStatus::Serving),
            tonic_health::ServingStatus::Serving
        );
        assert_eq!(
            tonic_health::ServingStatus::from(HealthStatus::NotServing),
            tonic_health::ServingStatus::NotServing
        );
        assert_eq!(
            tonic_health::ServingStatus::from(HealthStatus::Unknown),
            tonic_health::ServingStatus::Unknown
        );
    }
}
