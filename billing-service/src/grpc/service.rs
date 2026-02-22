//! BillingService gRPC implementation.

use crate::grpc::proto::billing_service_server::BillingService;
use crate::grpc::proto::*;
use crate::services::Database;
use std::sync::Arc;
use tonic::{Request, Response, Status};

use crate::grpc::{billing_cycles, billing_runs, charges, plans, subscriptions, usage};

/// BillingService implementation.
pub struct BillingServiceImpl {
    db: Arc<Database>,
}

impl BillingServiceImpl {
    /// Create a new BillingServiceImpl.
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }
}

#[tonic::async_trait]
impl BillingService for BillingServiceImpl {
    // =========================================================================
    // Plan Management
    // =========================================================================

    #[tracing::instrument(skip(self, request), fields(method = "CreatePlan"))]
    async fn create_plan(
        &self,
        request: Request<CreatePlanRequest>,
    ) -> Result<Response<CreatePlanResponse>, Status> {
        plans::create_plan(&self.db, request).await
    }

    #[tracing::instrument(skip(self, request), fields(method = "GetPlan"))]
    async fn get_plan(
        &self,
        request: Request<GetPlanRequest>,
    ) -> Result<Response<GetPlanResponse>, Status> {
        plans::get_plan(&self.db, request).await
    }

    #[tracing::instrument(skip(self, request), fields(method = "UpdatePlan"))]
    async fn update_plan(
        &self,
        request: Request<UpdatePlanRequest>,
    ) -> Result<Response<UpdatePlanResponse>, Status> {
        plans::update_plan(&self.db, request).await
    }

    #[tracing::instrument(skip(self, request), fields(method = "ListPlans"))]
    async fn list_plans(
        &self,
        request: Request<ListPlansRequest>,
    ) -> Result<Response<ListPlansResponse>, Status> {
        plans::list_plans(&self.db, request).await
    }

    #[tracing::instrument(skip(self, request), fields(method = "ArchivePlan"))]
    async fn archive_plan(
        &self,
        request: Request<ArchivePlanRequest>,
    ) -> Result<Response<ArchivePlanResponse>, Status> {
        plans::archive_plan(&self.db, request).await
    }

    // =========================================================================
    // Subscription Management
    // =========================================================================

    #[tracing::instrument(skip(self, request), fields(method = "CreateSubscription"))]
    async fn create_subscription(
        &self,
        request: Request<CreateSubscriptionRequest>,
    ) -> Result<Response<CreateSubscriptionResponse>, Status> {
        subscriptions::create_subscription(&self.db, request).await
    }

    #[tracing::instrument(skip(self, request), fields(method = "GetSubscription"))]
    async fn get_subscription(
        &self,
        request: Request<GetSubscriptionRequest>,
    ) -> Result<Response<GetSubscriptionResponse>, Status> {
        subscriptions::get_subscription(&self.db, request).await
    }

    #[tracing::instrument(skip(self, request), fields(method = "ListSubscriptions"))]
    async fn list_subscriptions(
        &self,
        request: Request<ListSubscriptionsRequest>,
    ) -> Result<Response<ListSubscriptionsResponse>, Status> {
        subscriptions::list_subscriptions(&self.db, request).await
    }

    // =========================================================================
    // Subscription Lifecycle
    // =========================================================================

    #[tracing::instrument(skip(self, request), fields(method = "ActivateSubscription"))]
    async fn activate_subscription(
        &self,
        request: Request<ActivateSubscriptionRequest>,
    ) -> Result<Response<ActivateSubscriptionResponse>, Status> {
        subscriptions::activate_subscription(&self.db, request).await
    }

    #[tracing::instrument(skip(self, request), fields(method = "PauseSubscription"))]
    async fn pause_subscription(
        &self,
        request: Request<PauseSubscriptionRequest>,
    ) -> Result<Response<PauseSubscriptionResponse>, Status> {
        subscriptions::pause_subscription(&self.db, request).await
    }

    #[tracing::instrument(skip(self, request), fields(method = "ResumeSubscription"))]
    async fn resume_subscription(
        &self,
        request: Request<ResumeSubscriptionRequest>,
    ) -> Result<Response<ResumeSubscriptionResponse>, Status> {
        subscriptions::resume_subscription(&self.db, request).await
    }

    #[tracing::instrument(skip(self, request), fields(method = "CancelSubscription"))]
    async fn cancel_subscription(
        &self,
        request: Request<CancelSubscriptionRequest>,
    ) -> Result<Response<CancelSubscriptionResponse>, Status> {
        subscriptions::cancel_subscription(&self.db, request).await
    }

    // =========================================================================
    // Plan Changes
    // =========================================================================

    #[tracing::instrument(skip(self, request), fields(method = "ChangePlan"))]
    async fn change_plan(
        &self,
        request: Request<ChangePlanRequest>,
    ) -> Result<Response<ChangePlanResponse>, Status> {
        subscriptions::change_plan(&self.db, request).await
    }

