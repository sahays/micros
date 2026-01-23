//! gRPC implementation of OrgService.

use crate::grpc::capability_check::require_capability;
use crate::grpc::proto::auth::{
    org_service_server::OrgService, CreateOrgNodeRequest, CreateOrgNodeResponse,
    GetOrgNodeDescendantsRequest, GetOrgNodeDescendantsResponse, GetOrgNodeRequest,
    GetOrgNodeResponse, GetTenantOrgTreeRequest, GetTenantOrgTreeResponse,
    ListTenantOrgNodesRequest, ListTenantOrgNodesResponse, OrgNode, OrgNodeTree,
};
use crate::models::OrgNode as ModelOrgNode;
use crate::AppState;
use prost_types::Timestamp;
use service_core::grpc::IntoStatus;
use tonic::{Request, Response, Status};
use uuid::Uuid;

/// gRPC OrgService implementation.
pub struct OrgServiceImpl {
    state: AppState,
}

impl OrgServiceImpl {
    /// Create a new OrgServiceImpl.
    pub fn new(state: AppState) -> Self {
        Self { state }
    }
}

/// Convert model OrgNode to proto OrgNode.
fn model_to_proto(node: ModelOrgNode) -> OrgNode {
    OrgNode {
        org_node_id: node.org_node_id.to_string(),
        tenant_id: node.tenant_id.to_string(),
        parent_org_node_id: node.parent_org_node_id.map(|id| id.to_string()),
        node_type_code: node.node_type_code,
        node_label: node.node_label,
        active_flag: node.active_flag,
        created_utc: Some(Timestamp {
            seconds: node.created_utc.timestamp(),
            nanos: node.created_utc.timestamp_subsec_nanos() as i32,
        }),
    }
}

/// Build tree structure from flat list of org nodes.
fn build_org_tree(nodes: Vec<ModelOrgNode>) -> Vec<OrgNodeTree> {
    use std::collections::HashMap;

    let mut children_map: HashMap<Uuid, Vec<ModelOrgNode>> = HashMap::new();
    let mut root_nodes: Vec<ModelOrgNode> = Vec::new();

    for node in nodes {
        if let Some(parent_id) = node.parent_org_node_id {
            children_map.entry(parent_id).or_default().push(node);
        } else {
            root_nodes.push(node);
        }
    }

    fn build_subtree(
        node: ModelOrgNode,
        children_map: &HashMap<Uuid, Vec<ModelOrgNode>>,
    ) -> OrgNodeTree {
        let node_id = node.org_node_id;
        let children = children_map
            .get(&node_id)
            .map(|children| {
                children
                    .iter()
                    .cloned()
                    .map(|child| build_subtree(child, children_map))
                    .collect()
            })
            .unwrap_or_default();

        OrgNodeTree {
            node: Some(model_to_proto(node)),
            children,
        }
    }

    root_nodes
        .into_iter()
        .map(|node| build_subtree(node, &children_map))
        .collect()
}

#[tonic::async_trait]
impl OrgService for OrgServiceImpl {
    async fn create_org_node(
        &self,
        request: Request<CreateOrgNodeRequest>,
    ) -> Result<Response<CreateOrgNodeResponse>, Status> {
        // Require org.node:create capability
        let _auth = require_capability(&self.state, &request, "org.node:create").await?;

        let req = request.into_inner();

        let tenant_id = Uuid::parse_str(&req.tenant_id)
            .map_err(|_| Status::invalid_argument("Invalid tenant_id"))?;

        let parent_org_node_id = req
            .parent_org_node_id
            .map(|id| Uuid::parse_str(&id))
            .transpose()
            .map_err(|_| Status::invalid_argument("Invalid parent_org_node_id"))?;

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

        // Verify parent if specified
        if let Some(parent_id) = parent_org_node_id {
            let parent = self
                .state
                .db
                .find_org_node_by_id(parent_id)
                .await
                .map_err(|e| e.into_status())?
                .ok_or_else(|| Status::not_found("Parent org node not found"))?;

            if parent.tenant_id != tenant_id {
                return Err(Status::invalid_argument(
                    "Parent org node belongs to different tenant",
                ));
            }
        }

        // Create org node
        let node = ModelOrgNode::new(
            tenant_id,
            req.node_type_code,
            req.node_label,
            parent_org_node_id,
        );

        self.state
            .db
            .insert_org_node(&node)
            .await
            .map_err(|e| e.into_status())?;

        Ok(Response::new(CreateOrgNodeResponse {
            org_node: Some(model_to_proto(node)),
        }))
    }

    async fn get_org_node(
        &self,
        request: Request<GetOrgNodeRequest>,
    ) -> Result<Response<GetOrgNodeResponse>, Status> {
        // Require org.node:read capability
        let _auth = require_capability(&self.state, &request, "org.node:read").await?;

        let req = request.into_inner();

        let org_node_id = Uuid::parse_str(&req.org_node_id)
            .map_err(|_| Status::invalid_argument("Invalid org_node_id"))?;

        let node = self
            .state
            .db
            .find_org_node_by_id(org_node_id)
            .await
            .map_err(|e| e.into_status())?
            .ok_or_else(|| Status::not_found("Org node not found"))?;

        Ok(Response::new(GetOrgNodeResponse {
            org_node: Some(model_to_proto(node)),
        }))
    }

    async fn get_org_node_descendants(
        &self,
        request: Request<GetOrgNodeDescendantsRequest>,
    ) -> Result<Response<GetOrgNodeDescendantsResponse>, Status> {
        // Require org.node:read capability
        let _auth = require_capability(&self.state, &request, "org.node:read").await?;

        let req = request.into_inner();

        let org_node_id = Uuid::parse_str(&req.org_node_id)
            .map_err(|_| Status::invalid_argument("Invalid org_node_id"))?;

        // Verify org node exists
        self.state
            .db
            .find_org_node_by_id(org_node_id)
            .await
            .map_err(|e| e.into_status())?
            .ok_or_else(|| Status::not_found("Org node not found"))?;

        let descendants = self
            .state
            .db
            .find_org_node_descendants(org_node_id)
            .await
            .map_err(|e| e.into_status())?;

        let proto_descendants: Vec<OrgNode> = descendants.into_iter().map(model_to_proto).collect();

        Ok(Response::new(GetOrgNodeDescendantsResponse {
            descendants: proto_descendants,
        }))
    }

    async fn list_tenant_org_nodes(
        &self,
        request: Request<ListTenantOrgNodesRequest>,
    ) -> Result<Response<ListTenantOrgNodesResponse>, Status> {
        // Require org.node:read capability
        let _auth = require_capability(&self.state, &request, "org.node:read").await?;

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

        let nodes = self
            .state
            .db
            .find_org_nodes_by_tenant(tenant_id)
            .await
            .map_err(|e| e.into_status())?;

        let proto_nodes: Vec<OrgNode> = nodes.into_iter().map(model_to_proto).collect();

        Ok(Response::new(ListTenantOrgNodesResponse {
            org_nodes: proto_nodes,
        }))
    }

    async fn get_tenant_org_tree(
        &self,
        request: Request<GetTenantOrgTreeRequest>,
    ) -> Result<Response<GetTenantOrgTreeResponse>, Status> {
        // Require org.node:read capability
        let _auth = require_capability(&self.state, &request, "org.node:read").await?;

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

        let nodes = self
            .state
            .db
            .find_org_nodes_by_tenant(tenant_id)
            .await
            .map_err(|e| e.into_status())?;

        let tree = build_org_tree(nodes);

        Ok(Response::new(GetTenantOrgTreeResponse { roots: tree }))
    }
}
