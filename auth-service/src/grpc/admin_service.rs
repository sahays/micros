//! gRPC implementation of AdminService.
//!
//! Provides administrative operations including the bootstrap mechanism
//! for creating the first tenant and superadmin user, and self-service
//! tenant creation.

use crate::grpc::proto::auth::{
    admin_service_server::AdminService, BootstrapRequest, BootstrapResponse,
    CreateTenantRequest, CreateTenantResponse,
};
use crate::models::{
    AuditEvent, AuditEventType, Capability, OrgAssignment, OrgNode, RefreshSession, Role, Tenant,
    User, UserIdentity,
};
use crate::services::{hash_password, CreateTenantSetup};
use crate::AppState;
use sha2::{Digest, Sha256};
use tonic::{Request, Response, Status};

/// gRPC AdminService implementation.
pub struct AdminServiceImpl {
    state: AppState,
}

impl AdminServiceImpl {
    /// Create a new AdminServiceImpl.
    pub fn new(state: AppState) -> Self {
        Self { state }
    }
}

/// Hash a token for storage.
fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    hex::encode(hasher.finalize())
}

#[tonic::async_trait]
impl AdminService for AdminServiceImpl {
    async fn bootstrap(
        &self,
        request: Request<BootstrapRequest>,
    ) -> Result<Response<BootstrapResponse>, Status> {
        let req = request.into_inner();

        // Validate input
        if req.tenant_slug.is_empty() {
            return Err(Status::invalid_argument("tenant_slug is required"));
        }
        if req.tenant_label.is_empty() {
            return Err(Status::invalid_argument("tenant_label is required"));
        }
        if req.admin_email.is_empty() {
            return Err(Status::invalid_argument("admin_email is required"));
        }
        if req.admin_password.len() < 8 {
            return Err(Status::invalid_argument(
                "admin_password must be at least 8 characters",
            ));
        }

        // Check if tenant slug is already taken
        if self
            .state
            .db
            .find_tenant_by_slug(&req.tenant_slug)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to check tenant slug");
                Status::internal("Database error")
            })?
            .is_some()
        {
            return Err(Status::already_exists("Tenant slug already exists"));
        }

        // 1. Create tenant
        let tenant = Tenant::new(req.tenant_slug.clone(), req.tenant_label.clone());
        let tenant_id = tenant.tenant_id;

        self.state.db.insert_tenant(&tenant).await.map_err(|e| {
            tracing::error!(error = %e, "Failed to create tenant");
            Status::internal("Failed to create tenant")
        })?;

        tracing::info!(tenant_id = %tenant_id, tenant_slug = %req.tenant_slug, "Created tenant");

        // 2. Create root org node
        let root_org_node = OrgNode::new(
            tenant_id,
            "org".to_string(),                    // node_type_code
            format!("{} Root", req.tenant_label), // node_label
            None,                                 // no parent - this is root
        );
        let root_org_node_id = root_org_node.org_node_id;

        self.state
            .db
            .insert_org_node(&root_org_node)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to create root org node");
                Status::internal("Failed to create root org node")
            })?;

        tracing::info!(org_node_id = %root_org_node_id, "Created root org node");

        // 3. Ensure "*" capability exists and create Superadmin role
        let superadmin_cap = match self
            .state
            .db
            .find_capability_by_key("*")
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to check superadmin capability");
                Status::internal("Database error")
            })? {
            Some(cap) => cap,
            None => {
                // Create the "*" capability if it doesn't exist
                let cap = Capability::new("*".to_string());
                self.state.db.insert_capability(&cap).await.map_err(|e| {
                    tracing::error!(error = %e, "Failed to create superadmin capability");
                    Status::internal("Failed to create superadmin capability")
                })?;
                tracing::info!("Created superadmin (*) capability");
                cap
            }
        };

        // 4. Create Superadmin role
        let superadmin_role = Role::new(tenant_id, "Superadmin".to_string());
        let superadmin_role_id = superadmin_role.role_id;

        self.state
            .db
            .insert_role(&superadmin_role)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to create superadmin role");
                Status::internal("Failed to create superadmin role")
            })?;

        // 5. Assign "*" capability to Superadmin role
        self.state
            .db
            .assign_capability_to_role(superadmin_role_id, superadmin_cap.cap_id)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to assign capability to superadmin role");
                Status::internal("Failed to assign capability to superadmin role")
            })?;

        tracing::info!(role_id = %superadmin_role_id, "Created Superadmin role with * capability");

        // 6. Create admin user
        let display_name = req
            .admin_display_name
            .unwrap_or_else(|| "Admin".to_string());
        let user = User::new(tenant_id, req.admin_email.clone(), Some(display_name));
        let admin_user_id = user.user_id;

        self.state.db.insert_user(&user).await.map_err(|e| {
            tracing::error!(error = %e, "Failed to create admin user");
            Status::internal("Failed to create admin user")
        })?;

        // 7. Create user identity with password
        let password_hash = hash_password(&req.admin_password).map_err(|e| {
            tracing::error!(error = %e, "Failed to hash password");
            Status::internal("Failed to hash password")
        })?;

        let identity = UserIdentity::new_password(admin_user_id, password_hash);
        self.state
            .db
            .insert_user_identity(&identity)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to create user identity");
                Status::internal("Failed to create user identity")
            })?;

        tracing::info!(user_id = %admin_user_id, email = %req.admin_email, "Created admin user");

        // 8. Create assignment (user -> root org -> superadmin role)
        let assignment = OrgAssignment::new(
            tenant_id,
            admin_user_id,
            root_org_node_id,
            superadmin_role_id,
        );

        self.state
            .db
            .insert_org_assignment(&assignment)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to create assignment");
                Status::internal("Failed to create assignment")
            })?;

        tracing::info!(
            assignment_id = %assignment.assignment_id,
            "Created superadmin assignment"
        );

        // 9. Generate tokens
        let (access_token, refresh_token, refresh_token_id) = self
            .state
            .jwt
            .generate_token_pair(
                &admin_user_id.to_string(),
                &tenant_id.to_string(),
                "", // org_id - empty for now
                &req.admin_email,
            )
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to generate tokens");
                Status::internal("Failed to generate tokens")
            })?;

        // 10. Store refresh session
        let refresh_hash = hash_token(&refresh_token_id);
        let session = RefreshSession::new(
            admin_user_id,
            refresh_hash,
            self.state.jwt.refresh_token_expiry_days(),
        );

        self.state
            .db
            .insert_refresh_session(&session)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to create refresh session");
                Status::internal("Failed to create refresh session")
            })?;

        tracing::info!(
            tenant_id = %tenant_id,
            admin_user_id = %admin_user_id,
            "Bootstrap completed successfully"
        );

        Ok(Response::new(BootstrapResponse {
            tenant_id: tenant_id.to_string(),
            root_org_node_id: root_org_node_id.to_string(),
            superadmin_role_id: superadmin_role_id.to_string(),
            admin_user_id: admin_user_id.to_string(),
            access_token,
            refresh_token,
        }))
    }

    async fn create_tenant(
        &self,
        request: Request<CreateTenantRequest>,
    ) -> Result<Response<CreateTenantResponse>, Status> {
        let req = request.into_inner();

        // Validate slug format: 3-63 chars, lowercase alphanumeric + hyphens,
        // no leading/trailing hyphens
        if req.tenant_slug.len() < 3 || req.tenant_slug.len() > 63 {
            return Err(Status::invalid_argument(
                "tenant_slug must be 3-63 characters",
            ));
        }
        if !req
            .tenant_slug
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        {
            return Err(Status::invalid_argument(
                "tenant_slug must contain only lowercase letters, digits, and hyphens",
            ));
        }
        if req.tenant_slug.starts_with('-') || req.tenant_slug.ends_with('-') {
            return Err(Status::invalid_argument(
                "tenant_slug must not start or end with a hyphen",
            ));
        }

        if req.tenant_label.is_empty() {
            return Err(Status::invalid_argument("tenant_label is required"));
        }
        if !req.admin_email.contains('@') {
            return Err(Status::invalid_argument(
                "admin_email must be a valid email address",
            ));
        }
        if req.admin_password.len() < 8 {
            return Err(Status::invalid_argument(
                "admin_password must be at least 8 characters",
            ));
        }

        // Check slug uniqueness (DB UNIQUE constraint is the real safety net)
        if self
            .state
            .db
            .find_tenant_by_slug(&req.tenant_slug)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to check tenant slug");
                Status::internal("Database error")
            })?
            .is_some()
        {
            return Err(Status::already_exists("Tenant slug already exists"));
        }

        // Hash password (CPU-intensive, done before transaction)
        let password_hash = hash_password(&req.admin_password).map_err(|e| {
            tracing::error!(error = %e, "Failed to hash password");
            Status::internal("Failed to hash password")
        })?;

        // Fetch all capabilities, filter out "*"
        let all_caps = self.state.db.get_all_capabilities().await.map_err(|e| {
            tracing::error!(error = %e, "Failed to fetch capabilities");
            Status::internal("Database error")
        })?;
        let cap_ids: Vec<_> = all_caps
            .iter()
            .filter(|c| c.cap_key != "*")
            .map(|c| c.cap_id)
            .collect();

        // Build all domain objects
        let tenant = Tenant::new(req.tenant_slug.clone(), req.tenant_label.clone());
        let tenant_id = tenant.tenant_id;

        let root_org_node = OrgNode::new(
            tenant_id,
            "org".to_string(),
            format!("{} Root", req.tenant_label),
            None,
        );
        let root_org_node_id = root_org_node.org_node_id;

        let admin_role = Role::new(tenant_id, "Admin".to_string());
        let admin_role_id = admin_role.role_id;

        let display_name = req
            .admin_display_name
            .unwrap_or_else(|| "Admin".to_string());
        let admin_user = User::new(tenant_id, req.admin_email.clone(), Some(display_name));
        let admin_user_id = admin_user.user_id;

        let admin_identity = UserIdentity::new_password(admin_user_id, password_hash);

        let assignment = OrgAssignment::new(tenant_id, admin_user_id, root_org_node_id, admin_role_id);

        // Generate tokens
        let (access_token, refresh_token, refresh_token_id) = self
            .state
            .jwt
            .generate_token_pair(
                &admin_user_id.to_string(),
                &tenant_id.to_string(),
                "",
                &req.admin_email,
            )
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to generate tokens");
                Status::internal("Failed to generate tokens")
            })?;

        let refresh_hash = hash_token(&refresh_token_id);
        let refresh_session = RefreshSession::new(
            admin_user_id,
            refresh_hash,
            self.state.jwt.refresh_token_expiry_days(),
        );

        let audit_event = AuditEvent::user_action(
            tenant_id,
            admin_user_id,
            AuditEventType::TenantCreated,
            Some("tenant".to_string()),
            Some(tenant_id),
            Some(serde_json::json!({
                "tenant_slug": req.tenant_slug,
            })),
            None,
            None,
        );

        // Execute everything in a single transaction
        let setup = CreateTenantSetup {
            tenant,
            root_org_node,
            admin_role,
            cap_ids,
            admin_user,
            admin_identity,
            assignment,
            refresh_session,
            audit_event,
        };

        self.state
            .db
            .create_tenant_with_setup(&setup)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to create tenant");
                Status::internal("Failed to create tenant")
            })?;

        tracing::info!(
            tenant_id = %tenant_id,
            tenant_slug = %req.tenant_slug,
            admin_user_id = %admin_user_id,
            "Tenant created successfully"
        );

        Ok(Response::new(CreateTenantResponse {
            tenant_id: tenant_id.to_string(),
            root_org_node_id: root_org_node_id.to_string(),
            admin_role_id: admin_role_id.to_string(),
            admin_user_id: admin_user_id.to_string(),
            access_token,
            refresh_token,
        }))
    }
}
