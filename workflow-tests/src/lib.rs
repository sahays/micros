//! Cross-service workflow integration tests library.
//!
//! Provides test infrastructure for running end-to-end tests across multiple microservices.
//! Tests connect to running services via gRPC and verify complete business workflows.
//!
//! ## Usage
//!
//! ```bash
//! # Start all services
//! ./scripts/dev-up.sh
//!
//! # Run workflow tests
//! ./scripts/integ-tests.sh -p workflow-tests
//! ```

use anyhow::{anyhow, Result};
use std::sync::Once;
use std::time::Duration;
use tonic::transport::Channel;
use tonic::Request;
use uuid::Uuid;

// Re-export service clients
// Services in service-core proto module:
pub use service_core::grpc::proto::auth::auth_service_client::AuthServiceClient;
pub use service_core::grpc::proto::auth::authz_service_client::AuthzServiceClient;
pub use service_core::grpc::proto::document::document_service_client::DocumentServiceClient;
pub use service_core::grpc::proto::genai::gen_ai_service_client::GenAiServiceClient;
pub use service_core::grpc::proto::ledger::ledger_service_client::LedgerServiceClient;
pub use service_core::grpc::proto::notification::notification_service_client::NotificationServiceClient;
pub use service_core::grpc::proto::payment::payment_service_client::PaymentServiceClient;

// Services not in service-core (have their own proto modules):
pub use billing_service::grpc::proto::billing_service_client::BillingServiceClient;
pub use invoicing_service::grpc::proto::invoicing_service_client::InvoicingServiceClient;
pub use reconciliation_service::grpc::proto::reconciliation_service_client::ReconciliationServiceClient;

// Re-export proto modules for request/response types
pub mod proto {
    pub mod auth {
        pub use service_core::grpc::proto::auth::*;
    }
    pub mod billing {
        pub use billing_service::grpc::proto::*;
    }
    pub mod document {
        pub use service_core::grpc::proto::document::*;
    }
    pub mod genai {
        pub use service_core::grpc::proto::genai::*;
    }
    pub mod invoicing {
        pub use invoicing_service::grpc::proto::*;
    }
    pub mod ledger {
        pub use service_core::grpc::proto::ledger::*;
    }
    pub mod notification {
        pub use service_core::grpc::proto::notification::*;
    }
    pub mod payment {
        pub use service_core::grpc::proto::payment::*;
    }
    pub mod reconciliation {
        pub use reconciliation_service::grpc::proto::*;
    }
}

static INIT: Once = Once::new();

/// Initialize tracing for tests (only once).
pub fn init_tracing() {
    INIT.call_once(|| {
        tracing_subscriber::fmt()
            .with_env_filter("info,workflow_tests=debug")
            .with_test_writer()
            .try_init()
            .ok();
    });
}

/// Service endpoint configuration from environment variables.
#[derive(Debug, Clone)]
pub struct ServiceEndpoints {
    pub auth: String,
    pub billing: String,
    pub document: String,
    pub genai: String,
    pub invoicing: String,
    pub ledger: String,
    pub notification: String,
    pub payment: String,
    pub reconciliation: String,
}

impl ServiceEndpoints {
    /// Load endpoints from environment variables or use defaults.
    pub fn from_env() -> Self {
        Self {
            auth: std::env::var("AUTH_GRPC_ENDPOINT")
                .unwrap_or_else(|_| "http://localhost:50051".to_string()),
            billing: std::env::var("BILLING_GRPC_ENDPOINT")
                .unwrap_or_else(|_| "http://localhost:50057".to_string()),
            document: std::env::var("DOCUMENT_GRPC_ENDPOINT")
                .unwrap_or_else(|_| "http://localhost:50052".to_string()),
            genai: std::env::var("GENAI_GRPC_ENDPOINT")
                .unwrap_or_else(|_| "http://localhost:50055".to_string()),
            invoicing: std::env::var("INVOICING_GRPC_ENDPOINT")
                .unwrap_or_else(|_| "http://localhost:50059".to_string()),
            ledger: std::env::var("LEDGER_GRPC_ENDPOINT")
                .unwrap_or_else(|_| "http://localhost:50056".to_string()),
            notification: std::env::var("NOTIFICATION_GRPC_ENDPOINT")
                .unwrap_or_else(|_| "http://localhost:50053".to_string()),
            payment: std::env::var("PAYMENT_GRPC_ENDPOINT")
                .unwrap_or_else(|_| "http://localhost:50054".to_string()),
            reconciliation: std::env::var("RECONCILIATION_GRPC_ENDPOINT")
                .unwrap_or_else(|_| "http://localhost:50058".to_string()),
        }
    }

