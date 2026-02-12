//! gRPC implementation of PaymentService.

use crate::grpc::capability_check::{capabilities, CapabilityMetadata};
use crate::grpc::helpers::{
    extract_tenant_context, proto_to_status, status_to_proto, transaction_to_proto,
};
use crate::grpc::proto::{
    payment_service_server::PaymentService, CancelPaymentLinkRequest, CancelPaymentLinkResponse,
    CancelRazorpaySubscriptionRequest, CancelRazorpaySubscriptionResponse, CreateCustomerRequest,
    CreateCustomerResponse, CreateDirectTransferRequest, CreateDirectTransferResponse,
    CreateLinkedAccountRequest, CreateLinkedAccountResponse, CreatePaymentLinkRequest,
    CreatePaymentLinkResponse, CreateRazorpayOrderRequest, CreateRazorpayOrderResponse,
    CreateRazorpayPlanRequest, CreateRazorpayPlanResponse, CreateRazorpaySubscriptionRequest,
    CreateRazorpaySubscriptionResponse, CreateTransactionRequest, CreateTransactionResponse,
    CreateTransferFromOrderRequest, CreateTransferFromOrderResponse,
    CreateTransferFromPaymentRequest, CreateTransferFromPaymentResponse, GenerateUpiQrRequest,
    GenerateUpiQrResponse, GetCustomerRequest, GetCustomerResponse, GetLinkedAccountRequest,
    GetLinkedAccountResponse, GetPaymentLinkRequest, GetPaymentLinkResponse,
    GetRazorpayPlanRequest, GetRazorpayPlanResponse, GetRazorpaySubscriptionRequest,
    GetRazorpaySubscriptionResponse, GetRefundRequest, GetRefundResponse, GetSettlementRequest,
    GetSettlementResponse, GetTransactionRequest, GetTransactionResponse, GetTransferRequest,
    GetTransferResponse, HandleRazorpayWebhookRequest, HandleRazorpayWebhookResponse,
    HoldTransferSettlementRequest, HoldTransferSettlementResponse, InitiateRefundRequest,
    InitiateRefundResponse, ListCustomersRequest, ListCustomersResponse, ListLinkedAccountsRequest,
    ListLinkedAccountsResponse, ListPaymentLinksRequest, ListPaymentLinksResponse,
    ListRazorpayPlansRequest, ListRazorpayPlansResponse, ListRazorpaySubscriptionsRequest,
    ListRazorpaySubscriptionsResponse, ListRefundsRequest, ListRefundsResponse,
    ListSettlementsRequest, ListSettlementsResponse, ListTransactionsRequest,
    ListTransactionsResponse, ListTransfersRequest, ListTransfersResponse,
    PauseRazorpaySubscriptionRequest, PauseRazorpaySubscriptionResponse,
    RecordDirectUpiPaymentRequest, RecordDirectUpiPaymentResponse, RecordOfflinePaymentRequest,
    RecordOfflinePaymentResponse, ReleaseTransferSettlementRequest,
    ReleaseTransferSettlementResponse, RequestOnDemandSettlementRequest,
    RequestOnDemandSettlementResponse, ResumeRazorpaySubscriptionRequest,
    ResumeRazorpaySubscriptionResponse, ReverseTransferRequest, ReverseTransferResponse,
    UpdateCommissionConfigRequest, UpdateCommissionConfigResponse, UpdateCustomerRequest,
    UpdateCustomerResponse, UpdateLinkedAccountRequest, UpdateLinkedAccountResponse,
    UpdateRazorpaySubscriptionRequest, UpdateRazorpaySubscriptionResponse,
    UpdateTransactionStatusRequest, UpdateTransactionStatusResponse, VerifyRazorpayPaymentRequest,
    VerifyRazorpayPaymentResponse,
};
use crate::models::Transaction;
use crate::models::TransactionStatus;
use crate::services::razorpay::PaymentVerification;
use crate::startup::AppState;
use mongodb::bson::DateTime;
use tonic::{Request, Response, Status};
use uuid::Uuid;

use super::{
    customers, direct_payments, linked_accounts, payment_links, refunds, settlements,
    subscriptions, transfers, webhooks,
};

pub struct PaymentGrpcService {
    state: AppState,
}

impl PaymentGrpcService {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }
}

