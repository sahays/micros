//! Plan, subscription, payment link, refund, and direct/offline payment operations.

use crate::grpc::proto::payment::{
    CancelPaymentLinkRequest, CancelRazorpaySubscriptionRequest, CreatePaymentLinkRequest,
    CreateRazorpayPlanRequest, CreateRazorpaySubscriptionRequest, GetPaymentLinkRequest,
    GetRazorpayPlanRequest, GetRazorpaySubscriptionRequest, GetRefundRequest,
    InitiateRefundRequest, ListPaymentLinksRequest, ListRazorpayPlansRequest,
    ListRazorpaySubscriptionsRequest, ListRefundsRequest, PauseRazorpaySubscriptionRequest,
    PaymentLink, PaymentLinkStatus, PaymentMethodType, PlanPeriod, RazorpayPlan,
    RazorpaySubscription, RecordDirectUpiPaymentRequest, RecordOfflinePaymentRequest, Refund,
    RefundSpeed, RefundStatus, ResumeRazorpaySubscriptionRequest, SubscriptionStatus, Transaction,
    UpdateRazorpaySubscriptionRequest,
};
use tonic::Request;

use super::PaymentClient;

impl PaymentClient {
    /// Create a subscription plan.
    #[allow(clippy::too_many_arguments)]
    pub async fn create_razorpay_plan(
        &mut self,
        app_id: &str,
        tenant_id: &str,
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

        let request = self.add_tenant_context(Request::new(request), app_id, tenant_id, user_id);
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
        tenant_id: &str,
        user_id: Option<&str>,
        plan_id: &str,
    ) -> Result<RazorpayPlan, tonic::Status> {
        let request = GetRazorpayPlanRequest {
            plan_id: plan_id.to_string(),
        };

        let request = self.add_tenant_context(Request::new(request), app_id, tenant_id, user_id);
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
        tenant_id: &str,
        user_id: Option<&str>,
        limit: i32,
        offset: i32,
    ) -> Result<(Vec<RazorpayPlan>, i64), tonic::Status> {
        let request = ListRazorpayPlansRequest { limit, offset };

        let request = self.add_tenant_context(Request::new(request), app_id, tenant_id, user_id);
        let response = self.client.list_razorpay_plans(request).await?;
        let inner = response.into_inner();

        Ok((inner.plans, inner.total_count))
    }

    /// Create a subscription.
    #[allow(clippy::too_many_arguments)]
    pub async fn create_razorpay_subscription(
        &mut self,
        app_id: &str,
        tenant_id: &str,
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

        let request = self.add_tenant_context(Request::new(request), app_id, tenant_id, user_id);
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
        tenant_id: &str,
        user_id: Option<&str>,
        subscription_id: &str,
    ) -> Result<RazorpaySubscription, tonic::Status> {
        let request = GetRazorpaySubscriptionRequest {
            subscription_id: subscription_id.to_string(),
        };

        let request = self.add_tenant_context(Request::new(request), app_id, tenant_id, user_id);
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
        tenant_id: &str,
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

        let request = self.add_tenant_context(Request::new(request), app_id, tenant_id, user_id);
        let response = self.client.list_razorpay_subscriptions(request).await?;
        let inner = response.into_inner();

        Ok((inner.subscriptions, inner.total_count))
    }

    /// Pause a subscription.
    pub async fn pause_razorpay_subscription(
        &mut self,
        app_id: &str,
        tenant_id: &str,
        user_id: Option<&str>,
        subscription_id: &str,
    ) -> Result<RazorpaySubscription, tonic::Status> {
        let request = PauseRazorpaySubscriptionRequest {
            subscription_id: subscription_id.to_string(),
        };

        let request = self.add_tenant_context(Request::new(request), app_id, tenant_id, user_id);
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
        tenant_id: &str,
        user_id: Option<&str>,
        subscription_id: &str,
    ) -> Result<RazorpaySubscription, tonic::Status> {
        let request = ResumeRazorpaySubscriptionRequest {
            subscription_id: subscription_id.to_string(),
        };

        let request = self.add_tenant_context(Request::new(request), app_id, tenant_id, user_id);
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
        tenant_id: &str,
        user_id: Option<&str>,
        subscription_id: &str,
        cancel_at_cycle_end: bool,
    ) -> Result<RazorpaySubscription, tonic::Status> {
        let request = CancelRazorpaySubscriptionRequest {
            subscription_id: subscription_id.to_string(),
            cancel_at_cycle_end,
        };

        let request = self.add_tenant_context(Request::new(request), app_id, tenant_id, user_id);
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
        tenant_id: &str,
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

        let request = self.add_tenant_context(Request::new(request), app_id, tenant_id, user_id);
        let response = self.client.update_razorpay_subscription(request).await?;

        response
            .into_inner()
            .subscription
            .ok_or_else(|| tonic::Status::internal("Missing subscription in response"))
    }