    /// Get health check URLs for all services.
    pub fn health_urls(&self) -> Vec<(&'static str, String)> {
        vec![
            ("auth", std::env::var("AUTH_HEALTH_URL").unwrap_or_else(|_| "http://localhost:9005/health".to_string())),
            ("billing", std::env::var("BILLING_HEALTH_URL").unwrap_or_else(|_| "http://localhost:9012/health".to_string())),
            ("document", std::env::var("DOCUMENT_HEALTH_URL").unwrap_or_else(|_| "http://localhost:9007/health".to_string())),
            ("genai", std::env::var("GENAI_HEALTH_URL").unwrap_or_else(|_| "http://localhost:9010/health".to_string())),
            ("invoicing", std::env::var("INVOICING_HEALTH_URL").unwrap_or_else(|_| "http://localhost:9014/health".to_string())),
            ("ledger", std::env::var("LEDGER_HEALTH_URL").unwrap_or_else(|_| "http://localhost:9011/health".to_string())),
            ("notification", std::env::var("NOTIFICATION_HEALTH_URL").unwrap_or_else(|_| "http://localhost:9008/health".to_string())),
            ("payment", std::env::var("PAYMENT_HEALTH_URL").unwrap_or_else(|_| "http://localhost:9009/health".to_string())),
            ("reconciliation", std::env::var("RECONCILIATION_HEALTH_URL").unwrap_or_else(|_| "http://localhost:9013/health".to_string())),
        ]
    }
}

/// Context for workflow tests with all service clients.
///
/// Each test should create a new context with its own tenant for isolation.
pub struct WorkflowTestContext {
    /// Unique tenant ID for this test
    pub tenant_id: Uuid,
    /// User ID for this test
    pub user_id: Uuid,
    /// Auth token for authenticated requests (if using real auth)
    pub auth_token: Option<String>,

    // Service clients
    pub auth: AuthServiceClient<Channel>,
    pub authz: AuthzServiceClient<Channel>,
    pub billing: BillingServiceClient<Channel>,
    pub document: DocumentServiceClient<Channel>,
    pub genai: GenAiServiceClient<Channel>,
    pub invoicing: InvoicingServiceClient<Channel>,
    pub ledger: LedgerServiceClient<Channel>,
    pub notification: NotificationServiceClient<Channel>,
    pub payment: PaymentServiceClient<Channel>,
    pub reconciliation: ReconciliationServiceClient<Channel>,
}

