//! Ledger service gRPC client for service-to-service communication.
//!
//! Provides a high-level client for calling ledger-service with built-in retry support.

use std::time::Duration;
use tonic::Request;
use tonic::transport::{Channel, Endpoint};

use super::proto::ledger::ledger_service_client::LedgerServiceClient;
use super::proto::ledger::{
    AccountType, CreateAccountRequest, CreateAccountResponse, Direction, GetAccountRequest,
    GetAccountResponse, GetBalanceRequest, GetBalanceResponse, GetBalancesRequest,
    GetBalancesResponse, GetStatementRequest, GetStatementResponse, GetTransactionRequest,
    GetTransactionResponse, ListAccountsRequest, ListAccountsResponse, ListTransactionsRequest,
    ListTransactionsResponse, PostTransactionEntry, PostTransactionRequest,
    PostTransactionResponse,
};
use super::retry::{RetryConfig, retry_grpc_call};

/// Configuration for the ledger service client.
#[derive(Clone, Debug)]
pub struct LedgerClientConfig {
    /// The gRPC endpoint of the ledger service.
    pub endpoint: String,
    /// Connection timeout.
    pub connect_timeout: Duration,
    /// Request timeout.
    pub request_timeout: Duration,
    /// Retry configuration.
    pub retry_config: RetryConfig,
}

impl Default for LedgerClientConfig {
    fn default() -> Self {
        Self {
            endpoint: "http://localhost:50052".to_string(),
            connect_timeout: Duration::from_secs(5),
            request_timeout: Duration::from_secs(30),
            retry_config: RetryConfig::default(),
        }
    }
}

/// Ledger service client with retry support.
#[derive(Clone)]
pub struct LedgerClient {
    client: LedgerServiceClient<Channel>,
    retry_config: RetryConfig,
}

impl LedgerClient {
    /// Create a new ledger client with the given configuration.
    pub async fn new(config: LedgerClientConfig) -> Result<Self, tonic::transport::Error> {
        let channel = Endpoint::from_shared(config.endpoint)?
            .connect_timeout(config.connect_timeout)
            .timeout(config.request_timeout)
            .connect()
            .await?;

        Ok(Self {
            client: LedgerServiceClient::new(channel),
            retry_config: config.retry_config,
        })
    }

    /// Create a new ledger client connecting to the specified endpoint.
    pub async fn connect(endpoint: &str) -> Result<Self, tonic::transport::Error> {
        Self::new(LedgerClientConfig {
            endpoint: endpoint.to_string(),
            ..Default::default()
        })
        .await
    }

    /// Create a new ledger client with custom retry configuration.
    pub async fn with_retry(
        endpoint: &str,
        retry_config: RetryConfig,
    ) -> Result<Self, tonic::transport::Error> {
        Self::new(LedgerClientConfig {
            endpoint: endpoint.to_string(),
            retry_config,
            ..Default::default()
        })
        .await
    }

    // =========================================================================
    // Account Management
    // =========================================================================

    /// Create a new ledger account.
    pub async fn create_account(
        &self,
        tenant_id: &str,
        account_type: AccountType,
        account_code: &str,
        currency: &str,
        allow_negative: bool,
        metadata: Option<&str>,
    ) -> Result<CreateAccountResponse, tonic::Status> {
        let client = self.client.clone();
        let request = CreateAccountRequest {
            tenant_id: tenant_id.to_string(),
            account_type: account_type.into(),
            account_code: account_code.to_string(),
            currency: currency.to_string(),
            allow_negative,
            metadata: metadata.unwrap_or("").to_string(),
        };

        retry_grpc_call(&self.retry_config, "create_account", || {
            let mut c = client.clone();
            let req = request.clone();
            async move {
                let response = c.create_account(Request::new(req)).await?;
                Ok(response.into_inner())
            }
        })
        .await
    }

