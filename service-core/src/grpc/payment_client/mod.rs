//! Payment service gRPC client for service-to-service communication.

mod marketplace;
mod subscriptions;
mod transactions;
mod transfers;

use std::time::Duration;
use tonic::Request;
use tonic::transport::{Channel, Endpoint};

use crate::grpc::proto::payment::payment_service_client::PaymentServiceClient;

/// Configuration for the payment service client.
#[derive(Clone, Debug)]
pub struct PaymentClientConfig {
    /// The gRPC endpoint of the payment service (e.g., "http://payment-service:3004").
    pub endpoint: String,
    /// Connection timeout.
    pub connect_timeout: Duration,
    /// Request timeout.
    pub request_timeout: Duration,
}

impl Default for PaymentClientConfig {
    fn default() -> Self {
        Self {
            endpoint: "http://localhost:50054".to_string(),
            connect_timeout: Duration::from_secs(5),
            request_timeout: Duration::from_secs(30),
        }
    }
}

/// Payment service client for calling payment-service via gRPC.
#[derive(Clone)]
pub struct PaymentClient {
    pub(crate) client: PaymentServiceClient<Channel>,
}

impl PaymentClient {
    /// Create a new payment client with the given configuration.
    pub async fn new(config: PaymentClientConfig) -> Result<Self, tonic::transport::Error> {
        let channel = Endpoint::from_shared(config.endpoint)?
            .connect_timeout(config.connect_timeout)
            .timeout(config.request_timeout)
            .connect()
            .await?;

        Ok(Self {
            client: PaymentServiceClient::new(channel),
        })
    }

    /// Create a new payment client connecting to the specified endpoint.
    pub async fn connect(endpoint: &str) -> Result<Self, tonic::transport::Error> {
        Self::new(PaymentClientConfig {
            endpoint: endpoint.to_string(),
            ..Default::default()
        })
        .await
    }

    /// Helper to add tenant context metadata to a request.
    pub(crate) fn add_tenant_context<T>(
        &self,
        mut request: Request<T>,
        app_id: &str,
        tenant_id: &str,
        user_id: Option<&str>,
    ) -> Request<T> {
        request
            .metadata_mut()
            .insert("x-app-id", app_id.parse().unwrap());
        request
            .metadata_mut()
            .insert("x-tenant-id", tenant_id.parse().unwrap());
        if let Some(uid) = user_id {
            request
                .metadata_mut()
                .insert("x-user-id", uid.parse().unwrap());
        }
        request
    }
}