#[tonic::async_trait]
impl PaymentService for PaymentGrpcService {
    async fn create_transaction(
        &self,
        request: Request<CreateTransactionRequest>,
    ) -> Result<Response<CreateTransactionResponse>, Status> {
        if let Some(metadata) = CapabilityMetadata::try_from_request(&request) {
            self.state
                .capability_checker
                .require_capability_from_metadata(
                    &metadata,
                    capabilities::PAYMENT_TRANSACTION_CREATE,
                )
                .await?;
        }

        let tenant = extract_tenant_context(&request)?;
        let req = request.into_inner();

        if req.amount_paise == 0 {
            return Err(Status::invalid_argument("Amount must be greater than 0"));
        }

        let now = DateTime::now();
        let transaction_id = Uuid::new_v4().to_string();
        let transaction = Transaction {
            id: transaction_id.clone(),
            app_id: tenant.app_id.clone(),
            org_id: tenant.org_id.clone(),
            user_id: tenant.user_id.clone(),
            amount_paise: req.amount_paise,
            currency: req.currency,
            status: TransactionStatus::Created,
            provider_order_id: None,
            linked_account_id: None,
            subscription_id: None,
            payment_link_id: None,
            payment_channel: None,
            payment_method_type: None,
            external_reference: None,
            notes: None,
            created_at: now,
            updated_at: now,
        };

        tracing::info!(
            transaction_id = %transaction_id,
            app_id = %tenant.app_id,
            org_id = %tenant.org_id,
            amount_paise = req.amount_paise,
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

        Ok(Response::new(CreateTransactionResponse {
            transaction: Some(transaction_to_proto(transaction)),
        }))
    }

    async fn get_transaction(
        &self,
        request: Request<GetTransactionRequest>,
    ) -> Result<Response<GetTransactionResponse>, Status> {
        if let Some(metadata) = CapabilityMetadata::try_from_request(&request) {
            self.state
                .capability_checker
                .require_capability_from_metadata(&metadata, capabilities::PAYMENT_TRANSACTION_READ)
                .await?;
        }

        let tenant = extract_tenant_context(&request)?;
        let req = request.into_inner();

        let _uuid = Uuid::parse_str(&req.transaction_id)
            .map_err(|_| Status::invalid_argument("Invalid transaction ID"))?;

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
        if let Some(metadata) = CapabilityMetadata::try_from_request(&request) {
            self.state
                .capability_checker
                .require_capability_from_metadata(
                    &metadata,
                    capabilities::PAYMENT_TRANSACTION_UPDATE,
                )
                .await?;
        }

        let tenant = extract_tenant_context(&request)?;
        let req = request.into_inner();

        let _uuid = Uuid::parse_str(&req.transaction_id)
            .map_err(|_| Status::invalid_argument("Invalid transaction ID"))?;

        let new_status = proto_to_status(req.status)
            .ok_or_else(|| Status::invalid_argument("Invalid status"))?;

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

        Ok(Response::new(UpdateTransactionStatusResponse {}))
    }

    async fn list_transactions(
        &self,
        request: Request<ListTransactionsRequest>,
    ) -> Result<Response<ListTransactionsResponse>, Status> {
        if let Some(metadata) = CapabilityMetadata::try_from_request(&request) {
            self.state
                .capability_checker
                .require_capability_from_metadata(&metadata, capabilities::PAYMENT_TRANSACTION_READ)
                .await?;
        }

        let tenant = extract_tenant_context(&request)?;
        let req = request.into_inner();

        let status_filter = req.status.and_then(proto_to_status);
        let limit = req.limit.clamp(1, 100) as i64;
        let offset = req.offset.max(0) as u64;

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

        Ok(Response::new(ListTransactionsResponse {
            transactions: transactions.into_iter().map(transaction_to_proto).collect(),
            total_count,
        }))
    }

    async fn create_razorpay_order(
        &self,
        request: Request<CreateRazorpayOrderRequest>,
    ) -> Result<Response<CreateRazorpayOrderResponse>, Status> {
        if let Some(metadata) = CapabilityMetadata::try_from_request(&request) {
            self.state
                .capability_checker
                .require_capability_from_metadata(&metadata, capabilities::PAYMENT_RAZORPAY_CREATE)
                .await?;
        }

        let tenant = extract_tenant_context(&request)?;
        let req = request.into_inner();

        if req.amount == 0 {
            return Err(Status::invalid_argument("Amount must be greater than 0"));
        }

        if !self.state.razorpay.is_configured() {
            return Err(Status::failed_precondition(
                "Razorpay is not configured for this environment",
            ));
        }

        let notes: Option<serde_json::Value> = req
            .notes_json
            .as_ref()
            .and_then(|json| serde_json::from_str(json).ok());

        let razorpay_order = self
            .state
            .razorpay
            .create_order(req.amount, &req.currency, req.receipt.clone(), notes)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to create Razorpay order");
                Status::internal(format!("Failed to create payment order: {}", e))
            })?;

