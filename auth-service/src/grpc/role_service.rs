//! gRPC implementation of RoleService.

use crate::grpc::proto::auth::{
    role_service_server::RoleService, AssignCapabilityRequest, AssignCapabilityResponse,
    Capability as ProtoCapability, CreateRoleRequest, CreateRoleResponse, GetCapabilityRequest,
    GetCapabilityResponse, GetRoleCapabilitiesRequest, GetRoleCapabilitiesResponse, GetRoleRequest,
    GetRoleResponse, ListCapabilitiesRequest, ListCapabilitiesResponse, ListTenantRolesRequest,
    ListTenantRolesResponse, Role as ProtoRole,
};
use crate::models::{Capability, Role};
use crate::AppState;
use prost_types::Timestamp;
use service_core::grpc::IntoStatus;
use tonic::{Request, Response, Status};
use uuid::Uuid;

/// gRPC RoleService implementation.
pub struct RoleServiceImpl {
    state: AppState,
}

impl RoleServiceImpl {
    /// Create a new RoleServiceImpl.
    pub fn new(state: AppState) -> Self {
        Self { state }
    }
}

/// Convert model Role to proto Role.
fn role_to_proto(role: Role) -> ProtoRole {
    ProtoRole {
        role_id: role.role_id.to_string(),
        tenant_id: role.tenant_id.to_string(),
        role_label: role.role_label,
        created_utc: Some(Timestamp {
            seconds: role.created_utc.timestamp(),
            nanos: role.created_utc.timestamp_subsec_nanos() as i32,
        }),
    }
}

/// Convert model Capability to proto Capability.
fn capability_to_proto(cap: Capability) -> ProtoCapability {
    ProtoCapability {
        cap_id: cap.cap_id.to_string(),
        cap_key: cap.cap_key,
        created_utc: Some(Timestamp {
            seconds: cap.created_utc.timestamp(),
            nanos: cap.created_utc.timestamp_subsec_nanos() as i32,
        }),
    }
}

#[tonic::async_trait]
impl RoleService for RoleServiceImpl {
    async fn create_role(
        &self,
        request: Request<CreateRoleRequest>,
    ) -> Result<Response<CreateRoleResponse>, Status> {
        let req = request.into_inner();

        let tenant_id = Uuid::parse_str(&req.tenant_id)
            .map_err(|_| Status::invalid_argument("Invalid tenant_id"))?;

        // Verify tenant exists
        let tenant = self
            .state
            .db
            .find_tenant_by_id(tenant_id)
            .await
            .map_err(|e| e.into_status())?
            .ok_or_else(|| Status::not_found("Tenant not found"))?;

        if !tenant.is_active() {
            return Err(Status::failed_precondition("Tenant is suspended"));
        }

        // Create role
        let role = Role::new(tenant_id, req.role_label);

        self.state
            .db
            .insert_role(&role)
            .await
            .map_err(|e| e.into_status())?;

        Ok(Response::new(CreateRoleResponse {
            role: Some(role_to_proto(role)),
        }))
    }

    async fn get_role(
        &self,
        request: Request<GetRoleRequest>,
    ) -> Result<Response<GetRoleResponse>, Status> {
        let req = request.into_inner();

        let role_id = Uuid::parse_str(&req.role_id)
            .map_err(|_| Status::invalid_argument("Invalid role_id"))?;

        let role = self
            .state
            .db
            .find_role_by_id(role_id)
            .await
            .map_err(|e| e.into_status())?
            .ok_or_else(|| Status::not_found("Role not found"))?;

        let capabilities = self
            .state
            .db
            .get_role_capabilities(role_id)
            .await
            .map_err(|e| e.into_status())?;

        Ok(Response::new(GetRoleResponse {
            role: Some(role_to_proto(role)),
            capabilities,
        }))
    }

    async fn list_tenant_roles(
        &self,
        request: Request<ListTenantRolesRequest>,
    ) -> Result<Response<ListTenantRolesResponse>, Status> {
        let req = request.into_inner();

        let tenant_id = Uuid::parse_str(&req.tenant_id)
            .map_err(|_| Status::invalid_argument("Invalid tenant_id"))?;

        // Verify tenant exists
        self.state
            .db
            .find_tenant_by_id(tenant_id)
            .await
            .map_err(|e| e.into_status())?
            .ok_or_else(|| Status::not_found("Tenant not found"))?;

        let roles = self
            .state
            .db
            .find_roles_by_tenant(tenant_id)
            .await
            .map_err(|e| e.into_status())?;

        let proto_roles: Vec<ProtoRole> = roles.into_iter().map(role_to_proto).collect();

        Ok(Response::new(ListTenantRolesResponse {
            roles: proto_roles,
        }))
    }

    async fn get_role_capabilities(
        &self,
        request: Request<GetRoleCapabilitiesRequest>,
    ) -> Result<Response<GetRoleCapabilitiesResponse>, Status> {
        let req = request.into_inner();

        let role_id = Uuid::parse_str(&req.role_id)
            .map_err(|_| Status::invalid_argument("Invalid role_id"))?;

        // Verify role exists
        self.state
            .db
            .find_role_by_id(role_id)
            .await
            .map_err(|e| e.into_status())?
            .ok_or_else(|| Status::not_found("Role not found"))?;

        let capabilities = self
            .state
            .db
            .get_role_capabilities(role_id)
            .await
            .map_err(|e| e.into_status())?;

        Ok(Response::new(GetRoleCapabilitiesResponse { capabilities }))
    }

    async fn assign_capability(
        &self,
        request: Request<AssignCapabilityRequest>,
    ) -> Result<Response<AssignCapabilityResponse>, Status> {
        let req = request.into_inner();

        let role_id = Uuid::parse_str(&req.role_id)
            .map_err(|_| Status::invalid_argument("Invalid role_id"))?;

        // Verify role exists
        self.state
            .db
            .find_role_by_id(role_id)
            .await
            .map_err(|e| e.into_status())?
            .ok_or_else(|| Status::not_found("Role not found"))?;

        // Verify capability exists
        let capability = self
            .state
            .db
            .find_capability_by_key(&req.capability_key)
            .await
            .map_err(|e| e.into_status())?
            .ok_or_else(|| Status::not_found("Capability not found"))?;

        // Assign capability to role
        self.state
            .db
            .assign_capability_to_role(role_id, capability.cap_id)
            .await
            .map_err(|e| e.into_status())?;

        Ok(Response::new(AssignCapabilityResponse {
            message: format!("Capability '{}' assigned to role", req.capability_key),
        }))
    }

    async fn list_capabilities(
        &self,
        _request: Request<ListCapabilitiesRequest>,
    ) -> Result<Response<ListCapabilitiesResponse>, Status> {
        let capabilities = self
            .state
            .db
            .get_all_capabilities()
            .await
            .map_err(|e| e.into_status())?;

        let proto_caps: Vec<ProtoCapability> =
            capabilities.into_iter().map(capability_to_proto).collect();

        Ok(Response::new(ListCapabilitiesResponse {
            capabilities: proto_caps,
        }))
    }

    async fn get_capability(
        &self,
        request: Request<GetCapabilityRequest>,
    ) -> Result<Response<GetCapabilityResponse>, Status> {
        let req = request.into_inner();

        let capability = self
            .state
            .db
            .find_capability_by_key(&req.cap_key)
            .await
            .map_err(|e| e.into_status())?
            .ok_or_else(|| Status::not_found("Capability not found"))?;

        Ok(Response::new(GetCapabilityResponse {
            capability: Some(capability_to_proto(capability)),
        }))
    }
}
