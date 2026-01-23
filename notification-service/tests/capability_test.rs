//! Capability enforcement tests for notification-service.
//!
//! Tests capability-based access control for notification service gRPC endpoints.

use tonic::Request;

// ============================================================================
// Capability Checker Unit Tests
// ============================================================================

mod capability_checker_tests {
    use service_core::grpc::{extract_bearer_token, extract_org_node_id, CapabilityChecker};
    use tonic::Request;

    #[tokio::test]
    async fn disabled_checker_allows_all_requests() {
        let checker = CapabilityChecker::disabled();
        assert!(!checker.is_enabled());

        let request: Request<()> = Request::new(());
        let result = checker
            .require_capability(&request, "notification.email:send")
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
            .require_capability(&request, "notification:read")
            .await;
        assert!(result.is_ok());

        let auth_context = result.unwrap();
        assert_eq!(auth_context.user_id, "user-123");
        assert_eq!(auth_context.tenant_id, "tenant-456");
    }

    #[test]
    fn extract_bearer_token_missing_header() {
        let request: Request<()> = Request::new(());
        let result = extract_bearer_token(&request);
        assert!(result.is_err());
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
    fn extract_org_node_id_success() {
        let mut request: Request<()> = Request::new(());
        request
            .metadata_mut()
            .insert("x-org-id", "org-789".parse().unwrap());

        let result = extract_org_node_id(&request);
        assert_eq!(result, Some("org-789".to_string()));
    }
}

// ============================================================================
// Capability Constants Tests
// ============================================================================

mod capability_constants_tests {
    use notification_service::grpc::capability_check::capabilities;

    #[test]
    fn notification_capabilities_are_defined() {
        assert_eq!(
            capabilities::NOTIFICATION_EMAIL_SEND,
            "notification.email:send"
        );
        assert_eq!(capabilities::NOTIFICATION_SMS_SEND, "notification.sms:send");
        assert_eq!(
            capabilities::NOTIFICATION_PUSH_SEND,
            "notification.push:send"
        );
        assert_eq!(
            capabilities::NOTIFICATION_BATCH_SEND,
            "notification.batch:send"
        );
        assert_eq!(capabilities::NOTIFICATION_READ, "notification:read");
    }
}
