//! gRPC implementation of AssignmentService.

use crate::grpc::capability_check::require_capability;
use crate::grpc::proto::auth::{
    assignment_service_server::AssignmentService, Assignment as ProtoAssignment,
    CreateAssignmentRequest, CreateAssignmentResponse, EndAssignmentRequest, EndAssignmentResponse,
    ListUserAssignmentsRequest, ListUserAssignmentsResponse,
};
use crate::models::OrgAssignment;
use crate::AppState;
use prost_types::Timestamp;
use service_core::grpc::IntoStatus;
use tonic::{Request, Response, Status};
use uuid::Uuid;

/// gRPC AssignmentService implementation.
pub struct AssignmentServiceImpl {
    state: AppState,
}

impl AssignmentServiceImpl {
    /// Create a new AssignmentServiceImpl.
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
impl AssignmentService for AssignmentServiceImpl {
    async fn create_assignment(
        &self,
        request: Request<CreateAssignmentRequest>,
    ) -> Result<Response<CreateAssignmentResponse>, Status> {
        // Require org.assignment:create capability
        let _auth = require_capability(&self.state, &request, "org.assignment:create").await?;

        let req = request.into_inner();

        let user_id = Uuid::parse_str(&req.user_id)
            .map_err(|_| Status::invalid_argument("Invalid user_id"))?;
        let org_node_id = Uuid::parse_str(&req.org_node_id)
            .map_err(|_| Status::invalid_argument("Invalid org_node_id"))?;
        let role_id = Uuid::parse_str(&req.role_id)
            .map_err(|_| Status::invalid_argument("Invalid role_id"))?;

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

        // Verify role exists
        let role = self
            .state
            .db
            .find_role_by_id(role_id)
            .await
            .map_err(|e| e.into_status())?
            .ok_or_else(|| Status::not_found("Role not found"))?;

        // Verify org node exists
        let org_node = self
            .state
            .db
            .find_org_node_by_id(org_node_id)
            .await
            .map_err(|e| e.into_status())?
            .ok_or_else(|| Status::not_found("Org node not found"))?;

        // Verify role and org node are in the same tenant
        if role.tenant_id != org_node.tenant_id {
            return Err(Status::invalid_argument(
                "Role and org node must belong to the same tenant",
            ));
        }

        // Create assignment
        let mut assignment = OrgAssignment::new(role.tenant_id, user_id, org_node_id, role_id);

        // Set end_utc if provided
        if let Some(end) = end_utc {
            assignment.end_utc = Some(end);
        }

        self.state
            .db
            .insert_org_assignment(&assignment)
            .await
            .map_err(|e| e.into_status())?;

        Ok(Response::new(CreateAssignmentResponse {
            assignment: Some(ProtoAssignment {
                assignment_id: assignment.assignment_id.to_string(),
                user_id: assignment.user_id.to_string(),
                org_node_id: assignment.org_node_id.to_string(),
                role_id: assignment.role_id.to_string(),
                role_label: role.role_label,
                start_utc: Some(datetime_to_timestamp(assignment.start_utc)),
                end_utc: assignment.end_utc.map(datetime_to_timestamp),
                created_utc: Some(datetime_to_timestamp(assignment.start_utc)),
            }),
        }))
    }

    async fn end_assignment(
        &self,
        request: Request<EndAssignmentRequest>,
    ) -> Result<Response<EndAssignmentResponse>, Status> {
        // Require org.assignment:end capability
        let _auth = require_capability(&self.state, &request, "org.assignment:end").await?;

        let req = request.into_inner();

        let assignment_id = Uuid::parse_str(&req.assignment_id)
            .map_err(|_| Status::invalid_argument("Invalid assignment_id"))?;

        self.state
            .db
            .end_assignment(assignment_id)
            .await
            .map_err(|e| e.into_status())?;

        Ok(Response::new(EndAssignmentResponse {
            message: "Assignment ended successfully".to_string(),
        }))
    }

    async fn list_user_assignments(
        &self,
        request: Request<ListUserAssignmentsRequest>,
    ) -> Result<Response<ListUserAssignmentsResponse>, Status> {
        // Require org.assignment:read capability
        let _auth = require_capability(&self.state, &request, "org.assignment:read").await?;

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

        // Note: Currently only active assignments can be queried
        // TODO: Add find_all_assignments_for_user if needed
        let assignments = self
            .state
            .db
            .find_active_assignments_for_user(user_id)
            .await
            .map_err(|e| e.into_status())?;

        // Get role labels for assignments
        let mut proto_assignments = Vec::new();
        for a in assignments {
            let role_label = self
                .state
                .db
                .find_role_by_id(a.role_id)
                .await
                .map_err(|e| e.into_status())?
                .map(|r| r.role_label)
                .unwrap_or_default();

            proto_assignments.push(ProtoAssignment {
                assignment_id: a.assignment_id.to_string(),
                user_id: a.user_id.to_string(),
                org_node_id: a.org_node_id.to_string(),
                role_id: a.role_id.to_string(),
                role_label,
                start_utc: Some(datetime_to_timestamp(a.start_utc)),
                end_utc: a.end_utc.map(datetime_to_timestamp),
                created_utc: Some(datetime_to_timestamp(a.start_utc)),
            });
        }

        Ok(Response::new(ListUserAssignmentsResponse {
            assignments: proto_assignments,
        }))
    }
}
