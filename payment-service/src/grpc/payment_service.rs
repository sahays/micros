//! gRPC implementation of PaymentService.

use crate::grpc::capability_check::{capabilities, CapabilityMetadata};
use crate::grpc::proto::{
    payment_service_server::PaymentService, CreateRazorpayOrderRequest,
    CreateRazorpayOrderResponse, CreateTransactionRequest, CreateTransactionResponse,
    GenerateUpiQrRequest, GenerateUpiQrResponse, GetTransactionRequest, GetTransactionResponse,
    HandleRazorpayWebhookRequest, HandleRazorpayWebhookResponse, ListTransactionsRequest,
    ListTransactionsResponse, Transaction as ProtoTransaction,
    TransactionStatus as ProtoTransactionStatus, UpdateTransactionStatusRequest,
    UpdateTransactionStatusResponse, VerifyRazorpayPaymentRequest, VerifyRazorpayPaymentResponse,
};
use crate::middleware::TenantContext;
use crate::models::Transaction;
use crate::models::TransactionStatus;
use crate::services::metrics::{record_amount, record_transaction};
use crate::services::razorpay::PaymentVerification;
use crate::startup::AppState;
use mongodb::bson::DateTime;
use prost_types::Timestamp;
use tonic::{Request, Response, Status};
use uuid::Uuid;

pub struct PaymentGrpcService {
    state: AppState,
}

impl PaymentGrpcService {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }

    /// Extract tenant context from gRPC metadata.
    /// Note: tonic::Status is 176 bytes but is the standard gRPC error type.
    /// Boxing would make this non-idiomatic for tonic-based services.
    #[allow(clippy::result_large_err)]
    fn extract_tenant_context(
        request: &Request<impl std::any::Any>,
    ) -> Result<TenantContext, Status> {
        let metadata = request.metadata();

        let app_id = metadata
            .get("x-app-id")
            .and_then(|v| v.to_str().ok())
            .map(String::from)
            .ok_or_else(|| Status::unauthenticated("Missing x-app-id header"))?;

        let org_id = metadata
            .get("x-org-id")
            .and_then(|v| v.to_str().ok())
            .map(String::from)
            .ok_or_else(|| Status::unauthenticated("Missing x-org-id header"))?;

        let user_id = metadata
            .get("x-user-id")
            .and_then(|v| v.to_str().ok())
            .map(String::from);

        Ok(TenantContext::new(app_id, org_id, user_id))
    }

    /// Helper to update transaction by Razorpay order ID.
    async fn update_transaction_by_order_id(
        &self,
        order_id: &str,
        status: TransactionStatus,
    ) -> anyhow::Result<()> {
        use mongodb::bson::doc;

        let filter = doc! { "provider_order_id": order_id };
        let update = doc! {
            "$set": {
                "status": mongodb::bson::to_bson(&status)?,
                "updated_at": mongodb::bson::DateTime::now()
            }
        };

        self.state
            .db
            .collection::<Transaction>("transactions")
            .update_one(filter, update, None)
            .await?;

        tracing::info!(
            order_id = %order_id,
            status = ?status,
            "Transaction updated via webhook"
        );

        Ok(())
    }
}

/// Convert MongoDB DateTime to protobuf Timestamp.
fn datetime_to_timestamp(dt: DateTime) -> Option<Timestamp> {
    let millis = dt.timestamp_millis();
    Some(Timestamp {
        seconds: millis / 1000,
        nanos: ((millis % 1000) * 1_000_000) as i32,
    })
}

/// Convert model Transaction to proto Transaction.
fn transaction_to_proto(t: Transaction) -> ProtoTransaction {
    ProtoTransaction {
        id: t.id.to_string(),
        app_id: t.app_id,
        org_id: t.org_id,
        user_id: t.user_id,
        amount: t.amount,
        currency: t.currency,
        status: status_to_proto(t.status).into(),
        provider_order_id: t.provider_order_id,
        created_at: datetime_to_timestamp(t.created_at),
        updated_at: datetime_to_timestamp(t.updated_at),
    }
}

