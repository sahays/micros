//! Capability enforcement tests for reconciliation-service.
//!
//! Tests capability-based access control for reconciliation service gRPC endpoints.
//!
//! ## Test Categories
//!
//! 1. **Unit Tests**: Test capability checker behavior in isolation
//! 2. **Capability Constants**: Verify capability strings are correctly defined
//! 3. **Integration Tests**: Test actual endpoint authorization (requires auth-service)
//!
//! ## Running Integration Tests
//!
//! Integration tests require auth-service to be running. Set the environment variable:
//! ```bash
//! AUTH_SERVICE_ENDPOINT=http://localhost:9005 cargo test --test capability_test
//! ```

// ============================================================================
// Capability Checker Unit Tests
// ============================================================================

mod capability_checker_tests {
    use reconciliation_service::grpc::capability_check::{
        extract_bearer_token, extract_org_node_id, CapabilityChecker, CapabilityMetadata,
    };
    use tonic::Request;

    #[tokio::test]
    async fn disabled_checker_allows_all_requests() {
        let checker = CapabilityChecker::disabled();
        assert!(!checker.is_enabled());

        let request: Request<()> = Request::new(());
        let result = checker
            .require_capability(&request, "reconciliation.bank_account:create")
            .await;
        assert!(result.is_ok(), "Disabled checker should allow all requests");
    }

    #[tokio::test]
    async fn disabled_checker_returns_auth_context_from_headers() {
        let checker = CapabilityChecker::disabled();

        let mut request: Request<()> = Request::new(());
        request
            .metadata_mut()
            .insert("x-user-id", "user-123".parse().unwrap());
        request
            .metadata_mut()
            .insert("x-tenant-id", "tenant-456".parse().unwrap());

        let result = checker
            .require_capability(&request, "reconciliation.process:read")
            .await;
        assert!(result.is_ok());

        let auth_context = result.unwrap();
        assert_eq!(auth_context.user_id, "user-123");
        assert_eq!(auth_context.tenant_id, "tenant-456");
    }

    #[tokio::test]
    async fn disabled_checker_returns_default_when_headers_missing() {
        let checker = CapabilityChecker::disabled();

        let request: Request<()> = Request::new(());
        let result = checker
            .require_capability(&request, "reconciliation.process:read")
            .await;
        assert!(result.is_ok());

        let auth_context = result.unwrap();
        assert_eq!(auth_context.user_id, "system");
        assert_eq!(auth_context.tenant_id, "");
    }

    #[tokio::test]
    async fn disabled_checker_require_auth_works() {
        let checker = CapabilityChecker::disabled();

        let mut request: Request<()> = Request::new(());
        request
            .metadata_mut()
            .insert("x-user-id", "test-user".parse().unwrap());
        request
            .metadata_mut()
            .insert("x-tenant-id", "test-tenant".parse().unwrap());

        let result = checker.require_auth(&request).await;
        assert!(result.is_ok());

        let auth = result.unwrap();
        assert_eq!(auth.user_id, "test-user");
        assert_eq!(auth.tenant_id, "test-tenant");
    }

