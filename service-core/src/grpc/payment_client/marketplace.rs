//! Linked account and customer operations.

use crate::grpc::proto::payment::{
    BankAccount, CommissionConfig, CreateCustomerRequest, CreateLinkedAccountRequest,
    GetCustomerRequest, GetLinkedAccountRequest, LegalInfo, LinkedAccount, LinkedAccountStatus,
    ListCustomersRequest, ListLinkedAccountsRequest, RazorpayCustomer,
    UpdateCommissionConfigRequest, UpdateCustomerRequest, UpdateLinkedAccountRequest,
};
use tonic::Request;

use super::PaymentClient;

impl PaymentClient {
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
}
