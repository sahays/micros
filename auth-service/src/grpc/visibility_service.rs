//! gRPC implementation of VisibilityService.

use crate::grpc::capability_check::require_capability;
use crate::grpc::proto::auth::{
    visibility_service_server::VisibilityService, CreateVisibilityGrantRequest,
    CreateVisibilityGrantResponse, ListUserVisibilityGrantsRequest,
    ListUserVisibilityGrantsResponse, RevokeVisibilityGrantRequest, RevokeVisibilityGrantResponse,
    VisibilityGrant,
};
use crate::models::{AccessScope, VisibilityGrant as ModelVisibilityGrant};
use crate::AppState;
use prost_types::Timestamp;
use service_core::grpc::IntoStatus;
use tonic::{Request, Response, Status};
use uuid::Uuid;

/// gRPC VisibilityService implementation.
pub struct VisibilityServiceImpl {
    state: AppState,
}

impl VisibilityServiceImpl {
    /// Create a new VisibilityServiceImpl.
    pub fn new(state: AppState) -> Self {
        Self { state }
    }
}

/// Convert chrono DateTime to protobuf Timestamp.
fn datetime_to_timestamp(dt: chrono::DateTime<chrono::Utc>) -> Timestamp {
    Timestamp {
        seconds: dt.timestamp(),
        nanos: dt.timestamp_subsec_nanos() as i32,
    }
}

#[tonic::async_trait]
impl VisibilityService for VisibilityServiceImpl {
    async fn create_visibility_grant(
        &self,
        request: Request<CreateVisibilityGrantRequest>,
    ) -> Result<Response<CreateVisibilityGrantResponse>, Status> {
        // Require visibility:grant capability
        let _auth = require_capability(&self.state, &request, "visibility:grant").await?;

        let req = request.into_inner();

        let tenant_id = Uuid::parse_str(&req.tenant_id)
            .map_err(|_| Status::invalid_argument("Invalid tenant_id"))?;
        let user_id = Uuid::parse_str(&req.user_id)
            .map_err(|_| Status::invalid_argument("Invalid user_id"))?;
        let org_node_id = Uuid::parse_str(&req.org_node_id)
            .map_err(|_| Status::invalid_argument("Invalid org_node_id"))?;

        let start_utc = req
            .start_utc
            .map(|ts| {
                chrono::DateTime::from_timestamp(ts.seconds, ts.nanos as u32)
                    .ok_or_else(|| Status::invalid_argument("Invalid start_utc timestamp"))
            })
            .transpose()?;

        let end_utc = req
            .end_utc
            .map(|ts| {
                chrono::DateTime::from_timestamp(ts.seconds, ts.nanos as u32)
                    .ok_or_else(|| Status::invalid_argument("Invalid end_utc timestamp"))
            })
            .transpose()?;

        // Verify user exists
        self.state
            .db
            .find_user_by_id(user_id)
            .await
            .map_err(|e| e.into_status())?
            .ok_or_else(|| Status::not_found("User not found"))?;

        // Verify org node exists
        self.state
            .db
            .find_org_node_by_id(org_node_id)
            .await
            .map_err(|e| e.into_status())?
            .ok_or_else(|| Status::not_found("Org node not found"))?;

        // Parse access scope (default to Read)
        let access_scope = req
            .access_scope
            .as_deref()
            .map(AccessScope::parse)
            .unwrap_or(AccessScope::Read);

        // Create visibility grant
        let grant = ModelVisibilityGrant::new(
            tenant_id,
            user_id,
            org_node_id,
            access_scope,
            start_utc,
            end_utc,
        );

        self.state
            .db
            .insert_visibility_grant(&grant)
            .await
            .map_err(|e| e.into_status())?;

        Ok(Response::new(CreateVisibilityGrantResponse {
            grant: Some(VisibilityGrant {
                grant_id: grant.grant_id.to_string(),
                tenant_id: grant.tenant_id.to_string(),
                user_id: grant.user_id.to_string(),
                org_node_id: grant.org_node_id.to_string(),
                access_scope: grant.access_scope_code.clone(),
                start_utc: Some(datetime_to_timestamp(grant.start_utc)),
                end_utc: grant.end_utc.map(datetime_to_timestamp),
                // Note: granter_user_id not tracked in current model
                granter_user_id: String::new(),
                revoked: grant.end_utc.is_some_and(|end| end <= chrono::Utc::now()),
                created_utc: Some(datetime_to_timestamp(grant.start_utc)),
            }),
        }))
    }

    async fn revoke_visibility_grant(
        &self,
        request: Request<RevokeVisibilityGrantRequest>,
    ) -> Result<Response<RevokeVisibilityGrantResponse>, Status> {
        // Require visibility:revoke capability
        let _auth = require_capability(&self.state, &request, "visibility:revoke").await?;

        let req = request.into_inner();

        let grant_id = Uuid::parse_str(&req.grant_id)
            .map_err(|_| Status::invalid_argument("Invalid grant_id"))?;

        // Verify grant exists
        self.state
            .db
            .find_visibility_grant_by_id(grant_id)
            .await
            .map_err(|e| e.into_status())?
            .ok_or_else(|| Status::not_found("Visibility grant not found"))?;

        self.state
            .db
            .revoke_visibility_grant(grant_id)
            .await
            .map_err(|e| e.into_status())?;

        Ok(Response::new(RevokeVisibilityGrantResponse {
            message: "Visibility grant revoked successfully".to_string(),
        }))
    }

    async fn list_user_visibility_grants(
        &self,
        request: Request<ListUserVisibilityGrantsRequest>,
    ) -> Result<Response<ListUserVisibilityGrantsResponse>, Status> {
        // Require visibility:read capability
        let _auth = require_capability(&self.state, &request, "visibility:read").await?;

        let req = request.into_inner();

        let user_id = Uuid::parse_str(&req.user_id)
            .map_err(|_| Status::invalid_argument("Invalid user_id"))?;

        // Verify user exists
        self.state
            .db
            .find_user_by_id(user_id)
            .await
            .map_err(|e| e.into_status())?
            .ok_or_else(|| Status::not_found("User not found"))?;

        let grants = if req.active_only {
            self.state
                .db
                .find_active_visibility_grants_for_user(user_id)
                .await
                .map_err(|e| e.into_status())?
        } else {
            self.state
                .db
                .find_visibility_grants_for_user(user_id)
                .await
                .map_err(|e| e.into_status())?
        };

        let now = chrono::Utc::now();
        let proto_grants: Vec<VisibilityGrant> = grants
            .into_iter()
            .map(|g| VisibilityGrant {
                grant_id: g.grant_id.to_string(),
                tenant_id: g.tenant_id.to_string(),
                user_id: g.user_id.to_string(),
                org_node_id: g.org_node_id.to_string(),
                access_scope: g.access_scope_code.clone(),
                start_utc: Some(datetime_to_timestamp(g.start_utc)),
                end_utc: g.end_utc.map(datetime_to_timestamp),
                // Note: granter_user_id not tracked in current model
                granter_user_id: String::new(),
                revoked: g.end_utc.is_some_and(|end| end <= now),
                created_utc: Some(datetime_to_timestamp(g.start_utc)),
            })
            .collect();

        Ok(Response::new(ListUserVisibilityGrantsResponse {
            grants: proto_grants,
        }))
    }
}