    #[test]
    fn extract_bearer_token_missing_header() {
        let request: Request<()> = Request::new(());
        let result = extract_bearer_token(&request);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .message()
            .contains("Missing authorization header"));
    }

    #[test]
    fn extract_bearer_token_invalid_format() {
        let mut request: Request<()> = Request::new(());
        request
            .metadata_mut()
            .insert("authorization", "Basic abc123".parse().unwrap());

        let result = extract_bearer_token(&request);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .message()
            .contains("Invalid Bearer token format"));
    }

    #[test]
    fn extract_bearer_token_success() {
        let mut request: Request<()> = Request::new(());
        request
            .metadata_mut()
            .insert("authorization", "Bearer test-token-123".parse().unwrap());

        let result = extract_bearer_token(&request);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "test-token-123");
    }

    #[test]
    fn extract_org_node_id_missing() {
        let request: Request<()> = Request::new(());
        let result = extract_org_node_id(&request);
        assert!(result.is_none());
    }

    #[test]
    fn extract_org_node_id_success() {
        let mut request: Request<()> = Request::new(());
        request
            .metadata_mut()
            .insert("x-org-id", "org-789".parse().unwrap());

        let result = extract_org_node_id(&request);
        assert_eq!(result, Some("org-789".to_string()));
    }

    #[test]
    fn capability_metadata_from_request_success() {
        let mut request: Request<()> = Request::new(());
        request
            .metadata_mut()
            .insert("authorization", "Bearer test-token".parse().unwrap());
        request
            .metadata_mut()
            .insert("x-org-id", "org-123".parse().unwrap());

        let metadata = CapabilityMetadata::from_request(&request);
        assert!(metadata.is_ok());

        let metadata = metadata.unwrap();
        assert_eq!(metadata.token, "test-token");
        assert_eq!(metadata.org_node_id, Some("org-123".to_string()));
    }

    #[test]
    fn capability_metadata_from_request_missing_auth() {
        let request: Request<()> = Request::new(());
        let metadata = CapabilityMetadata::from_request(&request);
        assert!(metadata.is_err());
    }

    #[test]
    fn capability_metadata_try_from_request_returns_none_without_auth() {
        let request: Request<()> = Request::new(());
        let metadata = CapabilityMetadata::try_from_request(&request);
        assert!(metadata.is_none());
    }

    #[test]
    fn capability_metadata_try_from_request_returns_some_with_auth() {
        let mut request: Request<()> = Request::new(());
        request
            .metadata_mut()
            .insert("authorization", "Bearer test-token".parse().unwrap());

        let metadata = CapabilityMetadata::try_from_request(&request);
        assert!(metadata.is_some());
        assert_eq!(metadata.unwrap().token, "test-token");
    }
}

// ============================================================================
// Capability Constants Tests
// ============================================================================

mod capability_constants_tests {
    use reconciliation_service::grpc::capability_check::capabilities;

    #[test]
    fn bank_account_capabilities_are_defined() {
        assert_eq!(
            capabilities::RECONCILIATION_BANK_ACCOUNT_CREATE,
            "reconciliation.bank_account:create"
        );
        assert_eq!(
            capabilities::RECONCILIATION_BANK_ACCOUNT_READ,
            "reconciliation.bank_account:read"
        );
        assert_eq!(
            capabilities::RECONCILIATION_BANK_ACCOUNT_UPDATE,
            "reconciliation.bank_account:update"
        );
    }

    #[test]
    fn statement_capabilities_are_defined() {
        assert_eq!(
            capabilities::RECONCILIATION_STATEMENT_IMPORT,
            "reconciliation.statement:import"
        );
        assert_eq!(
            capabilities::RECONCILIATION_STATEMENT_READ,
            "reconciliation.statement:read"
        );
        assert_eq!(
            capabilities::RECONCILIATION_STAGED_UPDATE,
            "reconciliation.staged:update"
        );
        assert_eq!(
            capabilities::RECONCILIATION_STATEMENT_COMMIT,
            "reconciliation.statement:commit"
        );
        assert_eq!(
            capabilities::RECONCILIATION_STATEMENT_ABANDON,
            "reconciliation.statement:abandon"
        );
    }

    #[test]
    fn rule_capabilities_are_defined() {
        assert_eq!(
            capabilities::RECONCILIATION_RULE_CREATE,
            "reconciliation.rule:create"
        );
        assert_eq!(
            capabilities::RECONCILIATION_RULE_READ,
            "reconciliation.rule:read"
        );
        assert_eq!(
            capabilities::RECONCILIATION_RULE_UPDATE,
            "reconciliation.rule:update"
        );
        assert_eq!(
            capabilities::RECONCILIATION_RULE_DELETE,
            "reconciliation.rule:delete"
        );
    }

    #[test]
    fn transaction_capabilities_are_defined() {
        assert_eq!(
            capabilities::RECONCILIATION_TRANSACTION_READ,
            "reconciliation.transaction:read"
        );
        assert_eq!(
            capabilities::RECONCILIATION_MATCH_CREATE,
            "reconciliation.match:create"
        );
        assert_eq!(
            capabilities::RECONCILIATION_MATCH_DELETE,
            "reconciliation.match:delete"
        );
        assert_eq!(
            capabilities::RECONCILIATION_EXCLUDE,
            "reconciliation.transaction:exclude"
        );
    }

