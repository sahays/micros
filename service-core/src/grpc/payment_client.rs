//! Payment service gRPC client for service-to-service communication.

use std::time::Duration;
use tonic::Request;
use tonic::transport::{Channel, Endpoint};

use super::proto::payment::payment_service_client::PaymentServiceClient;
use super::proto::payment::{
    CreateRazorpayOrderRequest, CreateRazorpayOrderResponse, CreateTransactionRequest,
    GenerateUpiQrRequest, GenerateUpiQrResponse, GetTransactionRequest,
    HandleRazorpayWebhookRequest, HandleRazorpayWebhookResponse, ListTransactionsRequest,
    Transaction, TransactionStatus, UpdateTransactionStatusRequest, VerifyRazorpayPaymentRequest,
    VerifyRazorpayPaymentResponse,
};

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
    client: PaymentServiceClient<Channel>,
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
    fn add_tenant_context<T>(
        &self,
        mut request: Request<T>,
        app_id: &str,
        org_id: &str,
        user_id: Option<&str>,
    ) -> Request<T> {
        request
            .metadata_mut()
            .insert("x-app-id", app_id.parse().unwrap());
        request
            .metadata_mut()
            .insert("x-org-id", org_id.parse().unwrap());
        if let Some(uid) = user_id {
            request
                .metadata_mut()
                .insert("x-user-id", uid.parse().unwrap());
        }
        request
    }

    // =========================================================================
    // Transaction Operations
    // =========================================================================

    /// Create a new transaction.
    pub async fn create_transaction(
        &mut self,
        app_id: &str,
        org_id: &str,
        user_id: Option<&str>,
        amount: f64,
        currency: &str,
    ) -> Result<Transaction, tonic::Status> {
        let request = CreateTransactionRequest {
            amount,
            currency: currency.to_string(),
        };

        let request = self.add_tenant_context(Request::new(request), app_id, org_id, user_id);
        let response = self.client.create_transaction(request).await?;

        response
            .into_inner()
            .transaction
            .ok_or_else(|| tonic::Status::internal("Missing transaction in response"))
    }

    /// Get a transaction by ID.
    pub async fn get_transaction(
        &mut self,
        app_id: &str,
        org_id: &str,
        user_id: Option<&str>,
        transaction_id: &str,
    ) -> Result<Transaction, tonic::Status> {
        let request = GetTransactionRequest {
            transaction_id: transaction_id.to_string(),
        };

        let request = self.add_tenant_context(Request::new(request), app_id, org_id, user_id);
        let response = self.client.get_transaction(request).await?;

        response
            .into_inner()
            .transaction
            .ok_or_else(|| tonic::Status::internal("Missing transaction in response"))
    }

    /// Update a transaction's status.
    pub async fn update_transaction_status(
        &mut self,
        app_id: &str,
        org_id: &str,
        user_id: Option<&str>,
        transaction_id: &str,
        status: TransactionStatus,
    ) -> Result<(), tonic::Status> {
        let request = UpdateTransactionStatusRequest {
            transaction_id: transaction_id.to_string(),
            status: status.into(),
        };

        let request = self.add_tenant_context(Request::new(request), app_id, org_id, user_id);
        self.client.update_transaction_status(request).await?;

        Ok(())
    }

    /// List transactions with optional status filter.
    pub async fn list_transactions(
        &mut self,
        app_id: &str,
        org_id: &str,
        user_id: Option<&str>,
        status: Option<TransactionStatus>,
        limit: i32,
        offset: i32,
    ) -> Result<(Vec<Transaction>, i64), tonic::Status> {
        let request = ListTransactionsRequest {
            status: status.map(|s| s.into()),
            limit,
            offset,
        };

        let request = self.add_tenant_context(Request::new(request), app_id, org_id, user_id);
        let response = self.client.list_transactions(request).await?;
        let inner = response.into_inner();

        Ok((inner.transactions, inner.total_count))
    }

    // =========================================================================
    // Razorpay Operations
    // =========================================================================

    /// Create a Razorpay order for payment.
    #[allow(clippy::too_many_arguments)]
    pub async fn create_razorpay_order(
        &mut self,
        app_id: &str,
        org_id: &str,
        user_id: Option<&str>,
        amount: u64,
        currency: &str,
        receipt: Option<String>,
        notes_json: Option<String>,
    ) -> Result<CreateRazorpayOrderResponse, tonic::Status> {
        let request = CreateRazorpayOrderRequest {
            amount,
            currency: currency.to_string(),
            receipt,
            notes_json,
        };

        let request = self.add_tenant_context(Request::new(request), app_id, org_id, user_id);
        let response = self.client.create_razorpay_order(request).await?;

        Ok(response.into_inner())
    }

    /// Verify a Razorpay payment after checkout.
    #[allow(clippy::too_many_arguments)]
    pub async fn verify_razorpay_payment(
        &mut self,
        app_id: &str,
        org_id: &str,
        user_id: Option<&str>,
        transaction_id: &str,
        razorpay_order_id: &str,
        razorpay_payment_id: &str,
        razorpay_signature: &str,
    ) -> Result<VerifyRazorpayPaymentResponse, tonic::Status> {
        let request = VerifyRazorpayPaymentRequest {
            transaction_id: transaction_id.to_string(),
            razorpay_order_id: razorpay_order_id.to_string(),
            razorpay_payment_id: razorpay_payment_id.to_string(),
            razorpay_signature: razorpay_signature.to_string(),
        };

        let request = self.add_tenant_context(Request::new(request), app_id, org_id, user_id);
        let response = self.client.verify_razorpay_payment(request).await?;

        Ok(response.into_inner())
    }

    // =========================================================================
    // UPI Operations
    // =========================================================================

    /// Generate a UPI QR code for payment.
    #[allow(clippy::too_many_arguments)]
    pub async fn generate_upi_qr(
        &mut self,
        app_id: &str,
        org_id: &str,
        user_id: Option<&str>,
        amount: f64,
        description: Option<String>,
        transaction_id: Option<String>,
        vpa: Option<String>,
        merchant_name: Option<String>,
    ) -> Result<GenerateUpiQrResponse, tonic::Status> {
        let request = GenerateUpiQrRequest {
            amount,
            description,
            transaction_id,
            vpa,
            merchant_name,
        };

        let request = self.add_tenant_context(Request::new(request), app_id, org_id, user_id);
        let response = self.client.generate_upi_qr(request).await?;

        Ok(response.into_inner())
    }

    // =========================================================================
    // Webhook Operations (called by BFF to proxy external webhooks)
    // =========================================================================

    /// Handle a Razorpay webhook event proxied from BFF.
    ///
    /// This method is called by the BFF to proxy Razorpay webhook events
    /// to the payment service for processing.
    pub async fn handle_razorpay_webhook(
        &mut self,
        body: &str,
        signature: &str,
    ) -> Result<HandleRazorpayWebhookResponse, tonic::Status> {
        let request = HandleRazorpayWebhookRequest {
            body: body.to_string(),
            signature: signature.to_string(),
        };

        let response = self.client.handle_razorpay_webhook(request).await?;

        Ok(response.into_inner())
    }
}
