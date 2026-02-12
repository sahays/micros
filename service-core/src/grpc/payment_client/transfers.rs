//! Transfer and settlement operations.

use crate::grpc::proto::payment::{
    CreateDirectTransferRequest, CreateTransferFromOrderRequest, CreateTransferFromPaymentRequest,
    GetSettlementRequest, GetTransferRequest, HoldTransferSettlementRequest,
    ListSettlementsRequest, ListTransfersRequest, ReleaseTransferSettlementRequest,
    RequestOnDemandSettlementRequest, ReverseTransferRequest, Settlement, SettlementStatus,
    SettlementType, Transfer, TransferStatus,
};
use tonic::Request;

use super::PaymentClient;

impl PaymentClient {
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
}