    #[test]
    fn ai_capabilities_are_defined() {
        assert_eq!(
            capabilities::RECONCILIATION_AI_SUGGEST,
            "reconciliation.ai:suggest"
        );
        assert_eq!(
            capabilities::RECONCILIATION_AI_CONFIRM,
            "reconciliation.ai:confirm"
        );
    }

    #[test]
    fn process_capabilities_are_defined() {
        assert_eq!(
            capabilities::RECONCILIATION_START,
            "reconciliation.process:start"
        );
        assert_eq!(
            capabilities::RECONCILIATION_READ,
            "reconciliation.process:read"
        );
        assert_eq!(
            capabilities::RECONCILIATION_COMPLETE,
            "reconciliation.process:complete"
        );
        assert_eq!(
            capabilities::RECONCILIATION_ABANDON,
            "reconciliation.process:abandon"
        );
    }

    #[test]
    fn adjustment_capabilities_are_defined() {
        assert_eq!(
            capabilities::RECONCILIATION_ADJUSTMENT_CREATE,
            "reconciliation.adjustment:create"
        );
        assert_eq!(
            capabilities::RECONCILIATION_ADJUSTMENT_READ,
            "reconciliation.adjustment:read"
        );
    }

    #[test]
    fn all_capabilities_follow_naming_convention() {
        // All capabilities should follow pattern: reconciliation.<resource>:<action>
        let all_capabilities = [
            capabilities::RECONCILIATION_BANK_ACCOUNT_CREATE,
            capabilities::RECONCILIATION_BANK_ACCOUNT_READ,
            capabilities::RECONCILIATION_BANK_ACCOUNT_UPDATE,
            capabilities::RECONCILIATION_STATEMENT_IMPORT,
            capabilities::RECONCILIATION_STATEMENT_READ,
            capabilities::RECONCILIATION_STAGED_UPDATE,
            capabilities::RECONCILIATION_STATEMENT_COMMIT,
            capabilities::RECONCILIATION_STATEMENT_ABANDON,
            capabilities::RECONCILIATION_RULE_CREATE,
            capabilities::RECONCILIATION_RULE_READ,
            capabilities::RECONCILIATION_RULE_UPDATE,
            capabilities::RECONCILIATION_RULE_DELETE,
            capabilities::RECONCILIATION_TRANSACTION_READ,
            capabilities::RECONCILIATION_MATCH_CREATE,
            capabilities::RECONCILIATION_MATCH_DELETE,
            capabilities::RECONCILIATION_EXCLUDE,
            capabilities::RECONCILIATION_AI_SUGGEST,
            capabilities::RECONCILIATION_AI_CONFIRM,
            capabilities::RECONCILIATION_START,
            capabilities::RECONCILIATION_READ,
            capabilities::RECONCILIATION_COMPLETE,
            capabilities::RECONCILIATION_ABANDON,
            capabilities::RECONCILIATION_ADJUSTMENT_CREATE,
            capabilities::RECONCILIATION_ADJUSTMENT_READ,
        ];

        for cap in &all_capabilities {
            assert!(
                cap.starts_with("reconciliation."),
                "Capability '{}' should start with 'reconciliation.'",
                cap
            );
            assert!(
                cap.contains(':'),
                "Capability '{}' should contain ':' separator",
                cap
            );
        }

        // Verify count matches expected
        assert_eq!(
            all_capabilities.len(),
            24,
            "Expected 24 capabilities defined"
        );
    }
}

// ============================================================================
// Tenant Isolation Tests
// ============================================================================

mod tenant_isolation_tests {
    //! Tests that verify tenant data isolation.
    //!
    //! These tests run with capability checking disabled (BFF trust model)
    //! but verify that tenant_id filtering works correctly in database queries.

    // Note: Tenant isolation is tested in the integration tests:
    // - bank_account_test.rs::tenant_isolation_for_bank_accounts
    // - reconciliation_test.rs::tenant_isolation_for_reconciliations
    // - adjustment_test.rs::tenant_isolation_for_adjustments
    //
    // These tests verify:
    // 1. Tenant A cannot see Tenant B's data
    // 2. Tenant A cannot modify Tenant B's data
    // 3. List operations only return data for the requesting tenant
}

