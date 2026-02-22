//! gRPC implementation of AuthzService.

use crate::grpc::capability_check::extract_auth_context;
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
}

#[tonic::async_trait]
impl AuthzService for AuthzServiceImpl {
    async fn get_auth_context(
        &self,
        request: Request<GetAuthContextRequest>,
    ) -> Result<Response<GetAuthContextResponse>, Status> {
        let auth = extract_auth_context(&request)?;
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
            context_handler::get_auth_context_impl(&self.state, auth.user_id, auth.app_id, org_node_id)
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
        let auth = extract_auth_context(&request)?;
        let req = request.into_inner();

        // Parse org_node_id
        let org_node_id = Uuid::parse_str(&req.org_node_id)
            .map_err(|_| Status::invalid_argument("Invalid org_node_id"))?;

        // Call handler implementation
        let result = context_handler::check_capability_impl(
            &self.state,
            auth.user_id,
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