/// Convert model TransactionStatus to proto TransactionStatus.
fn status_to_proto(status: TransactionStatus) -> ProtoTransactionStatus {
    match status {
        TransactionStatus::Created => ProtoTransactionStatus::Created,
        TransactionStatus::Pending => ProtoTransactionStatus::Pending,
        TransactionStatus::Completed => ProtoTransactionStatus::Completed,
        TransactionStatus::Failed => ProtoTransactionStatus::Failed,
        TransactionStatus::Refunded => ProtoTransactionStatus::Refunded,
    }
}

/// Convert proto TransactionStatus to model TransactionStatus.
fn proto_to_status(status: i32) -> Option<TransactionStatus> {
    match ProtoTransactionStatus::try_from(status) {
        Ok(ProtoTransactionStatus::Created) => Some(TransactionStatus::Created),
        Ok(ProtoTransactionStatus::Pending) => Some(TransactionStatus::Pending),
        Ok(ProtoTransactionStatus::Completed) => Some(TransactionStatus::Completed),
        Ok(ProtoTransactionStatus::Failed) => Some(TransactionStatus::Failed),
        Ok(ProtoTransactionStatus::Refunded) => Some(TransactionStatus::Refunded),
        _ => None,
    }
}

#[tonic::async_trait]
impl PaymentService for PaymentGrpcService {
    async fn create_transaction(
        &self,
        request: Request<CreateTransactionRequest>,
    ) -> Result<Response<CreateTransactionResponse>, Status> {
        // Check capability
        if let Some(metadata) = CapabilityMetadata::try_from_request(&request) {
            self.state
                .capability_checker
                .require_capability_from_metadata(
                    &metadata,
                    capabilities::PAYMENT_TRANSACTION_CREATE,
                )
                .await?;
        }

        let tenant = Self::extract_tenant_context(&request)?;
        let req = request.into_inner();

        let now = DateTime::now();
        let transaction_id = Uuid::new_v4().to_string();
        let transaction = Transaction {
            id: transaction_id.clone(),
            app_id: tenant.app_id.clone(),
            org_id: tenant.org_id.clone(),
            user_id: tenant.user_id.clone(),
            amount: req.amount,
            currency: req.currency,
            status: TransactionStatus::Created,
            provider_order_id: None,
            created_at: now,
            updated_at: now,
        };

        tracing::info!(
            transaction_id = %transaction_id,
            app_id = %tenant.app_id,
            org_id = %tenant.org_id,
            amount = req.amount,
            "Creating transaction via gRPC"
        );

        self.state
            .repository
            .create_transaction(transaction.clone())
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to create transaction");
                Status::internal("Failed to create transaction")
            })?;

        // Record metering for billing
        record_transaction(&tenant.app_id, "created");
        record_amount(
            &tenant.app_id,
            &transaction.currency,
            (transaction.amount * 100.0) as u64, // Convert to smallest unit (paise/cents)
        );

        Ok(Response::new(CreateTransactionResponse {
            transaction: Some(transaction_to_proto(transaction)),
        }))
    }

    async fn get_transaction(
        &self,
        request: Request<GetTransactionRequest>,
    ) -> Result<Response<GetTransactionResponse>, Status> {
        // Check capability
        if let Some(metadata) = CapabilityMetadata::try_from_request(&request) {
            self.state
                .capability_checker
                .require_capability_from_metadata(&metadata, capabilities::PAYMENT_TRANSACTION_READ)
                .await?;
        }

        let tenant = Self::extract_tenant_context(&request)?;
        let req = request.into_inner();

        // Validate UUID format
        let _uuid = Uuid::parse_str(&req.transaction_id)
            .map_err(|_| Status::invalid_argument("Invalid transaction ID"))?;

        tracing::info!(
            transaction_id = %req.transaction_id,
            app_id = %tenant.app_id,
            org_id = %tenant.org_id,
            "Fetching transaction via gRPC"
        );

        let transaction = self
            .state
            .repository
            .get_transaction_in_tenant(&tenant.app_id, &tenant.org_id, &req.transaction_id)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to fetch transaction");
                Status::internal("Failed to fetch transaction")
            })?
            .ok_or_else(|| Status::not_found("Transaction not found"))?;

        Ok(Response::new(GetTransactionResponse {
            transaction: Some(transaction_to_proto(transaction)),
        }))
    }

    async fn update_transaction_status(
        &self,
        request: Request<UpdateTransactionStatusRequest>,
    ) -> Result<Response<UpdateTransactionStatusResponse>, Status> {
        // Check capability
        if let Some(metadata) = CapabilityMetadata::try_from_request(&request) {
            self.state
                .capability_checker
                .require_capability_from_metadata(
                    &metadata,
                    capabilities::PAYMENT_TRANSACTION_UPDATE,
                )
                .await?;
        }

        let tenant = Self::extract_tenant_context(&request)?;
        let req = request.into_inner();

        // Validate UUID format
        let _uuid = Uuid::parse_str(&req.transaction_id)
            .map_err(|_| Status::invalid_argument("Invalid transaction ID"))?;

        let new_status = proto_to_status(req.status)
            .ok_or_else(|| Status::invalid_argument("Invalid status"))?;

        tracing::info!(
            transaction_id = %req.transaction_id,
            app_id = %tenant.app_id,
            org_id = %tenant.org_id,
            new_status = ?new_status,
            "Updating transaction status via gRPC"
        );

        // Verify transaction exists within tenant scope
        let _transaction = self
            .state
            .repository
            .get_transaction_in_tenant(&tenant.app_id, &tenant.org_id, &req.transaction_id)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to fetch transaction");
                Status::internal("Failed to fetch transaction")
            })?
            .ok_or_else(|| Status::not_found("Transaction not found"))?;

        self.state
            .repository
            .update_transaction_status_in_tenant(
                &tenant.app_id,
                &tenant.org_id,
                &req.transaction_id,
                new_status.clone(),
            )
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to update transaction status");
                Status::internal("Failed to update transaction status")
            })?;

        // Record metering for status change
        let status_str = match new_status {
            TransactionStatus::Created => "created",
            TransactionStatus::Pending => "pending",
            TransactionStatus::Completed => "completed",
            TransactionStatus::Failed => "failed",
            TransactionStatus::Refunded => "refunded",
        };
        record_transaction(&tenant.app_id, status_str);

        Ok(Response::new(UpdateTransactionStatusResponse {}))
    }

    async fn list_transactions(
        &self,
        request: Request<ListTransactionsRequest>,
    ) -> Result<Response<ListTransactionsResponse>, Status> {
        // Check capability
        if let Some(metadata) = CapabilityMetadata::try_from_request(&request) {
            self.state
                .capability_checker
                .require_capability_from_metadata(&metadata, capabilities::PAYMENT_TRANSACTION_READ)
                .await?;
        }

        let tenant = Self::extract_tenant_context(&request)?;
        let req = request.into_inner();

        let status_filter = req.status.and_then(proto_to_status);

        let limit = req.limit.clamp(1, 100) as i64;
        let offset = req.offset.max(0) as u64;

        tracing::info!(
            app_id = %tenant.app_id,
            org_id = %tenant.org_id,
            status_filter = ?status_filter,
            limit = limit,
            offset = offset,
            "Listing transactions via gRPC"
        );

        let (transactions, total_count) = self
            .state
            .repository
            .list_transactions_in_tenant(
                &tenant.app_id,
                &tenant.org_id,
                status_filter,
                limit,
                offset,
            )
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to list transactions");
                Status::internal("Failed to list transactions")
            })?;

        let proto_transactions: Vec<ProtoTransaction> =
            transactions.into_iter().map(transaction_to_proto).collect();

        Ok(Response::new(ListTransactionsResponse {
            transactions: proto_transactions,
            total_count,
        }))
    }

    async fn create_razorpay_order(
        &self,
        request: Request<CreateRazorpayOrderRequest>,
    ) -> Result<Response<CreateRazorpayOrderResponse>, Status> {
        // Check capability
        if let Some(metadata) = CapabilityMetadata::try_from_request(&request) {
            self.state
                .capability_checker
                .require_capability_from_metadata(&metadata, capabilities::PAYMENT_RAZORPAY_CREATE)
                .await?;
        }

        let tenant = Self::extract_tenant_context(&request)?;
        let req = request.into_inner();

        tracing::info!(
            app_id = %tenant.app_id,
            org_id = %tenant.org_id,
            amount = req.amount,
            currency = %req.currency,
            "Creating Razorpay order via gRPC"
        );

        // Check if Razorpay is configured
        if !self.state.razorpay.is_configured() {
            return Err(Status::failed_precondition(
                "Razorpay is not configured for this environment",
            ));
        }

        // Parse notes JSON if provided
        let notes: Option<serde_json::Value> = req
            .notes_json
            .as_ref()
            .and_then(|json| serde_json::from_str(json).ok());

        // Create Razorpay order
        let razorpay_order = self
            .state
            .razorpay
            .create_order(req.amount, &req.currency, req.receipt.clone(), notes)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to create Razorpay order");
                Status::internal(format!("Failed to create payment order: {}", e))
            })?;

        // Create local transaction record
        let now = DateTime::now();
        let transaction_id = Uuid::new_v4().to_string();
        let transaction = Transaction {
            id: transaction_id.clone(),
            app_id: tenant.app_id.clone(),
            org_id: tenant.org_id.clone(),
            user_id: tenant.user_id.clone(),
            amount: req.amount as f64 / 100.0, // Convert from paise to rupees
            currency: req.currency.clone(),
            status: TransactionStatus::Created,
            provider_order_id: Some(razorpay_order.id.clone()),
            created_at: now,
            updated_at: now,
        };

        self.state
            .repository
            .create_transaction(transaction.clone())
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to save transaction");
                Status::internal("Failed to save transaction")
            })?;

        tracing::info!(
            transaction_id = %transaction.id,
            razorpay_order_id = %razorpay_order.id,
            "Razorpay order created successfully via gRPC"
        );

        Ok(Response::new(CreateRazorpayOrderResponse {
            transaction_id: transaction.id.to_string(),
            razorpay_order_id: razorpay_order.id,
            amount: req.amount,
            currency: req.currency,
            razorpay_key_id: self.state.config.razorpay.key_id.clone(),
        }))
    }

    async fn verify_razorpay_payment(
        &self,
        request: Request<VerifyRazorpayPaymentRequest>,
    ) -> Result<Response<VerifyRazorpayPaymentResponse>, Status> {
        // Check capability
        if let Some(metadata) = CapabilityMetadata::try_from_request(&request) {
            self.state
                .capability_checker
                .require_capability_from_metadata(&metadata, capabilities::PAYMENT_RAZORPAY_VERIFY)
                .await?;
        }

        let tenant = Self::extract_tenant_context(&request)?;
        let req = request.into_inner();

        // Validate UUID format
        let _uuid = Uuid::parse_str(&req.transaction_id)
            .map_err(|_| Status::invalid_argument("Invalid transaction ID"))?;

        tracing::info!(
            transaction_id = %req.transaction_id,
            razorpay_order_id = %req.razorpay_order_id,
            razorpay_payment_id = %req.razorpay_payment_id,
            app_id = %tenant.app_id,
            org_id = %tenant.org_id,
            "Verifying Razorpay payment via gRPC"
        );

        // Fetch transaction within tenant scope
        let transaction = self
            .state
            .repository
            .get_transaction_in_tenant(&tenant.app_id, &tenant.org_id, &req.transaction_id)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to fetch transaction");
                Status::internal("Failed to fetch transaction")
            })?
            .ok_or_else(|| Status::not_found("Transaction not found"))?;

        // Verify the order ID matches
        if transaction.provider_order_id.as_deref() != Some(&req.razorpay_order_id) {
            tracing::warn!(
                transaction_id = %req.transaction_id,
                expected_order_id = ?transaction.provider_order_id,
                received_order_id = %req.razorpay_order_id,
                "Order ID mismatch"
            );
            return Err(Status::invalid_argument(
                "Order ID does not match transaction",
            ));
        }

        // Verify the signature
        let verification = PaymentVerification {
            razorpay_order_id: req.razorpay_order_id.clone(),
            razorpay_payment_id: req.razorpay_payment_id.clone(),
            razorpay_signature: req.razorpay_signature.clone(),
        };

        let is_valid = self
            .state
            .razorpay
            .verify_payment_signature(&verification)
            .map_err(|e| {
                tracing::error!(error = %e, "Signature verification error");
                Status::internal("Signature verification failed")
            })?;

        let (new_status, message) = if is_valid {
            (
                TransactionStatus::Completed,
                "Payment verified successfully".to_string(),
            )
        } else {
            (
                TransactionStatus::Failed,
                "Payment verification failed - invalid signature".to_string(),
            )
        };

        // Update transaction status
        self.state
            .repository
            .update_transaction_status_in_tenant(
                &tenant.app_id,
                &tenant.org_id,
                &req.transaction_id,
                new_status.clone(),
            )
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to update transaction status");
                Status::internal("Failed to update transaction status")
            })?;

        tracing::info!(
            transaction_id = %req.transaction_id,
            status = ?new_status,
            "Payment verification completed via gRPC"
        );

        Ok(Response::new(VerifyRazorpayPaymentResponse {
            transaction_id: req.transaction_id.clone(),
            status: status_to_proto(new_status).into(),
            razorpay_payment_id: req.razorpay_payment_id,
            message,
        }))
    }

    async fn generate_upi_qr(
        &self,
        request: Request<GenerateUpiQrRequest>,
    ) -> Result<Response<GenerateUpiQrResponse>, Status> {
        // Check capability
        if let Some(metadata) = CapabilityMetadata::try_from_request(&request) {
            self.state
                .capability_checker
                .require_capability_from_metadata(&metadata, capabilities::PAYMENT_UPI_GENERATE)
                .await?;
        }

        let tenant = Self::extract_tenant_context(&request)?;
        let req = request.into_inner();

        tracing::info!(
            app_id = %tenant.app_id,
            org_id = %tenant.org_id,
            amount = req.amount,
            "Generating UPI QR via gRPC"
        );

        // Generate UPI link
        let vpa = req.vpa.unwrap_or_else(|| self.state.config.upi.vpa.clone());
        let merchant_name = req
            .merchant_name
            .unwrap_or_else(|| self.state.config.upi.merchant_name.clone());

        let description = req.description.unwrap_or_default();
        let transaction_note = if description.is_empty() {
            format!("Payment to {}", merchant_name)
        } else {
            description
        };

        // Build UPI link
        let upi_link = format!(
            "upi://pay?pa={}&pn={}&am={:.2}&cu=INR&tn={}",
            urlencoding::encode(&vpa),
            urlencoding::encode(&merchant_name),
            req.amount,
            urlencoding::encode(&transaction_note)
        );

        // Generate QR code as base64 if image generation is available
        let qr_image_base64 = match crate::utils::generate_qr_base64(&upi_link) {
            Ok(base64) => Some(base64),
            Err(e) => {
                tracing::warn!(error = %e, "Failed to generate QR image");
                None
            }
        };

        Ok(Response::new(GenerateUpiQrResponse {
            upi_link,
            qr_image_base64,
        }))
    }

    async fn handle_razorpay_webhook(
        &self,
        request: Request<HandleRazorpayWebhookRequest>,
    ) -> Result<Response<HandleRazorpayWebhookResponse>, Status> {
        // Check capability (optional - webhooks may not have auth headers)
        if let Some(metadata) = CapabilityMetadata::try_from_request(&request) {
            self.state
                .capability_checker
                .require_capability_from_metadata(&metadata, capabilities::PAYMENT_WEBHOOK_HANDLE)
                .await?;
        }

        let req = request.into_inner();

        tracing::debug!(signature = %req.signature, "Received Razorpay webhook via gRPC");

        // Verify webhook signature
        let is_valid = self
            .state
            .razorpay
            .verify_webhook_signature(&req.body, &req.signature)
            .map_err(|e| {
                tracing::error!(error = %e, "Webhook signature verification error");
                Status::internal("Webhook verification failed")
            })?;

        if !is_valid {
            tracing::warn!("Invalid webhook signature");
            return Err(Status::unauthenticated("Invalid webhook signature"));
        }

        // Parse the webhook event
        let event = self
            .state
            .razorpay
            .parse_webhook_event(&req.body)
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to parse webhook event");
                Status::invalid_argument("Invalid webhook payload")
            })?;

        tracing::info!(
            event_type = %event.event,
            account_id = %event.account_id,
            "Processing Razorpay webhook via gRPC"
        );

        let event_type = event.event.clone();

        // Handle different event types
        match event.event.as_str() {
            "payment.captured" => {
                if let Some(ref payment_entity) = event.payload.payment {
                    let payment = &payment_entity.entity;
                    tracing::info!(
                        payment_id = %payment.id,
                        order_id = ?payment.order_id,
                        amount = payment.amount,
                        status = %payment.status,
                        "Payment captured webhook received"
                    );

                    // Update transaction status if we have an order_id
                    if let Some(ref order_id) = payment.order_id {
                        if let Err(e) = self
                            .update_transaction_by_order_id(order_id, TransactionStatus::Completed)
                            .await
                        {
                            tracing::error!(
                                order_id = %order_id,
                                error = %e,
                                "Failed to update transaction from webhook"
                            );
                        }
                    }
                }
            }
            "payment.failed" => {
                if let Some(ref payment_entity) = event.payload.payment {
                    let payment = &payment_entity.entity;
                    tracing::info!(
                        payment_id = %payment.id,
                        order_id = ?payment.order_id,
                        "Payment failed webhook received"
                    );

                    if let Some(ref order_id) = payment.order_id {
                        if let Err(e) = self
                            .update_transaction_by_order_id(order_id, TransactionStatus::Failed)
                            .await
                        {
                            tracing::error!(
                                order_id = %order_id,
                                error = %e,
                                "Failed to update transaction from webhook"
                            );
                        }
                    }
                }
            }
            "order.paid" => {
                if let Some(ref order_entity) = event.payload.order {
                    let order = &order_entity.entity;
                    tracing::info!(
                        order_id = %order.id,
                        amount = order.amount,
                        "Order paid webhook received"
                    );

                    if let Err(e) = self
                        .update_transaction_by_order_id(&order.id, TransactionStatus::Completed)
                        .await
                    {
                        tracing::error!(
                            order_id = %order.id,
                            error = %e,
                            "Failed to update transaction from webhook"
                        );
                    }
                }
            }
            "refund.created" | "refund.processed" => {
                tracing::info!(event_type = %event.event, "Refund webhook received");
                // Handle refund events - would update transaction to Refunded status
            }
            _ => {
                tracing::debug!(event_type = %event.event, "Unhandled webhook event type");
            }
        }

        Ok(Response::new(HandleRazorpayWebhookResponse {
            success: true,
            event_type,
            message: Some("Webhook processed successfully".to_string()),
        }))
    }
}
