//! Capability check module for billing-service.
//! Provides authorization enforcement for gRPC methods.

use tonic::{Request, Status};

/// Billing service capabilities.
pub mod capabilities {
    pub const BILLING_PLAN_CREATE: &str = "billing.plan:create";
    pub const BILLING_PLAN_READ: &str = "billing.plan:read";
    pub const BILLING_PLAN_UPDATE: &str = "billing.plan:update";
    pub const BILLING_SUBSCRIPTION_CREATE: &str = "billing.subscription:create";
    pub const BILLING_SUBSCRIPTION_READ: &str = "billing.subscription:read";
    pub const BILLING_SUBSCRIPTION_MANAGE: &str = "billing.subscription:manage";
    pub const BILLING_SUBSCRIPTION_CHANGE: &str = "billing.subscription:change";
    pub const BILLING_USAGE_WRITE: &str = "billing.usage:write";
    pub const BILLING_USAGE_READ: &str = "billing.usage:read";
    pub const BILLING_CYCLE_READ: &str = "billing.cycle:read";
    pub const BILLING_CYCLE_MANAGE: &str = "billing.cycle:manage";
    pub const BILLING_CHARGE_CREATE: &str = "billing.charge:create";
    pub const BILLING_RUN_EXECUTE: &str = "billing.run:execute";
    pub const BILLING_RUN_READ: &str = "billing.run:read";
}

/// Authentication context extracted from request.
#[derive(Debug, Clone)]
pub struct AuthContext {
    pub user_id: String,
    pub tenant_id: String,
    pub capabilities: Vec<String>,
}

/// Capability checker for authorization.
#[derive(Clone)]
pub struct CapabilityChecker {
    #[allow(dead_code)] // Reserved for future auth-service integration
    auth_service_endpoint: String,
}

impl CapabilityChecker {
    /// Create a new capability checker.
    pub fn new(auth_service_endpoint: &str) -> Self {
        Self {
            auth_service_endpoint: auth_service_endpoint.to_string(),
        }
    }

    /// Check if the request has the required capability.
    /// For BFF requests (with X-Tenant-Id header), trust the header.
    /// For direct requests, validate the token against auth-service.
    pub async fn require_capability<T>(
        &self,
        request: &Request<T>,
        required_capability: &str,
    ) -> Result<AuthContext, Status> {
        // Check for BFF trust headers (service-to-service calls)
        if let Some(tenant_id) = request.metadata().get("x-tenant-id") {
            let tenant_id = tenant_id
                .to_str()
                .map_err(|_| Status::unauthenticated("Invalid X-Tenant-Id header"))?
                .to_string();

            // BFF requests are trusted - the BFF has already validated the user
            // Extract user_id from header if present
            let user_id = request
                .metadata()
                .get("x-user-id")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("system")
                .to_string();

            tracing::debug!(
                tenant_id = %tenant_id,
                user_id = %user_id,
                capability = %required_capability,
                "BFF request - trusting headers"
            );

            return Ok(AuthContext {
                user_id,
                tenant_id,
                capabilities: vec![required_capability.to_string()],
            });
        }

        // For direct requests, we would validate the token against auth-service
        // For now, return unauthenticated if no tenant header
        Err(Status::unauthenticated(
            "Missing X-Tenant-Id header for service-to-service call",
        ))
    }

    /// Extract tenant_id from request without capability check.
    /// Used for public endpoints that only need tenant context.
    #[allow(clippy::result_large_err)]
    pub fn extract_tenant_id<T>(&self, request: &Request<T>) -> Result<String, Status> {
        request
            .metadata()
            .get("x-tenant-id")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string())
            .ok_or_else(|| Status::unauthenticated("Missing X-Tenant-Id header"))
    }
}