    /// Get an account by ID.
    pub async fn get_account(
        &self,
        tenant_id: &str,
        account_id: &str,
    ) -> Result<GetAccountResponse, tonic::Status> {
        let client = self.client.clone();
        let request = GetAccountRequest {
            tenant_id: tenant_id.to_string(),
            account_id: account_id.to_string(),
        };

        retry_grpc_call(&self.retry_config, "get_account", || {
            let mut c = client.clone();
            let req = request.clone();
            async move {
                let response = c.get_account(Request::new(req)).await?;
                Ok(response.into_inner())
            }
        })
        .await
    }

    /// List accounts for a tenant.
    pub async fn list_accounts(
        &self,
        tenant_id: &str,
        account_type: Option<AccountType>,
        currency: Option<&str>,
        page_size: i32,
        page_token: Option<&str>,
    ) -> Result<ListAccountsResponse, tonic::Status> {
        let client = self.client.clone();
        let request = ListAccountsRequest {
            tenant_id: tenant_id.to_string(),
            account_type: account_type.map(|t| t.into()).unwrap_or(0),
            currency: currency.unwrap_or("").to_string(),
            page_size,
            page_token: page_token.unwrap_or("").to_string(),
        };

        retry_grpc_call(&self.retry_config, "list_accounts", || {
            let mut c = client.clone();
            let req = request.clone();
            async move {
                let response = c.list_accounts(Request::new(req)).await?;
                Ok(response.into_inner())
            }
        })
        .await
    }

    // =========================================================================
    // Transaction Operations
    // =========================================================================

    /// Post a transaction (journal entry) with multiple entries.
    ///
    /// Entries must balance (total debits = total credits).
    pub async fn post_transaction(
        &self,
        tenant_id: &str,
        entries: Vec<TransactionEntry>,
        effective_date: Option<&str>,
        idempotency_key: &str,
        metadata: Option<&str>,
    ) -> Result<PostTransactionResponse, tonic::Status> {
        let client = self.client.clone();
        let request = PostTransactionRequest {
            tenant_id: tenant_id.to_string(),
            entries: entries.into_iter().map(|e| e.into()).collect(),
            effective_date: effective_date.unwrap_or("").to_string(),
            idempotency_key: idempotency_key.to_string(),
            metadata: metadata.unwrap_or("").to_string(),
        };

        retry_grpc_call(&self.retry_config, "post_transaction", || {
            let mut c = client.clone();
            let req = request.clone();
            async move {
                let response = c.post_transaction(Request::new(req)).await?;
                Ok(response.into_inner())
            }
        })
        .await
    }

    /// Get a transaction by journal ID.
    pub async fn get_transaction(
        &self,
        tenant_id: &str,
        journal_id: &str,
    ) -> Result<GetTransactionResponse, tonic::Status> {
        let client = self.client.clone();
        let request = GetTransactionRequest {
            tenant_id: tenant_id.to_string(),
            journal_id: journal_id.to_string(),
        };

        retry_grpc_call(&self.retry_config, "get_transaction", || {
            let mut c = client.clone();
            let req = request.clone();
            async move {
                let response = c.get_transaction(Request::new(req)).await?;
                Ok(response.into_inner())
            }
        })
        .await
    }

    /// List transactions for a tenant.
    pub async fn list_transactions(
        &self,
        tenant_id: &str,
        account_id: Option<&str>,
        start_date: Option<&str>,
        end_date: Option<&str>,
        page_size: i32,
        page_token: Option<&str>,
    ) -> Result<ListTransactionsResponse, tonic::Status> {
        let client = self.client.clone();
        let request = ListTransactionsRequest {
            tenant_id: tenant_id.to_string(),
            account_id: account_id.unwrap_or("").to_string(),
            start_date: start_date.unwrap_or("").to_string(),
            end_date: end_date.unwrap_or("").to_string(),
            page_size,
            page_token: page_token.unwrap_or("").to_string(),
        };

        retry_grpc_call(&self.retry_config, "list_transactions", || {
            let mut c = client.clone();
            let req = request.clone();
            async move {
                let response = c.list_transactions(Request::new(req)).await?;
                Ok(response.into_inner())
            }
        })
        .await
    }

    // =========================================================================
    // Balance Queries
    // =========================================================================