    // =========================================================================
    // Usage Tracking
    // =========================================================================

    #[tracing::instrument(skip(self, request), fields(method = "RecordUsage"))]
    async fn record_usage(
        &self,
        request: Request<RecordUsageRequest>,
    ) -> Result<Response<RecordUsageResponse>, Status> {
        usage::record_usage(&self.db, request).await
    }

    #[tracing::instrument(skip(self, request), fields(method = "GetUsage"))]
    async fn get_usage(
        &self,
        request: Request<GetUsageRequest>,
    ) -> Result<Response<GetUsageResponse>, Status> {
        usage::get_usage(&self.db, request).await
    }

    #[tracing::instrument(skip(self, request), fields(method = "ListUsage"))]
    async fn list_usage(
        &self,
        request: Request<ListUsageRequest>,
    ) -> Result<Response<ListUsageResponse>, Status> {
        usage::list_usage(&self.db, request).await
    }

    #[tracing::instrument(skip(self, request), fields(method = "GetUsageSummary"))]
    async fn get_usage_summary(
        &self,
        request: Request<GetUsageSummaryRequest>,
    ) -> Result<Response<GetUsageSummaryResponse>, Status> {
        usage::get_usage_summary(&self.db, request).await
    }

    // =========================================================================
    // Billing Cycles
    // =========================================================================

    #[tracing::instrument(skip(self, request), fields(method = "GetBillingCycle"))]
    async fn get_billing_cycle(
        &self,
        request: Request<GetBillingCycleRequest>,
    ) -> Result<Response<GetBillingCycleResponse>, Status> {
        billing_cycles::get_billing_cycle(&self.db, request).await
    }

    #[tracing::instrument(skip(self, request), fields(method = "ListBillingCycles"))]
    async fn list_billing_cycles(
        &self,
        request: Request<ListBillingCyclesRequest>,
    ) -> Result<Response<ListBillingCyclesResponse>, Status> {
        billing_cycles::list_billing_cycles(&self.db, request).await
    }

    #[tracing::instrument(skip(self, request), fields(method = "AdvanceBillingCycle"))]
    async fn advance_billing_cycle(
        &self,
        request: Request<AdvanceBillingCycleRequest>,
    ) -> Result<Response<AdvanceBillingCycleResponse>, Status> {
        billing_cycles::advance_billing_cycle(&self.db, request).await
    }

    // =========================================================================
    // Charges
    // =========================================================================

    #[tracing::instrument(skip(self, request), fields(method = "GetCharge"))]
    async fn get_charge(
        &self,
        request: Request<GetChargeRequest>,
    ) -> Result<Response<GetChargeResponse>, Status> {
        charges::get_charge(&self.db, request).await
    }

    #[tracing::instrument(skip(self, request), fields(method = "ListCharges"))]
    async fn list_charges(
        &self,
        request: Request<ListChargesRequest>,
    ) -> Result<Response<ListChargesResponse>, Status> {
        charges::list_charges(&self.db, request).await
    }

    #[tracing::instrument(skip(self, request), fields(method = "CreateOneTimeCharge"))]
    async fn create_one_time_charge(
        &self,
        request: Request<CreateOneTimeChargeRequest>,
    ) -> Result<Response<CreateOneTimeChargeResponse>, Status> {
        charges::create_one_time_charge(&self.db, request).await
    }

    // =========================================================================
    // Billing Runs
    // =========================================================================

    #[tracing::instrument(skip(self, request), fields(method = "RunBilling"))]
    async fn run_billing(
        &self,
        request: Request<RunBillingRequest>,
    ) -> Result<Response<RunBillingResponse>, Status> {
        billing_runs::run_billing(&self.db, request).await
    }

    #[tracing::instrument(skip(self, request), fields(method = "RunBillingForSubscription"))]
    async fn run_billing_for_subscription(
        &self,
        request: Request<RunBillingForSubscriptionRequest>,
    ) -> Result<Response<RunBillingForSubscriptionResponse>, Status> {
        billing_runs::run_billing_for_subscription(&self.db, request).await
    }

    #[tracing::instrument(skip(self, request), fields(method = "GetBillingRun"))]
    async fn get_billing_run(
        &self,
        request: Request<GetBillingRunRequest>,
    ) -> Result<Response<GetBillingRunResponse>, Status> {
        billing_runs::get_billing_run(&self.db, request).await
    }

    #[tracing::instrument(skip(self, request), fields(method = "ListBillingRuns"))]
    async fn list_billing_runs(
        &self,
        request: Request<ListBillingRunsRequest>,
    ) -> Result<Response<ListBillingRunsResponse>, Status> {
        billing_runs::list_billing_runs(&self.db, request).await
    }
}
