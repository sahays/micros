//! gRPC implementation of AuthzService.

use service_core::grpc::IntoStatus;
use tonic::{Request, Response, Status};
use uuid::Uuid;

use crate::grpc::proto::auth::{
    authz_service_server::AuthzService, AssignmentSummary as ProtoAssignmentSummary,
    CheckCapabilityRequest, CheckCapabilityResponse, GetAuthContextRequest, GetAuthContextResponse,
};
use crate::handlers::context as context_handler;
use crate::AppState;

/// gRPC AuthzService implementation.
pub struct AuthzServiceImpl {
    state: AppState,
}

impl AuthzServiceImpl {
    /// Create a new AuthzServiceImpl.
    pub fn new(state: AppState) -> Self {
        Self { state }
    }

    /// Extract user_id and tenant_id from request metadata.
    /// For gRPC internal services, these are passed via metadata headers.
    #[allow(clippy::result_large_err)]
    fn extract_user_context(
        &self,
        request: &Request<impl std::fmt::Debug>,
    ) -> Result<(Uuid, Uuid), Status> {
        // Try to get from metadata first (for internal gRPC calls)
        let metadata = request.metadata();

        // Check for x-user-id header
        let user_id = if let Some(user_id_value) = metadata.get("x-user-id") {
            let user_id_str = user_id_value
                .to_str()
                .map_err(|_| Status::invalid_argument("Invalid x-user-id header"))?;
            Uuid::parse_str(user_id_str)
                .map_err(|_| Status::invalid_argument("Invalid user_id format"))?
        } else if let Some(auth_value) = metadata.get("authorization") {
            // Fallback: validate bearer token
            let auth_str = auth_value
                .to_str()
                .map_err(|_| Status::unauthenticated("Invalid authorization header"))?;
            let token = auth_str
                .strip_prefix("Bearer ")
                .ok_or_else(|| Status::unauthenticated("Invalid authorization header format"))?;

            let claims = self
                .state
                .jwt
                .validate_access_token(token)
                .map_err(|e| Status::unauthenticated(format!("Invalid token: {}", e)))?;

            Uuid::parse_str(&claims.sub)
                .map_err(|_| Status::internal("Invalid user_id in token"))?
        } else {
            return Err(Status::unauthenticated(
                "Missing x-user-id or authorization header",
            ));
        };

        // Check for x-tenant-id header
        let tenant_id = if let Some(tenant_id_value) = metadata.get("x-tenant-id") {
            let tenant_id_str = tenant_id_value
                .to_str()
                .map_err(|_| Status::invalid_argument("Invalid x-tenant-id header"))?;
            Uuid::parse_str(tenant_id_str)
                .map_err(|_| Status::invalid_argument("Invalid tenant_id format"))?
        } else if let Some(auth_value) = metadata.get("authorization") {
            // Fallback: get from token
            let auth_str = auth_value
                .to_str()
                .map_err(|_| Status::unauthenticated("Invalid authorization header"))?;
            let token = auth_str
                .strip_prefix("Bearer ")
                .ok_or_else(|| Status::unauthenticated("Invalid authorization header format"))?;

            let claims = self
                .state
                .jwt
                .validate_access_token(token)
                .map_err(|e| Status::unauthenticated(format!("Invalid token: {}", e)))?;

            Uuid::parse_str(&claims.app_id)
                .map_err(|_| Status::internal("Invalid tenant_id in token"))?
        } else {
            return Err(Status::unauthenticated(
                "Missing x-tenant-id or authorization header",
            ));
        };

        Ok((user_id, tenant_id))
    }
}

#[tonic::async_trait]
impl AuthzService for AuthzServiceImpl {
    async fn get_auth_context(
        &self,
        request: Request<GetAuthContextRequest>,
    ) -> Result<Response<GetAuthContextResponse>, Status> {
        let (user_id, tenant_id) = self.extract_user_context(&request)?;
        let req = request.into_inner();

        // Parse optional org_node_id
        let org_node_id = req
            .org_node_id
            .as_ref()
            .map(|s| {
                Uuid::parse_str(s).map_err(|_| Status::invalid_argument("Invalid org_node_id"))
            })
            .transpose()?;

        // Call handler implementation
        let result =
            context_handler::get_auth_context_impl(&self.state, user_id, tenant_id, org_node_id)
                .await
                .map_err(|e| e.into_status())?;

        // Convert to proto response
        let assignments: Vec<ProtoAssignmentSummary> = result
            .assignments
            .into_iter()
            .map(|a| ProtoAssignmentSummary {
                assignment_id: a.assignment_id.to_string(),
                org_node_id: a.org_node_id.to_string(),
                role_id: a.role_id.to_string(),
                role_label: a.role_label,
                capabilities: a.capabilities,
            })
            .collect();

        Ok(Response::new(GetAuthContextResponse {
            user_id: result.user_id.to_string(),
            tenant_id: result.tenant_id.to_string(),
            org_node_id: result.org_node_id.map(|id| id.to_string()),
            capabilities: result.capabilities,
            assignments,
        }))
    }

    async fn check_capability(
        &self,
        request: Request<CheckCapabilityRequest>,
    ) -> Result<Response<CheckCapabilityResponse>, Status> {
        let (user_id, _tenant_id) = self.extract_user_context(&request)?;
        let req = request.into_inner();

        // Parse org_node_id
        let org_node_id = Uuid::parse_str(&req.org_node_id)
            .map_err(|_| Status::invalid_argument("Invalid org_node_id"))?;

        // Call handler implementation
        let result = context_handler::check_capability_impl(
            &self.state,
            user_id,
            org_node_id,
            req.capability,
        )
        .await
        .map_err(|e| e.into_status())?;

        Ok(Response::new(CheckCapabilityResponse {
            allowed: result.allowed,
            capability: result.capability,
            org_node_id: result.org_node_id.to_string(),
            matched_assignment_id: result.matched_assignment.map(|id| id.to_string()),
        }))
    }
}