    /// Create a payment link.
    #[allow(clippy::too_many_arguments)]
    pub async fn create_payment_link(
        &mut self,
        app_id: &str,
        tenant_id: &str,
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

        let request = self.add_tenant_context(Request::new(request), app_id, tenant_id, user_id);
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
        tenant_id: &str,
        user_id: Option<&str>,
        payment_link_id: &str,
    ) -> Result<PaymentLink, tonic::Status> {
        let request = GetPaymentLinkRequest {
            payment_link_id: payment_link_id.to_string(),
        };

        let request = self.add_tenant_context(Request::new(request), app_id, tenant_id, user_id);
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
        tenant_id: &str,
        user_id: Option<&str>,
        payment_link_id: &str,
    ) -> Result<PaymentLink, tonic::Status> {
        let request = CancelPaymentLinkRequest {
            payment_link_id: payment_link_id.to_string(),
        };

        let request = self.add_tenant_context(Request::new(request), app_id, tenant_id, user_id);
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
        tenant_id: &str,
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

        let request = self.add_tenant_context(Request::new(request), app_id, tenant_id, user_id);
        let response = self.client.list_payment_links(request).await?;
        let inner = response.into_inner();

        Ok((inner.payment_links, inner.total_count))
    }

    /// Initiate a refund for a payment.
    #[allow(clippy::too_many_arguments)]
    pub async fn initiate_refund(
        &mut self,
        app_id: &str,
        tenant_id: &str,
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

        let request = self.add_tenant_context(Request::new(request), app_id, tenant_id, user_id);
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
        tenant_id: &str,
        user_id: Option<&str>,
        refund_id: &str,
    ) -> Result<Refund, tonic::Status> {
        let request = GetRefundRequest {
            refund_id: refund_id.to_string(),
        };

        let request = self.add_tenant_context(Request::new(request), app_id, tenant_id, user_id);
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
        tenant_id: &str,
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

        let request = self.add_tenant_context(Request::new(request), app_id, tenant_id, user_id);
        let response = self.client.list_refunds(request).await?;
        let inner = response.into_inner();

        Ok((inner.refunds, inner.total_count))
    }

    /// Record a payment received directly via UPI.
    #[allow(clippy::too_many_arguments)]
    pub async fn record_direct_upi_payment(
        &mut self,
        app_id: &str,
        tenant_id: &str,
        user_id: Option<&str>,
        amount_paise: u64,
        currency: &str,
        utr: &str,
        payer_vpa: Option<String>,
        notes: Option<String>,
    ) -> Result<Transaction, tonic::Status> {
        let request = RecordDirectUpiPaymentRequest {
            amount_paise,
            currency: currency.to_string(),
            utr: utr.to_string(),
            payer_vpa,
            notes,
        };

        let request = self.add_tenant_context(Request::new(request), app_id, tenant_id, user_id);
        let response = self.client.record_direct_upi_payment(request).await?;

        response
            .into_inner()
            .transaction
            .ok_or_else(|| tonic::Status::internal("Missing transaction in response"))
    }

    /// Record an offline payment (cash, cheque, bank transfer, etc.).
    #[allow(clippy::too_many_arguments)]
    pub async fn record_offline_payment(
        &mut self,
        app_id: &str,
        tenant_id: &str,
        user_id: Option<&str>,
        amount_paise: u64,
        currency: &str,
        payment_method_type: PaymentMethodType,
        external_reference: Option<String>,
        notes: Option<String>,
    ) -> Result<Transaction, tonic::Status> {
        let request = RecordOfflinePaymentRequest {
            amount_paise,
            currency: currency.to_string(),
            payment_method_type: payment_method_type.into(),
            external_reference,
            notes,
        };

        let request = self.add_tenant_context(Request::new(request), app_id, tenant_id, user_id);
        let response = self.client.record_offline_payment(request).await?;

        response
            .into_inner()
            .transaction
            .ok_or_else(|| tonic::Status::internal("Missing transaction in response"))
    }
}