    /// Get balance for a single account.
    pub async fn get_balance(
        &self,
        tenant_id: &str,
        account_id: &str,
        as_of_date: Option<&str>,
    ) -> Result<GetBalanceResponse, tonic::Status> {
        let client = self.client.clone();
        let request = GetBalanceRequest {
            tenant_id: tenant_id.to_string(),
            account_id: account_id.to_string(),
            as_of_date: as_of_date.unwrap_or("").to_string(),
        };

        retry_grpc_call(&self.retry_config, "get_balance", || {
            let mut c = client.clone();
            let req = request.clone();
            async move {
                let response = c.get_balance(Request::new(req)).await?;
                Ok(response.into_inner())
            }
        })
        .await
    }

    /// Get balances for multiple accounts.
    pub async fn get_balances(
        &self,
        tenant_id: &str,
        account_ids: Vec<String>,
        as_of_date: Option<&str>,
    ) -> Result<GetBalancesResponse, tonic::Status> {
        let client = self.client.clone();
        let request = GetBalancesRequest {
            tenant_id: tenant_id.to_string(),
            account_ids,
            as_of_date: as_of_date.unwrap_or("").to_string(),
        };

        retry_grpc_call(&self.retry_config, "get_balances", || {
            let mut c = client.clone();
            let req = request.clone();
            async move {
                let response = c.get_balances(Request::new(req)).await?;
                Ok(response.into_inner())
            }
        })
        .await
    }

    // =========================================================================
    // Statements
    // =========================================================================

    /// Get a statement for an account.
    pub async fn get_statement(
        &self,
        tenant_id: &str,
        account_id: &str,
        start_date: &str,
        end_date: &str,
    ) -> Result<GetStatementResponse, tonic::Status> {
        let client = self.client.clone();
        let request = GetStatementRequest {
            tenant_id: tenant_id.to_string(),
            account_id: account_id.to_string(),
            start_date: start_date.to_string(),
            end_date: end_date.to_string(),
        };

        retry_grpc_call(&self.retry_config, "get_statement", || {
            let mut c = client.clone();
            let req = request.clone();
            async move {
                let response = c.get_statement(Request::new(req)).await?;
                Ok(response.into_inner())
            }
        })
        .await
    }
}

/// Helper struct for building transaction entries.
#[derive(Clone, Debug)]
pub struct TransactionEntry {
    pub account_id: String,
    pub amount: String,
    pub direction: Direction,
}

impl TransactionEntry {
    /// Create a debit entry.
    pub fn debit(account_id: &str, amount: &str) -> Self {
        Self {
            account_id: account_id.to_string(),
            amount: amount.to_string(),
            direction: Direction::Debit,
        }
    }

    /// Create a credit entry.
    pub fn credit(account_id: &str, amount: &str) -> Self {
        Self {
            account_id: account_id.to_string(),
            amount: amount.to_string(),
            direction: Direction::Credit,
        }
    }
}

impl From<TransactionEntry> for PostTransactionEntry {
    fn from(entry: TransactionEntry) -> Self {
        Self {
            account_id: entry.account_id,
            amount: entry.amount,
            direction: entry.direction.into(),
        }
    }
}

// Re-export useful types from proto
pub use super::proto::ledger::{Account as AccountProto, AccountType as AccountTypeProto};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ledger_client_config_default() {
        let config = LedgerClientConfig::default();
        assert_eq!(config.endpoint, "http://localhost:50052");
        assert_eq!(config.connect_timeout, Duration::from_secs(5));
        assert_eq!(config.request_timeout, Duration::from_secs(30));
    }

    #[test]
    fn test_transaction_entry_debit() {
        let entry = TransactionEntry::debit("acc123", "100.00");
        assert_eq!(entry.account_id, "acc123");
        assert_eq!(entry.amount, "100.00");
        assert!(matches!(entry.direction, Direction::Debit));
    }

    #[test]
    fn test_transaction_entry_credit() {
        let entry = TransactionEntry::credit("acc456", "50.00");
        assert_eq!(entry.account_id, "acc456");
        assert_eq!(entry.amount, "50.00");
        assert!(matches!(entry.direction, Direction::Credit));
    }
}