impl WorkflowTestContext {
    /// Create a new workflow test context connected to all services.
    ///
    /// This creates a unique tenant ID for test isolation but does NOT
    /// bootstrap the tenant in auth-service (tests can do this if needed).
    pub async fn new() -> Result<Self> {
        init_tracing();

        let endpoints = ServiceEndpoints::from_env();

        // Connect to all services
        let auth = AuthServiceClient::connect(endpoints.auth.clone())
            .await
            .map_err(|e| anyhow!("Failed to connect to auth-service: {}", e))?;

        let authz = AuthzServiceClient::connect(endpoints.auth.clone())
            .await
            .map_err(|e| anyhow!("Failed to connect to authz-service: {}", e))?;

        let billing = BillingServiceClient::connect(endpoints.billing.clone())
            .await
            .map_err(|e| anyhow!("Failed to connect to billing-service: {}", e))?;

        let document = DocumentServiceClient::connect(endpoints.document.clone())
            .await
            .map_err(|e| anyhow!("Failed to connect to document-service: {}", e))?;

        let genai = GenAiServiceClient::connect(endpoints.genai.clone())
            .await
            .map_err(|e| anyhow!("Failed to connect to genai-service: {}", e))?;

        let invoicing = InvoicingServiceClient::connect(endpoints.invoicing.clone())
            .await
            .map_err(|e| anyhow!("Failed to connect to invoicing-service: {}", e))?;

        let ledger = LedgerServiceClient::connect(endpoints.ledger.clone())
            .await
            .map_err(|e| anyhow!("Failed to connect to ledger-service: {}", e))?;

        let notification = NotificationServiceClient::connect(endpoints.notification.clone())
            .await
            .map_err(|e| anyhow!("Failed to connect to notification-service: {}", e))?;

        let payment = PaymentServiceClient::connect(endpoints.payment.clone())
            .await
            .map_err(|e| anyhow!("Failed to connect to payment-service: {}", e))?;

        let reconciliation = ReconciliationServiceClient::connect(endpoints.reconciliation.clone())
            .await
            .map_err(|e| anyhow!("Failed to connect to reconciliation-service: {}", e))?;

        Ok(Self {
            tenant_id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            auth_token: None,
            auth,
            authz,
            billing,
            document,
            genai,
            invoicing,
            ledger,
            notification,
            payment,
            reconciliation,
        })
    }

    /// Add authentication headers to a gRPC request.
    ///
    /// If auth_token is set, adds Bearer token.
    /// Always adds tenant_id and user_id for services with disabled capability checking.
    pub fn with_auth<T>(&self, request: T) -> Request<T> {
        let mut req = Request::new(request);

        // Add Bearer token if available
        if let Some(ref token) = self.auth_token {
            req.metadata_mut()
                .insert("authorization", format!("Bearer {}", token).parse().unwrap());
        }

        // Always add tenant and user ID (used when capability checking is disabled)
        req.metadata_mut()
            .insert("x-tenant-id", self.tenant_id.to_string().parse().unwrap());
        req.metadata_mut()
            .insert("x-user-id", self.user_id.to_string().parse().unwrap());

        req
    }

    /// Set the auth token for this context.
    pub fn set_auth_token(&mut self, token: String) {
        self.auth_token = Some(token);
    }
}

/// Wait for all services to be healthy.
///
/// Polls health endpoints until all services respond with 200 OK.
/// Times out after the specified duration.
pub async fn wait_for_services(timeout: Duration) -> Result<()> {
    let endpoints = ServiceEndpoints::from_env();
    let health_urls = endpoints.health_urls();
    let client = reqwest::Client::new();
    let start = std::time::Instant::now();

    tracing::info!("Waiting for {} services to be healthy...", health_urls.len());

    loop {
        let mut all_healthy = true;
        let mut unhealthy_services = Vec::new();

        for (name, url) in &health_urls {
            match client.get(url).timeout(Duration::from_secs(2)).send().await {
                Ok(resp) if resp.status().is_success() => {
                    // Service is healthy
                }
                Ok(resp) => {
                    all_healthy = false;
                    unhealthy_services.push(format!("{} (status: {})", name, resp.status()));
                }
                Err(e) => {
                    all_healthy = false;
                    unhealthy_services.push(format!("{} (error: {})", name, e));
                }
            }
        }

        if all_healthy {
            tracing::info!("All services are healthy");
            return Ok(());
        }

        if start.elapsed() > timeout {
            return Err(anyhow!(
                "Timeout waiting for services. Unhealthy: {}",
                unhealthy_services.join(", ")
            ));
        }

        tracing::debug!("Waiting for services: {}", unhealthy_services.join(", "));
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn service_endpoints_from_env_uses_defaults() {
        let endpoints = ServiceEndpoints::from_env();
        // Just verify it doesn't panic and has reasonable defaults
        assert!(endpoints.auth.contains("50051"));
        assert!(endpoints.billing.contains("50057"));
    }
}