        let now = DateTime::now();
        let transaction_id = Uuid::new_v4().to_string();
        let transaction = Transaction {
            id: transaction_id.clone(),
            app_id: tenant.app_id.clone(),
            org_id: tenant.org_id.clone(),
            user_id: tenant.user_id.clone(),
            amount_paise: req.amount,
            currency: req.currency.clone(),
            status: TransactionStatus::Created,
            provider_order_id: Some(razorpay_order.id.clone()),
            linked_account_id: None,
            subscription_id: None,
            payment_link_id: None,
            payment_channel: Some(crate::models::PaymentChannel::Razorpay),
            payment_method_type: None,
            external_reference: None,
            notes: None,
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
        if let Some(metadata) = CapabilityMetadata::try_from_request(&request) {
            self.state
                .capability_checker
                .require_capability_from_metadata(&metadata, capabilities::PAYMENT_RAZORPAY_VERIFY)
                .await?;
        }

        let tenant = extract_tenant_context(&request)?;
        let req = request.into_inner();

        let _uuid = Uuid::parse_str(&req.transaction_id)
            .map_err(|_| Status::invalid_argument("Invalid transaction ID"))?;

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

        if transaction.provider_order_id.as_deref() != Some(&req.razorpay_order_id) {
            return Err(Status::invalid_argument(
                "Order ID does not match transaction",
            ));
        }

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
        if let Some(metadata) = CapabilityMetadata::try_from_request(&request) {
            self.state
                .capability_checker
                .require_capability_from_metadata(&metadata, capabilities::PAYMENT_UPI_GENERATE)
                .await?;
        }

        let tenant = extract_tenant_context(&request)?;
        let req = request.into_inner();

        tracing::info!(
            app_id = %tenant.app_id,
            org_id = %tenant.org_id,
            amount_paise = req.amount_paise,
            "Generating UPI QR via gRPC"
        );

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

        let amount_rupees = req.amount_paise as f64 / 100.0;
        let upi_link = format!(
            "upi://pay?pa={}&pn={}&am={:.2}&cu=INR&tn={}",
            urlencoding::encode(&vpa),
            urlencoding::encode(&merchant_name),
            amount_rupees,
            urlencoding::encode(&transaction_note)
        );

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
        if let Some(metadata) = CapabilityMetadata::try_from_request(&request) {
            self.state
                .capability_checker
                .require_capability_from_metadata(&metadata, capabilities::PAYMENT_WEBHOOK_HANDLE)
                .await?;
        }

        let req = request.into_inner();

        let is_valid = self
            .state
            .razorpay
            .verify_webhook_signature(&req.body, &req.signature)
            .map_err(|e| {
                tracing::error!(error = %e, "Webhook signature verification error");
                Status::internal("Webhook verification failed")
            })?;

        if !is_valid {
            return Err(Status::unauthenticated("Invalid webhook signature"));
        }

        let event = self
            .state
            .razorpay
            .parse_webhook_event(&req.body)
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to parse webhook event");
                Status::invalid_argument("Invalid webhook payload")
            })?;

        tracing::info!(event_type = %event.event, "Processing Razorpay webhook via gRPC");

        let event_type = webhooks::process_webhook_event(&self.state, &event).await?;

        Ok(Response::new(HandleRazorpayWebhookResponse {
            success: true,
            event_type,
            message: Some("Webhook processed successfully".to_string()),
        }))
    }

    // =========================================================================
    // Linked Account Operations (delegated)
    // =========================================================================

    async fn create_linked_account(
        &self,
        request: Request<CreateLinkedAccountRequest>,
    ) -> Result<Response<CreateLinkedAccountResponse>, Status> {
        linked_accounts::create_linked_account(&self.state, request).await
    }

    async fn get_linked_account(
        &self,
        request: Request<GetLinkedAccountRequest>,
    ) -> Result<Response<GetLinkedAccountResponse>, Status> {
        linked_accounts::get_linked_account(&self.state, request).await
    }

    async fn update_linked_account(
        &self,
        request: Request<UpdateLinkedAccountRequest>,
    ) -> Result<Response<UpdateLinkedAccountResponse>, Status> {
        linked_accounts::update_linked_account(&self.state, request).await
    }

    async fn list_linked_accounts(
        &self,
        request: Request<ListLinkedAccountsRequest>,
    ) -> Result<Response<ListLinkedAccountsResponse>, Status> {
        linked_accounts::list_linked_accounts(&self.state, request).await
    }

    async fn update_commission_config(
        &self,
        request: Request<UpdateCommissionConfigRequest>,
    ) -> Result<Response<UpdateCommissionConfigResponse>, Status> {
        linked_accounts::update_commission_config(&self.state, request).await
    }

    // =========================================================================
    // Customer Operations (delegated)
    // =========================================================================

    async fn create_customer(
        &self,
        request: Request<CreateCustomerRequest>,
    ) -> Result<Response<CreateCustomerResponse>, Status> {
        customers::create_customer(&self.state, request).await
    }

    async fn get_customer(
        &self,
        request: Request<GetCustomerRequest>,
    ) -> Result<Response<GetCustomerResponse>, Status> {
        customers::get_customer(&self.state, request).await
    }

    async fn update_customer(
        &self,
        request: Request<UpdateCustomerRequest>,
    ) -> Result<Response<UpdateCustomerResponse>, Status> {
        customers::update_customer(&self.state, request).await
    }

    async fn list_customers(
        &self,
        request: Request<ListCustomersRequest>,
    ) -> Result<Response<ListCustomersResponse>, Status> {
        customers::list_customers(&self.state, request).await
    }

    // =========================================================================
    // Transfer Operations (delegated)
    // =========================================================================

    async fn create_transfer_from_payment(
        &self,
        request: Request<CreateTransferFromPaymentRequest>,
    ) -> Result<Response<CreateTransferFromPaymentResponse>, Status> {
        transfers::create_transfer_from_payment(&self.state, request).await
    }

    async fn create_transfer_from_order(
        &self,
        request: Request<CreateTransferFromOrderRequest>,
    ) -> Result<Response<CreateTransferFromOrderResponse>, Status> {
        transfers::create_transfer_from_order(&self.state, request).await
    }

    async fn create_direct_transfer(
        &self,
        request: Request<CreateDirectTransferRequest>,
    ) -> Result<Response<CreateDirectTransferResponse>, Status> {
        transfers::create_direct_transfer(&self.state, request).await
    }

    async fn reverse_transfer(
        &self,
        request: Request<ReverseTransferRequest>,
    ) -> Result<Response<ReverseTransferResponse>, Status> {
        transfers::reverse_transfer(&self.state, request).await
    }

    async fn get_transfer(
        &self,
        request: Request<GetTransferRequest>,
    ) -> Result<Response<GetTransferResponse>, Status> {
        transfers::get_transfer(&self.state, request).await
    }

    async fn list_transfers(
        &self,
        request: Request<ListTransfersRequest>,
    ) -> Result<Response<ListTransfersResponse>, Status> {
        transfers::list_transfers(&self.state, request).await
    }

    async fn hold_transfer_settlement(
        &self,
        request: Request<HoldTransferSettlementRequest>,
    ) -> Result<Response<HoldTransferSettlementResponse>, Status> {
        transfers::hold_transfer_settlement(&self.state, request).await
    }

    async fn release_transfer_settlement(
        &self,
        request: Request<ReleaseTransferSettlementRequest>,
    ) -> Result<Response<ReleaseTransferSettlementResponse>, Status> {
        transfers::release_transfer_settlement(&self.state, request).await
    }

    // =========================================================================
    // Settlement Operations (delegated)
    // =========================================================================

    async fn request_on_demand_settlement(
        &self,
        request: Request<RequestOnDemandSettlementRequest>,
    ) -> Result<Response<RequestOnDemandSettlementResponse>, Status> {
        settlements::request_on_demand_settlement(&self.state, request).await
    }

    async fn get_settlement(
        &self,
        request: Request<GetSettlementRequest>,
    ) -> Result<Response<GetSettlementResponse>, Status> {
        settlements::get_settlement(&self.state, request).await
    }

    async fn list_settlements(
        &self,
        request: Request<ListSettlementsRequest>,
    ) -> Result<Response<ListSettlementsResponse>, Status> {
        settlements::list_settlements(&self.state, request).await
    }

    // =========================================================================
    // Subscription Operations (delegated)
    // =========================================================================

    async fn create_razorpay_plan(
        &self,
        request: Request<CreateRazorpayPlanRequest>,
    ) -> Result<Response<CreateRazorpayPlanResponse>, Status> {
        subscriptions::create_razorpay_plan(&self.state, request).await
    }

    async fn get_razorpay_plan(
        &self,
        request: Request<GetRazorpayPlanRequest>,
    ) -> Result<Response<GetRazorpayPlanResponse>, Status> {
        subscriptions::get_razorpay_plan(&self.state, request).await
    }

    async fn list_razorpay_plans(
        &self,
        request: Request<ListRazorpayPlansRequest>,
    ) -> Result<Response<ListRazorpayPlansResponse>, Status> {
        subscriptions::list_razorpay_plans(&self.state, request).await
    }

    async fn create_razorpay_subscription(
        &self,
        request: Request<CreateRazorpaySubscriptionRequest>,
    ) -> Result<Response<CreateRazorpaySubscriptionResponse>, Status> {
        subscriptions::create_razorpay_subscription(&self.state, request).await
    }

    async fn get_razorpay_subscription(
        &self,
        request: Request<GetRazorpaySubscriptionRequest>,
    ) -> Result<Response<GetRazorpaySubscriptionResponse>, Status> {
        subscriptions::get_razorpay_subscription(&self.state, request).await
    }

    async fn list_razorpay_subscriptions(
        &self,
        request: Request<ListRazorpaySubscriptionsRequest>,
    ) -> Result<Response<ListRazorpaySubscriptionsResponse>, Status> {
        subscriptions::list_razorpay_subscriptions(&self.state, request).await
    }

    async fn pause_razorpay_subscription(
        &self,
        request: Request<PauseRazorpaySubscriptionRequest>,
    ) -> Result<Response<PauseRazorpaySubscriptionResponse>, Status> {
        subscriptions::pause_razorpay_subscription(&self.state, request).await
    }

    async fn resume_razorpay_subscription(
        &self,
        request: Request<ResumeRazorpaySubscriptionRequest>,
    ) -> Result<Response<ResumeRazorpaySubscriptionResponse>, Status> {
        subscriptions::resume_razorpay_subscription(&self.state, request).await
    }

    async fn cancel_razorpay_subscription(
        &self,
        request: Request<CancelRazorpaySubscriptionRequest>,
    ) -> Result<Response<CancelRazorpaySubscriptionResponse>, Status> {
        subscriptions::cancel_razorpay_subscription(&self.state, request).await
    }

    async fn update_razorpay_subscription(
        &self,
        request: Request<UpdateRazorpaySubscriptionRequest>,
    ) -> Result<Response<UpdateRazorpaySubscriptionResponse>, Status> {
        subscriptions::update_razorpay_subscription(&self.state, request).await
    }

    // =========================================================================
    // Payment Link Operations (delegated)
    // =========================================================================

    async fn create_payment_link(
        &self,
        request: Request<CreatePaymentLinkRequest>,
    ) -> Result<Response<CreatePaymentLinkResponse>, Status> {
        payment_links::create_payment_link(&self.state, request).await
    }

    async fn get_payment_link(
        &self,
        request: Request<GetPaymentLinkRequest>,
    ) -> Result<Response<GetPaymentLinkResponse>, Status> {
        payment_links::get_payment_link(&self.state, request).await
    }

    async fn cancel_payment_link(
        &self,
        request: Request<CancelPaymentLinkRequest>,
    ) -> Result<Response<CancelPaymentLinkResponse>, Status> {
        payment_links::cancel_payment_link(&self.state, request).await
    }

    async fn list_payment_links(
        &self,
        request: Request<ListPaymentLinksRequest>,
    ) -> Result<Response<ListPaymentLinksResponse>, Status> {
        payment_links::list_payment_links(&self.state, request).await
    }

    // =========================================================================
    // Refund Operations (delegated)
    // =========================================================================

    async fn initiate_refund(
        &self,
        request: Request<InitiateRefundRequest>,
    ) -> Result<Response<InitiateRefundResponse>, Status> {
        refunds::initiate_refund(&self.state, request).await
    }

    async fn get_refund(
        &self,
        request: Request<GetRefundRequest>,
    ) -> Result<Response<GetRefundResponse>, Status> {
        refunds::get_refund(&self.state, request).await
    }

    async fn list_refunds(
        &self,
        request: Request<ListRefundsRequest>,
    ) -> Result<Response<ListRefundsResponse>, Status> {
        refunds::list_refunds(&self.state, request).await
    }

    // =========================================================================
    // Direct & Offline Payment Operations (delegated)
    // =========================================================================

    async fn record_direct_upi_payment(
        &self,
        request: Request<RecordDirectUpiPaymentRequest>,
    ) -> Result<Response<RecordDirectUpiPaymentResponse>, Status> {
        direct_payments::record_direct_upi_payment(&self.state, request).await
    }

    async fn record_offline_payment(
        &self,
        request: Request<RecordOfflinePaymentRequest>,
    ) -> Result<Response<RecordOfflinePaymentResponse>, Status> {
        direct_payments::record_offline_payment(&self.state, request).await
    }
}
