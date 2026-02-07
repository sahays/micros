//! Payment service gRPC client for service-to-service communication.

use std::time::Duration;
use tonic::Request;
use tonic::transport::{Channel, Endpoint};

use super::proto::payment::payment_service_client::PaymentServiceClient;
use super::proto::payment::{
    BankAccount,
    CancelPaymentLinkRequest,
    CancelRazorpaySubscriptionRequest,
    CommissionConfig,
    // Customers
    CreateCustomerRequest,
    CreateDirectTransferRequest,
    // Linked Accounts
    CreateLinkedAccountRequest,
    // Payment Links
    CreatePaymentLinkRequest,
    // Transaction
    CreateRazorpayOrderRequest,
    CreateRazorpayOrderResponse,
    // Subscriptions
    CreateRazorpayPlanRequest,
    CreateRazorpaySubscriptionRequest,
    CreateTransactionRequest,
    CreateTransferFromOrderRequest,
    // Transfers
    CreateTransferFromPaymentRequest,
    GenerateUpiQrRequest,
    GenerateUpiQrResponse,
    GetCustomerRequest,
    GetLinkedAccountRequest,
    GetPaymentLinkRequest,
    GetRazorpayPlanRequest,
    GetRazorpaySubscriptionRequest,
    GetRefundRequest,
    GetSettlementRequest,
    GetTransactionRequest,
    GetTransferRequest,
    HandleRazorpayWebhookRequest,
    HandleRazorpayWebhookResponse,
    HoldTransferSettlementRequest,
    // Refunds
    InitiateRefundRequest,
    LegalInfo,
    LinkedAccount,
    LinkedAccountStatus,
    ListCustomersRequest,
    ListLinkedAccountsRequest,
    ListPaymentLinksRequest,
    ListRazorpayPlansRequest,
    ListRazorpaySubscriptionsRequest,
    ListRefundsRequest,
    ListSettlementsRequest,
    ListTransactionsRequest,
    ListTransfersRequest,
    PauseRazorpaySubscriptionRequest,
    PaymentLink,
    PaymentLinkStatus,
    PlanPeriod,
    RazorpayCustomer,
    RazorpayPlan,
    RazorpaySubscription,
    Refund,
    RefundSpeed,
    RefundStatus,
    ReleaseTransferSettlementRequest,
    // Settlements
    RequestOnDemandSettlementRequest,
    ResumeRazorpaySubscriptionRequest,
    ReverseTransferRequest,
    Settlement,
    SettlementStatus,
    SettlementType,
    SubscriptionStatus,
    Transaction,
    TransactionStatus,
    Transfer,
    TransferStatus,
    UpdateCommissionConfigRequest,
    UpdateCustomerRequest,
    UpdateLinkedAccountRequest,
    UpdateRazorpaySubscriptionRequest,
    UpdateTransactionStatusRequest,
    VerifyRazorpayPaymentRequest,
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
        amount_paise: u64,
        currency: &str,
    ) -> Result<Transaction, tonic::Status> {
        let request = CreateTransactionRequest {
            amount_paise,
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
        amount_paise: u64,
        description: Option<String>,
        transaction_id: Option<String>,
        vpa: Option<String>,
        merchant_name: Option<String>,
    ) -> Result<GenerateUpiQrResponse, tonic::Status> {
        let request = GenerateUpiQrRequest {
            amount_paise,
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

    // =========================================================================
    // Linked Account Operations
    // =========================================================================

    /// Create a linked account for marketplace/route payments.
    #[allow(clippy::too_many_arguments)]
    pub async fn create_linked_account(
        &mut self,
        app_id: &str,
        org_id: &str,
        user_id: Option<&str>,
        name: &str,
        email: &str,
        bank_account: BankAccount,
        legal_info: LegalInfo,
        commission: Option<CommissionConfig>,
    ) -> Result<LinkedAccount, tonic::Status> {
        let request = CreateLinkedAccountRequest {
            name: name.to_string(),
            email: email.to_string(),
            bank_account: Some(bank_account),
            legal_info: Some(legal_info),
            commission,
        };

        let request = self.add_tenant_context(Request::new(request), app_id, org_id, user_id);
        let response = self.client.create_linked_account(request).await?;

        response
            .into_inner()
            .linked_account
            .ok_or_else(|| tonic::Status::internal("Missing linked_account in response"))
    }

    /// Get a linked account by ID.
    pub async fn get_linked_account(
        &mut self,
        app_id: &str,
        org_id: &str,
        user_id: Option<&str>,
        linked_account_id: &str,
    ) -> Result<LinkedAccount, tonic::Status> {
        let request = GetLinkedAccountRequest {
            linked_account_id: linked_account_id.to_string(),
        };

        let request = self.add_tenant_context(Request::new(request), app_id, org_id, user_id);
        let response = self.client.get_linked_account(request).await?;

        response
            .into_inner()
            .linked_account
            .ok_or_else(|| tonic::Status::internal("Missing linked_account in response"))
    }

    /// Update a linked account.
    pub async fn update_linked_account(
        &mut self,
        app_id: &str,
        org_id: &str,
        user_id: Option<&str>,
        linked_account_id: &str,
        name: Option<String>,
        email: Option<String>,
    ) -> Result<LinkedAccount, tonic::Status> {
        let request = UpdateLinkedAccountRequest {
            linked_account_id: linked_account_id.to_string(),
            name,
            email,
        };

        let request = self.add_tenant_context(Request::new(request), app_id, org_id, user_id);
        let response = self.client.update_linked_account(request).await?;

        response
            .into_inner()
            .linked_account
            .ok_or_else(|| tonic::Status::internal("Missing linked_account in response"))
    }

    /// List linked accounts with optional status filter.
    pub async fn list_linked_accounts(
        &mut self,
        app_id: &str,
        org_id: &str,
        user_id: Option<&str>,
        status: Option<LinkedAccountStatus>,
        limit: i32,
        offset: i32,
    ) -> Result<(Vec<LinkedAccount>, i64), tonic::Status> {
        let request = ListLinkedAccountsRequest {
            status: status.map(|s| s.into()),
            limit,
            offset,
        };

        let request = self.add_tenant_context(Request::new(request), app_id, org_id, user_id);
        let response = self.client.list_linked_accounts(request).await?;
        let inner = response.into_inner();

        Ok((inner.linked_accounts, inner.total_count))
    }

    /// Update commission configuration for a linked account.
    pub async fn update_commission_config(
        &mut self,
        app_id: &str,
        org_id: &str,
        user_id: Option<&str>,
        linked_account_id: &str,
        commission: CommissionConfig,
    ) -> Result<LinkedAccount, tonic::Status> {
        let request = UpdateCommissionConfigRequest {
            linked_account_id: linked_account_id.to_string(),
            commission: Some(commission),
        };

        let request = self.add_tenant_context(Request::new(request), app_id, org_id, user_id);
        let response = self.client.update_commission_config(request).await?;

        response
            .into_inner()
            .linked_account
            .ok_or_else(|| tonic::Status::internal("Missing linked_account in response"))
    }

    // =========================================================================
    // Customer Operations
    // =========================================================================

    /// Create a customer in Razorpay.
    pub async fn create_customer(
        &mut self,
        app_id: &str,
        org_id: &str,
        user_id: Option<&str>,
        name: &str,
        email: &str,
        phone: Option<String>,
    ) -> Result<RazorpayCustomer, tonic::Status> {
        let request = CreateCustomerRequest {
            name: name.to_string(),
            email: email.to_string(),
            phone,
        };

        let request = self.add_tenant_context(Request::new(request), app_id, org_id, user_id);
        let response = self.client.create_customer(request).await?;

        response
            .into_inner()
            .customer
            .ok_or_else(|| tonic::Status::internal("Missing customer in response"))
    }

    /// Get a customer by ID.
    pub async fn get_customer(
        &mut self,
        app_id: &str,
        org_id: &str,
        user_id: Option<&str>,
        customer_id: &str,
    ) -> Result<RazorpayCustomer, tonic::Status> {
        let request = GetCustomerRequest {
            customer_id: customer_id.to_string(),
        };

        let request = self.add_tenant_context(Request::new(request), app_id, org_id, user_id);
        let response = self.client.get_customer(request).await?;

        response
            .into_inner()
            .customer
            .ok_or_else(|| tonic::Status::internal("Missing customer in response"))
    }

    /// Update a customer.
    #[allow(clippy::too_many_arguments)]
    pub async fn update_customer(
        &mut self,
        app_id: &str,
        org_id: &str,
        user_id: Option<&str>,
        customer_id: &str,
        name: Option<String>,
        email: Option<String>,
        phone: Option<String>,
    ) -> Result<RazorpayCustomer, tonic::Status> {
        let request = UpdateCustomerRequest {
            customer_id: customer_id.to_string(),
            name,
            email,
            phone,
        };

        let request = self.add_tenant_context(Request::new(request), app_id, org_id, user_id);
        let response = self.client.update_customer(request).await?;

        response
            .into_inner()
            .customer
            .ok_or_else(|| tonic::Status::internal("Missing customer in response"))
    }

    /// List customers.
    pub async fn list_customers(
        &mut self,
        app_id: &str,
        org_id: &str,
        user_id: Option<&str>,
        limit: i32,
        offset: i32,
    ) -> Result<(Vec<RazorpayCustomer>, i64), tonic::Status> {
        let request = ListCustomersRequest { limit, offset };

        let request = self.add_tenant_context(Request::new(request), app_id, org_id, user_id);
        let response = self.client.list_customers(request).await?;
        let inner = response.into_inner();

        Ok((inner.customers, inner.total_count))
    }

    // =========================================================================
    // Transfer Operations
    // =========================================================================

    /// Create a transfer from a payment to a linked account.
    #[allow(clippy::too_many_arguments)]
    pub async fn create_transfer_from_payment(
        &mut self,
        app_id: &str,
        org_id: &str,
        user_id: Option<&str>,
        payment_id: &str,
        linked_account_id: &str,
        amount: u64,
        currency: &str,
        on_hold: bool,
        on_hold_until: Option<prost_types::Timestamp>,
    ) -> Result<Transfer, tonic::Status> {
        let request = CreateTransferFromPaymentRequest {
            payment_id: payment_id.to_string(),
            linked_account_id: linked_account_id.to_string(),
            amount,
            currency: currency.to_string(),
            on_hold,
            on_hold_until,
        };

        let request = self.add_tenant_context(Request::new(request), app_id, org_id, user_id);
        let response = self.client.create_transfer_from_payment(request).await?;

        response
            .into_inner()
            .transfer
            .ok_or_else(|| tonic::Status::internal("Missing transfer in response"))
    }

    /// Create a transfer with an order.
    #[allow(clippy::too_many_arguments)]
    pub async fn create_transfer_from_order(
        &mut self,
        app_id: &str,
        org_id: &str,
        user_id: Option<&str>,
        order_id: &str,
        linked_account_id: &str,
        amount: u64,
        currency: &str,
        on_hold: bool,
    ) -> Result<Transfer, tonic::Status> {
        let request = CreateTransferFromOrderRequest {
            order_id: order_id.to_string(),
            linked_account_id: linked_account_id.to_string(),
            amount,
            currency: currency.to_string(),
            on_hold,
        };

        let request = self.add_tenant_context(Request::new(request), app_id, org_id, user_id);
        let response = self.client.create_transfer_from_order(request).await?;

        response
            .into_inner()
            .transfer
            .ok_or_else(|| tonic::Status::internal("Missing transfer in response"))
    }

    /// Create a direct transfer to a linked account.
    pub async fn create_direct_transfer(
        &mut self,
        app_id: &str,
        org_id: &str,
        user_id: Option<&str>,
        linked_account_id: &str,
        amount: u64,
        currency: &str,
    ) -> Result<Transfer, tonic::Status> {
        let request = CreateDirectTransferRequest {
            linked_account_id: linked_account_id.to_string(),
            amount,
            currency: currency.to_string(),
        };

        let request = self.add_tenant_context(Request::new(request), app_id, org_id, user_id);
        let response = self.client.create_direct_transfer(request).await?;

        response
            .into_inner()
            .transfer
            .ok_or_else(|| tonic::Status::internal("Missing transfer in response"))
    }

    /// Reverse a transfer.
    pub async fn reverse_transfer(
        &mut self,
        app_id: &str,
        org_id: &str,
        user_id: Option<&str>,
        transfer_id: &str,
        amount: u64,
    ) -> Result<Transfer, tonic::Status> {
        let request = ReverseTransferRequest {
            transfer_id: transfer_id.to_string(),
            amount,
        };

        let request = self.add_tenant_context(Request::new(request), app_id, org_id, user_id);
        let response = self.client.reverse_transfer(request).await?;

        response
            .into_inner()
            .transfer
            .ok_or_else(|| tonic::Status::internal("Missing transfer in response"))
    }

    /// Get a transfer by ID.
    pub async fn get_transfer(
        &mut self,
        app_id: &str,
        org_id: &str,
        user_id: Option<&str>,
        transfer_id: &str,
    ) -> Result<Transfer, tonic::Status> {
        let request = GetTransferRequest {
            transfer_id: transfer_id.to_string(),
        };

        let request = self.add_tenant_context(Request::new(request), app_id, org_id, user_id);
        let response = self.client.get_transfer(request).await?;

        response
            .into_inner()
            .transfer
            .ok_or_else(|| tonic::Status::internal("Missing transfer in response"))
    }

    /// List transfers with optional filters.
    #[allow(clippy::too_many_arguments)]
    pub async fn list_transfers(
        &mut self,
        app_id: &str,
        org_id: &str,
        user_id: Option<&str>,
        linked_account_id: Option<String>,
        payment_id: Option<String>,
        status: Option<TransferStatus>,
        limit: i32,
        offset: i32,
    ) -> Result<(Vec<Transfer>, i64), tonic::Status> {
        let request = ListTransfersRequest {
            linked_account_id,
            payment_id,
            status: status.map(|s| s.into()),
            limit,
            offset,
        };

        let request = self.add_tenant_context(Request::new(request), app_id, org_id, user_id);
        let response = self.client.list_transfers(request).await?;
        let inner = response.into_inner();

        Ok((inner.transfers, inner.total_count))
    }

    /// Hold settlement for a transfer.
    pub async fn hold_transfer_settlement(
        &mut self,
        app_id: &str,
        org_id: &str,
        user_id: Option<&str>,
        transfer_id: &str,
        on_hold_until: Option<prost_types::Timestamp>,
    ) -> Result<Transfer, tonic::Status> {
        let request = HoldTransferSettlementRequest {
            transfer_id: transfer_id.to_string(),
            on_hold_until,
        };

        let request = self.add_tenant_context(Request::new(request), app_id, org_id, user_id);
        let response = self.client.hold_transfer_settlement(request).await?;

        response
            .into_inner()
            .transfer
            .ok_or_else(|| tonic::Status::internal("Missing transfer in response"))
    }

    /// Release settlement for a transfer.
    pub async fn release_transfer_settlement(
        &mut self,
        app_id: &str,
        org_id: &str,
        user_id: Option<&str>,
        transfer_id: &str,
    ) -> Result<Transfer, tonic::Status> {
        let request = ReleaseTransferSettlementRequest {
            transfer_id: transfer_id.to_string(),
        };

        let request = self.add_tenant_context(Request::new(request), app_id, org_id, user_id);
        let response = self.client.release_transfer_settlement(request).await?;

        response
            .into_inner()
            .transfer
            .ok_or_else(|| tonic::Status::internal("Missing transfer in response"))
    }

    // =========================================================================
    // Settlement Operations
    // =========================================================================

    /// Request an on-demand settlement.
    pub async fn request_on_demand_settlement(
        &mut self,
        app_id: &str,
        org_id: &str,
        user_id: Option<&str>,
        linked_account_id: Option<String>,
        amount: u64,
        currency: &str,
    ) -> Result<Settlement, tonic::Status> {
        let request = RequestOnDemandSettlementRequest {
            linked_account_id,
            amount,
            currency: currency.to_string(),
        };

        let request = self.add_tenant_context(Request::new(request), app_id, org_id, user_id);
        let response = self.client.request_on_demand_settlement(request).await?;

        response
            .into_inner()
            .settlement
            .ok_or_else(|| tonic::Status::internal("Missing settlement in response"))
    }

    /// Get a settlement by ID.
    pub async fn get_settlement(
        &mut self,
        app_id: &str,
        org_id: &str,
        user_id: Option<&str>,
        settlement_id: &str,
    ) -> Result<Settlement, tonic::Status> {
        let request = GetSettlementRequest {
            settlement_id: settlement_id.to_string(),
        };

        let request = self.add_tenant_context(Request::new(request), app_id, org_id, user_id);
        let response = self.client.get_settlement(request).await?;

        response
            .into_inner()
            .settlement
            .ok_or_else(|| tonic::Status::internal("Missing settlement in response"))
    }

    /// List settlements with optional filters.
    #[allow(clippy::too_many_arguments)]
    pub async fn list_settlements(
        &mut self,
        app_id: &str,
        org_id: &str,
        user_id: Option<&str>,
        linked_account_id: Option<String>,
        status: Option<SettlementStatus>,
        settlement_type: Option<SettlementType>,
        limit: i32,
        offset: i32,
    ) -> Result<(Vec<Settlement>, i64), tonic::Status> {
        let request = ListSettlementsRequest {
            linked_account_id,
            status: status.map(|s| s.into()),
            settlement_type: settlement_type.map(|s| s.into()),
            limit,
            offset,
        };

        let request = self.add_tenant_context(Request::new(request), app_id, org_id, user_id);
        let response = self.client.list_settlements(request).await?;
        let inner = response.into_inner();

        Ok((inner.settlements, inner.total_count))
    }

    // =========================================================================
    // Subscription Operations
    // =========================================================================

    /// Create a subscription plan.
    #[allow(clippy::too_many_arguments)]
    pub async fn create_razorpay_plan(
        &mut self,
        app_id: &str,
        org_id: &str,
        user_id: Option<&str>,
        name: &str,
        description: &str,
        amount: u64,
        currency: &str,
        period: PlanPeriod,
        interval: i32,
    ) -> Result<RazorpayPlan, tonic::Status> {
        let request = CreateRazorpayPlanRequest {
            name: name.to_string(),
            description: description.to_string(),
            amount,
            currency: currency.to_string(),
            period: period.into(),
            interval,
        };

        let request = self.add_tenant_context(Request::new(request), app_id, org_id, user_id);
        let response = self.client.create_razorpay_plan(request).await?;

        response
            .into_inner()
            .plan
            .ok_or_else(|| tonic::Status::internal("Missing plan in response"))
    }

    /// Get a subscription plan by ID.
    pub async fn get_razorpay_plan(
        &mut self,
        app_id: &str,
        org_id: &str,
        user_id: Option<&str>,
        plan_id: &str,
    ) -> Result<RazorpayPlan, tonic::Status> {
        let request = GetRazorpayPlanRequest {
            plan_id: plan_id.to_string(),
        };

        let request = self.add_tenant_context(Request::new(request), app_id, org_id, user_id);
        let response = self.client.get_razorpay_plan(request).await?;

        response
            .into_inner()
            .plan
            .ok_or_else(|| tonic::Status::internal("Missing plan in response"))
    }

    /// List subscription plans.
    pub async fn list_razorpay_plans(
        &mut self,
        app_id: &str,
        org_id: &str,
        user_id: Option<&str>,
        limit: i32,
        offset: i32,
    ) -> Result<(Vec<RazorpayPlan>, i64), tonic::Status> {
        let request = ListRazorpayPlansRequest { limit, offset };

        let request = self.add_tenant_context(Request::new(request), app_id, org_id, user_id);
        let response = self.client.list_razorpay_plans(request).await?;
        let inner = response.into_inner();

        Ok((inner.plans, inner.total_count))
    }

    /// Create a subscription.
    #[allow(clippy::too_many_arguments)]
    pub async fn create_razorpay_subscription(
        &mut self,
        app_id: &str,
        org_id: &str,
        user_id: Option<&str>,
        plan_id: &str,
        customer_id: Option<String>,
        total_count: i32,
        notes_json: Option<String>,
    ) -> Result<RazorpaySubscription, tonic::Status> {
        let request = CreateRazorpaySubscriptionRequest {
            plan_id: plan_id.to_string(),
            customer_id,
            total_count,
            notes_json,
        };

        let request = self.add_tenant_context(Request::new(request), app_id, org_id, user_id);
        let response = self.client.create_razorpay_subscription(request).await?;

        response
            .into_inner()
            .subscription
            .ok_or_else(|| tonic::Status::internal("Missing subscription in response"))
    }

    /// Get a subscription by ID.
    pub async fn get_razorpay_subscription(
        &mut self,
        app_id: &str,
        org_id: &str,
        user_id: Option<&str>,
        subscription_id: &str,
    ) -> Result<RazorpaySubscription, tonic::Status> {
        let request = GetRazorpaySubscriptionRequest {
            subscription_id: subscription_id.to_string(),
        };

        let request = self.add_tenant_context(Request::new(request), app_id, org_id, user_id);
        let response = self.client.get_razorpay_subscription(request).await?;

        response
            .into_inner()
            .subscription
            .ok_or_else(|| tonic::Status::internal("Missing subscription in response"))
    }

    /// List subscriptions with optional filters.
    #[allow(clippy::too_many_arguments)]
    pub async fn list_razorpay_subscriptions(
        &mut self,
        app_id: &str,
        org_id: &str,
        user_id: Option<&str>,
        customer_id: Option<String>,
        plan_id: Option<String>,
        status: Option<SubscriptionStatus>,
        limit: i32,
        offset: i32,
    ) -> Result<(Vec<RazorpaySubscription>, i64), tonic::Status> {
        let request = ListRazorpaySubscriptionsRequest {
            customer_id,
            plan_id,
            status: status.map(|s| s.into()),
            limit,
            offset,
        };

        let request = self.add_tenant_context(Request::new(request), app_id, org_id, user_id);
        let response = self.client.list_razorpay_subscriptions(request).await?;
        let inner = response.into_inner();

        Ok((inner.subscriptions, inner.total_count))
    }

    /// Pause a subscription.
    pub async fn pause_razorpay_subscription(
        &mut self,
        app_id: &str,
        org_id: &str,
        user_id: Option<&str>,
        subscription_id: &str,
    ) -> Result<RazorpaySubscription, tonic::Status> {
        let request = PauseRazorpaySubscriptionRequest {
            subscription_id: subscription_id.to_string(),
        };

        let request = self.add_tenant_context(Request::new(request), app_id, org_id, user_id);
        let response = self.client.pause_razorpay_subscription(request).await?;

        response
            .into_inner()
            .subscription
            .ok_or_else(|| tonic::Status::internal("Missing subscription in response"))
    }

    /// Resume a paused subscription.
    pub async fn resume_razorpay_subscription(
        &mut self,
        app_id: &str,
        org_id: &str,
        user_id: Option<&str>,
        subscription_id: &str,
    ) -> Result<RazorpaySubscription, tonic::Status> {
        let request = ResumeRazorpaySubscriptionRequest {
            subscription_id: subscription_id.to_string(),
        };

        let request = self.add_tenant_context(Request::new(request), app_id, org_id, user_id);
        let response = self.client.resume_razorpay_subscription(request).await?;

        response
            .into_inner()
            .subscription
            .ok_or_else(|| tonic::Status::internal("Missing subscription in response"))
    }

    /// Cancel a subscription.
    pub async fn cancel_razorpay_subscription(
        &mut self,
        app_id: &str,
        org_id: &str,
        user_id: Option<&str>,
        subscription_id: &str,
        cancel_at_cycle_end: bool,
    ) -> Result<RazorpaySubscription, tonic::Status> {
        let request = CancelRazorpaySubscriptionRequest {
            subscription_id: subscription_id.to_string(),
            cancel_at_cycle_end,
        };

        let request = self.add_tenant_context(Request::new(request), app_id, org_id, user_id);
        let response = self.client.cancel_razorpay_subscription(request).await?;

        response
            .into_inner()
            .subscription
            .ok_or_else(|| tonic::Status::internal("Missing subscription in response"))
    }

    /// Update a subscription.
    pub async fn update_razorpay_subscription(
        &mut self,
        app_id: &str,
        org_id: &str,
        user_id: Option<&str>,
        subscription_id: &str,
        plan_id: Option<String>,
        total_count: Option<i32>,
    ) -> Result<RazorpaySubscription, tonic::Status> {
        let request = UpdateRazorpaySubscriptionRequest {
            subscription_id: subscription_id.to_string(),
            plan_id,
            total_count,
        };

        let request = self.add_tenant_context(Request::new(request), app_id, org_id, user_id);
        let response = self.client.update_razorpay_subscription(request).await?;

        response
            .into_inner()
            .subscription
            .ok_or_else(|| tonic::Status::internal("Missing subscription in response"))
    }

    // =========================================================================
    // Payment Link Operations
    // =========================================================================

    /// Create a payment link.
    #[allow(clippy::too_many_arguments)]
    pub async fn create_payment_link(
        &mut self,
        app_id: &str,
        org_id: &str,
        user_id: Option<&str>,
        amount: u64,
        currency: &str,
        description: &str,
        customer_name: Option<String>,
        customer_email: Option<String>,
        customer_phone: Option<String>,
        expire_by_seconds: Option<i64>,
    ) -> Result<PaymentLink, tonic::Status> {
        let request = CreatePaymentLinkRequest {
            amount,
            currency: currency.to_string(),
            description: description.to_string(),
            customer_name,
            customer_email,
            customer_phone,
            expire_by_seconds,
        };

        let request = self.add_tenant_context(Request::new(request), app_id, org_id, user_id);
        let response = self.client.create_payment_link(request).await?;

        response
            .into_inner()
            .payment_link
            .ok_or_else(|| tonic::Status::internal("Missing payment_link in response"))
    }

    /// Get a payment link by ID.
    pub async fn get_payment_link(
        &mut self,
        app_id: &str,
        org_id: &str,
        user_id: Option<&str>,
        payment_link_id: &str,
    ) -> Result<PaymentLink, tonic::Status> {
        let request = GetPaymentLinkRequest {
            payment_link_id: payment_link_id.to_string(),
        };

        let request = self.add_tenant_context(Request::new(request), app_id, org_id, user_id);
        let response = self.client.get_payment_link(request).await?;

        response
            .into_inner()
            .payment_link
            .ok_or_else(|| tonic::Status::internal("Missing payment_link in response"))
    }

    /// Cancel a payment link.
    pub async fn cancel_payment_link(
        &mut self,
        app_id: &str,
        org_id: &str,
        user_id: Option<&str>,
        payment_link_id: &str,
    ) -> Result<PaymentLink, tonic::Status> {
        let request = CancelPaymentLinkRequest {
            payment_link_id: payment_link_id.to_string(),
        };

        let request = self.add_tenant_context(Request::new(request), app_id, org_id, user_id);
        let response = self.client.cancel_payment_link(request).await?;

        response
            .into_inner()
            .payment_link
            .ok_or_else(|| tonic::Status::internal("Missing payment_link in response"))
    }

    /// List payment links with optional status filter.
    pub async fn list_payment_links(
        &mut self,
        app_id: &str,
        org_id: &str,
        user_id: Option<&str>,
        status: Option<PaymentLinkStatus>,
        limit: i32,
        offset: i32,
    ) -> Result<(Vec<PaymentLink>, i64), tonic::Status> {
        let request = ListPaymentLinksRequest {
            status: status.map(|s| s.into()),
            limit,
            offset,
        };

        let request = self.add_tenant_context(Request::new(request), app_id, org_id, user_id);
        let response = self.client.list_payment_links(request).await?;
        let inner = response.into_inner();

        Ok((inner.payment_links, inner.total_count))
    }

    // =========================================================================
    // Refund Operations
    // =========================================================================

    /// Initiate a refund for a payment.
    #[allow(clippy::too_many_arguments)]
    pub async fn initiate_refund(
        &mut self,
        app_id: &str,
        org_id: &str,
        user_id: Option<&str>,
        payment_id: &str,
        amount: u64,
        speed: RefundSpeed,
        reason: Option<String>,
        reverse_all_transfers: bool,
    ) -> Result<Refund, tonic::Status> {
        let request = InitiateRefundRequest {
            payment_id: payment_id.to_string(),
            amount,
            speed: speed.into(),
            reason,
            reverse_all_transfers,
        };

        let request = self.add_tenant_context(Request::new(request), app_id, org_id, user_id);
        let response = self.client.initiate_refund(request).await?;

        response
            .into_inner()
            .refund
            .ok_or_else(|| tonic::Status::internal("Missing refund in response"))
    }

    /// Get a refund by ID.
    pub async fn get_refund(
        &mut self,
        app_id: &str,
        org_id: &str,
        user_id: Option<&str>,
        refund_id: &str,
    ) -> Result<Refund, tonic::Status> {
        let request = GetRefundRequest {
            refund_id: refund_id.to_string(),
        };

        let request = self.add_tenant_context(Request::new(request), app_id, org_id, user_id);
        let response = self.client.get_refund(request).await?;

        response
            .into_inner()
            .refund
            .ok_or_else(|| tonic::Status::internal("Missing refund in response"))
    }

    /// List refunds with optional filters.
    #[allow(clippy::too_many_arguments)]
    pub async fn list_refunds(
        &mut self,
        app_id: &str,
        org_id: &str,
        user_id: Option<&str>,
        payment_id: Option<String>,
        status: Option<RefundStatus>,
        limit: i32,
        offset: i32,
    ) -> Result<(Vec<Refund>, i64), tonic::Status> {
        let request = ListRefundsRequest {
            payment_id,
            status: status.map(|s| s.into()),
            limit,
            offset,
        };

        let request = self.add_tenant_context(Request::new(request), app_id, org_id, user_id);
        let response = self.client.list_refunds(request).await?;
        let inner = response.into_inner();

        Ok((inner.refunds, inner.total_count))
    }
}
