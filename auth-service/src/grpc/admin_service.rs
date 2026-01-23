//! gRPC implementation of AdminService.
//!
//! Provides administrative operations including the bootstrap mechanism
//! for creating the first tenant and superadmin user.

use crate::grpc::capability_check::require_admin_api_key;
use crate::grpc::proto::auth::{
    admin_service_server::AdminService, BootstrapRequest, BootstrapResponse,
};
use crate::models::{
    Capability, OrgAssignment, OrgNode, RefreshSession, Role, Tenant, User, UserIdentity,
};
use crate::AppState;
use argon2::{
    password_hash::{rand_core::OsRng, SaltString},
    Argon2, PasswordHasher,
};
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

/// Hash a password using argon2.
fn hash_password(password: &str) -> Result<String, argon2::password_hash::Error> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    argon2
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
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
        // Validate admin API key
        require_admin_api_key(&self.state.config, &request)?;

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

        // Check if bootstrap has already been performed
        let tenant_count = self.state.db.count_tenants().await.map_err(|e| {
            tracing::error!(error = %e, "Failed to count tenants");
            Status::internal("Database error")
        })?;

        if tenant_count > 0 {
            return Err(Status::failed_precondition(
                "Bootstrap already completed. System already has tenants.",
            ));
        }

        // Check if tenant slug is already taken (extra safety)
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
}
