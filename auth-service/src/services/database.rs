//! PostgreSQL database service for auth-service v2.
//!
//! Uses sqlx with compile-time checked queries.

use service_core::error::AppError;
use sqlx::postgres::PgPool;
use uuid::Uuid;

use crate::models::{
    AuditEvent, Capability, IdentProvider, Invitation, OrgAssignment, OrgNode, OtpCode,
    RefreshSession, Role, Service, ServiceSecret, Tenant, User, UserIdentity, VisibilityGrant,
};

/// PostgreSQL database wrapper.
#[derive(Clone)]
pub struct Database {
    pool: PgPool,
}

impl Database {
    /// Create a new database wrapper from a connection pool.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Get the underlying connection pool.
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Health check - ping the database.
    pub async fn health_check(&self) -> Result<(), AppError> {
        sqlx::query("SELECT 1")
            .execute(&self.pool)
            .await
            .map_err(|e| {
                tracing::error!("Database health check failed: {}", e);
                AppError::DatabaseError(anyhow::anyhow!("Database health check failed: {}", e))
            })?;
        Ok(())
    }

    // ==================== Tenant Operations ====================

    /// Find tenant by ID.
    pub async fn find_tenant_by_id(&self, tenant_id: Uuid) -> Result<Option<Tenant>, AppError> {
        sqlx::query_as::<_, Tenant>("SELECT * FROM tenants WHERE tenant_id = $1")
            .bind(tenant_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))
    }

    /// Find tenant by slug.
    pub async fn find_tenant_by_slug(&self, slug: &str) -> Result<Option<Tenant>, AppError> {
        sqlx::query_as::<_, Tenant>("SELECT * FROM tenants WHERE tenant_slug = $1")
            .bind(slug)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))
    }

    /// Insert a new tenant.
    pub async fn insert_tenant(&self, tenant: &Tenant) -> Result<(), AppError> {
        sqlx::query(
            r#"
            INSERT INTO tenants (tenant_id, tenant_slug, tenant_label, tenant_state_code, created_utc)
            VALUES ($1, $2, $3, $4, $5)
            "#,
        )
        .bind(tenant.tenant_id)
        .bind(&tenant.tenant_slug)
        .bind(&tenant.tenant_label)
        .bind(&tenant.tenant_state_code)
        .bind(tenant.created_utc)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))?;
        Ok(())
    }

    // ==================== User Operations ====================

    /// Find user by ID.
    pub async fn find_user_by_id(&self, user_id: Uuid) -> Result<Option<User>, AppError> {
        sqlx::query_as::<_, User>("SELECT * FROM users WHERE user_id = $1")
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))
    }

    /// Find user by email within a tenant.
    pub async fn find_user_by_email_in_tenant(
        &self,
        tenant_id: Uuid,
        email: &str,
    ) -> Result<Option<User>, AppError> {
        sqlx::query_as::<_, User>(
            "SELECT * FROM users WHERE tenant_id = $1 AND LOWER(email) = LOWER($2)",
        )
        .bind(tenant_id)
        .bind(email)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))
    }

    /// Insert a new user.
    pub async fn insert_user(&self, user: &User) -> Result<(), AppError> {
        sqlx::query(
            r#"
            INSERT INTO users (user_id, tenant_id, email, email_verified, google_id, display_name, user_state_code, created_utc)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
        )
        .bind(user.user_id)
        .bind(user.tenant_id)
        .bind(&user.email)
        .bind(user.email_verified)
        .bind(&user.google_id)
        .bind(&user.display_name)
        .bind(&user.user_state_code)
        .bind(user.created_utc)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))?;
        Ok(())
    }

    /// Update user email verified status.
    pub async fn update_user_email_verified(
        &self,
        user_id: Uuid,
        verified: bool,
    ) -> Result<(), AppError> {
        sqlx::query("UPDATE users SET email_verified = $1 WHERE user_id = $2")
            .bind(verified)
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))?;
        Ok(())
    }

    // ==================== User Identity Operations ====================

    /// Find user identity by user ID and provider.
    pub async fn find_user_identity(
        &self,
        user_id: Uuid,
        provider: &str,
    ) -> Result<Option<UserIdentity>, AppError> {
        sqlx::query_as::<_, UserIdentity>(
            "SELECT * FROM user_identities WHERE user_id = $1 AND ident_provider_code = $2",
        )
        .bind(user_id)
        .bind(provider)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))
    }

    /// Insert a new user identity.
    pub async fn insert_user_identity(&self, identity: &UserIdentity) -> Result<(), AppError> {
        sqlx::query(
            r#"
            INSERT INTO user_identities (ident_id, user_id, ident_provider_code, ident_hash, created_utc)
            VALUES ($1, $2, $3, $4, $5)
            "#,
        )
        .bind(identity.ident_id)
        .bind(identity.user_id)
        .bind(&identity.ident_provider_code)
        .bind(&identity.ident_hash)
        .bind(identity.created_utc)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))?;
        Ok(())
    }

    /// Find user identity by provider subject (e.g., Google sub) within a tenant.
    pub async fn find_user_identity_by_subject(
        &self,
        tenant_id: Uuid,
        provider: &IdentProvider,
        subject: &str,
    ) -> Result<Option<UserIdentity>, AppError> {
        sqlx::query_as::<_, UserIdentity>(
            r#"
            SELECT ui.* FROM user_identities ui
            JOIN users u ON ui.user_id = u.user_id
            WHERE u.tenant_id = $1 AND ui.ident_provider_code = $2 AND ui.ident_hash = $3
            "#,
        )
        .bind(tenant_id)
        .bind(provider.as_str())
        .bind(subject)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))
    }

    /// Update user identity hash (for password changes).
    pub async fn update_user_identity_hash(
        &self,
        user_id: Uuid,
        provider: &str,
        new_hash: &str,
    ) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE user_identities SET ident_hash = $1 WHERE user_id = $2 AND ident_provider_code = $3",
        )
        .bind(new_hash)
        .bind(user_id)
        .bind(provider)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))?;
        Ok(())
    }

    // ==================== Refresh Session Operations ====================

    /// Find refresh session by token hash.
    pub async fn find_refresh_session_by_hash(
        &self,
        token_hash: &str,
    ) -> Result<Option<RefreshSession>, AppError> {
        sqlx::query_as::<_, RefreshSession>(
            "SELECT * FROM refresh_sessions WHERE token_hash_text = $1 AND revoked_utc IS NULL",
        )
        .bind(token_hash)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))
    }

    /// Insert a new refresh session.
    pub async fn insert_refresh_session(&self, session: &RefreshSession) -> Result<(), AppError> {
        sqlx::query(
            r#"
            INSERT INTO refresh_sessions (session_id, user_id, token_hash_text, expiry_utc, revoked_utc, created_utc)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
        )
        .bind(session.session_id)
        .bind(session.user_id)
        .bind(&session.token_hash_text)
        .bind(session.expiry_utc)
        .bind(session.revoked_utc)
        .bind(session.created_utc)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))?;
        Ok(())
    }

    /// Revoke a refresh session.
    pub async fn revoke_refresh_session(&self, session_id: Uuid) -> Result<(), AppError> {
        sqlx::query("UPDATE refresh_sessions SET revoked_utc = NOW() WHERE session_id = $1")
            .bind(session_id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))?;
        Ok(())
    }

    /// Revoke all refresh sessions for a user.
    pub async fn revoke_all_user_sessions(&self, user_id: Uuid) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE refresh_sessions SET revoked_utc = NOW() WHERE user_id = $1 AND revoked_utc IS NULL",
        )
        .bind(user_id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))?;
        Ok(())
    }

    // ==================== Org Node Operations ====================

    /// Find org node by ID.
    pub async fn find_org_node_by_id(
        &self,
        org_node_id: Uuid,
    ) -> Result<Option<OrgNode>, AppError> {
        sqlx::query_as::<_, OrgNode>("SELECT * FROM org_nodes WHERE org_node_id = $1")
            .bind(org_node_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))
    }

    /// Find all org nodes for a tenant.
    pub async fn find_org_nodes_by_tenant(
        &self,
        tenant_id: Uuid,
    ) -> Result<Vec<OrgNode>, AppError> {
        sqlx::query_as::<_, OrgNode>(
            "SELECT * FROM org_nodes WHERE tenant_id = $1 AND active_flag = true ORDER BY node_label",
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))
    }

    /// Find descendants of an org node (using closure table).
    pub async fn find_org_node_descendants(
        &self,
        org_node_id: Uuid,
    ) -> Result<Vec<OrgNode>, AppError> {
        sqlx::query_as::<_, OrgNode>(
            r#"
            SELECT n.* FROM org_nodes n
            JOIN org_node_paths p ON n.org_node_id = p.descendant_org_node_id
            WHERE p.ancestor_org_node_id = $1 AND n.active_flag = true
            ORDER BY p.depth_val, n.node_label
            "#,
        )
        .bind(org_node_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))
    }

    /// Insert a new org node and update closure table.
    pub async fn insert_org_node(&self, node: &OrgNode) -> Result<(), AppError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))?;

        // Insert the node
        sqlx::query(
            r#"
            INSERT INTO org_nodes (org_node_id, tenant_id, node_type_code, node_label, parent_org_node_id, active_flag, created_utc)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
        )
        .bind(node.org_node_id)
        .bind(node.tenant_id)
        .bind(&node.node_type_code)
        .bind(&node.node_label)
        .bind(node.parent_org_node_id)
        .bind(node.active_flag)
        .bind(node.created_utc)
        .execute(&mut *tx)
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))?;

        // Insert self-reference in closure table
        sqlx::query(
            r#"
            INSERT INTO org_node_paths (tenant_id, ancestor_org_node_id, descendant_org_node_id, depth_val)
            VALUES ($1, $2, $2, 0)
            "#,
        )
        .bind(node.tenant_id)
        .bind(node.org_node_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))?;

        // If there's a parent, copy all ancestor paths
        if let Some(parent_id) = node.parent_org_node_id {
            sqlx::query(
                r#"
                INSERT INTO org_node_paths (tenant_id, ancestor_org_node_id, descendant_org_node_id, depth_val)
                SELECT $1, ancestor_org_node_id, $2, depth_val + 1
                FROM org_node_paths
                WHERE descendant_org_node_id = $3
                "#,
            )
            .bind(node.tenant_id)
            .bind(node.org_node_id)
            .bind(parent_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))?;
        }

        tx.commit()
            .await
            .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))?;
        Ok(())
    }

    // ==================== Role Operations ====================

    /// Find role by ID.
    pub async fn find_role_by_id(&self, role_id: Uuid) -> Result<Option<Role>, AppError> {
        sqlx::query_as::<_, Role>("SELECT * FROM roles WHERE role_id = $1")
            .bind(role_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))
    }

    /// Find all roles for a tenant.
    pub async fn find_roles_by_tenant(&self, tenant_id: Uuid) -> Result<Vec<Role>, AppError> {
        sqlx::query_as::<_, Role>("SELECT * FROM roles WHERE tenant_id = $1 ORDER BY role_label")
            .bind(tenant_id)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))
    }

    /// Insert a new role.
    pub async fn insert_role(&self, role: &Role) -> Result<(), AppError> {
        sqlx::query(
            r#"
            INSERT INTO roles (role_id, tenant_id, role_label, created_utc)
            VALUES ($1, $2, $3, $4)
            "#,
        )
        .bind(role.role_id)
        .bind(role.tenant_id)
        .bind(&role.role_label)
        .bind(role.created_utc)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))?;
        Ok(())
    }

    /// Get capabilities for a role.
    pub async fn get_role_capabilities(&self, role_id: Uuid) -> Result<Vec<String>, AppError> {
        let rows: Vec<(String,)> = sqlx::query_as(
            r#"
            SELECT c.cap_key FROM capabilities c
            JOIN role_capabilities rc ON c.cap_id = rc.cap_id
            WHERE rc.role_id = $1
            "#,
        )
        .bind(role_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))?;

        Ok(rows.into_iter().map(|(k,)| k).collect())
    }

    /// Assign capability to role.
    pub async fn assign_capability_to_role(
        &self,
        role_id: Uuid,
        cap_id: Uuid,
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO role_capabilities (role_id, cap_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
        )
        .bind(role_id)
        .bind(cap_id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))?;
        Ok(())
    }

    // ==================== Capability Operations ====================

    /// Find capability by ID.
    pub async fn find_capability_by_id(
        &self,
        cap_id: Uuid,
    ) -> Result<Option<Capability>, AppError> {
        sqlx::query_as::<_, Capability>("SELECT * FROM capabilities WHERE cap_id = $1")
            .bind(cap_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))
    }

    /// Find capability by key.
    pub async fn find_capability_by_key(
        &self,
        cap_key: &str,
    ) -> Result<Option<Capability>, AppError> {
        sqlx::query_as::<_, Capability>("SELECT * FROM capabilities WHERE cap_key = $1")
            .bind(cap_key)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))
    }

    /// Get all capabilities.
    pub async fn get_all_capabilities(&self) -> Result<Vec<Capability>, AppError> {
        sqlx::query_as::<_, Capability>("SELECT * FROM capabilities ORDER BY cap_key")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))
    }

    /// Insert a new capability.
    pub async fn insert_capability(&self, cap: &Capability) -> Result<(), AppError> {
        sqlx::query(
            r#"
            INSERT INTO capabilities (cap_id, cap_key, created_utc)
            VALUES ($1, $2, $3)
            "#,
        )
        .bind(cap.cap_id)
        .bind(&cap.cap_key)
        .bind(cap.created_utc)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))?;
        Ok(())
    }

    // ==================== Org Assignment Operations ====================

    /// Find active assignments for a user.
    pub async fn find_active_assignments_for_user(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<OrgAssignment>, AppError> {
        sqlx::query_as::<_, OrgAssignment>(
            r#"
            SELECT * FROM org_assignments
            WHERE user_id = $1
            AND start_utc <= NOW()
            AND (end_utc IS NULL OR end_utc > NOW())
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))
    }

    /// Insert a new org assignment.
    pub async fn insert_org_assignment(&self, assignment: &OrgAssignment) -> Result<(), AppError> {
        sqlx::query(
            r#"
            INSERT INTO org_assignments (assignment_id, tenant_id, user_id, org_node_id, role_id, start_utc, end_utc)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
        )
        .bind(assignment.assignment_id)
        .bind(assignment.tenant_id)
        .bind(assignment.user_id)
        .bind(assignment.org_node_id)
        .bind(assignment.role_id)
        .bind(assignment.start_utc)
        .bind(assignment.end_utc)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))?;
        Ok(())
    }

    /// End an assignment (set end_utc).
    pub async fn end_assignment(&self, assignment_id: Uuid) -> Result<(), AppError> {
        sqlx::query("UPDATE org_assignments SET end_utc = NOW() WHERE assignment_id = $1")
            .bind(assignment_id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))?;
        Ok(())
    }

    // ==================== Service (KYS) Operations ====================

    /// Find service by ID.
    pub async fn find_service_by_id(&self, svc_id: Uuid) -> Result<Option<Service>, AppError> {
        sqlx::query_as::<_, Service>("SELECT * FROM services WHERE svc_id = $1")
            .bind(svc_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))
    }

    /// Find service by key.
    pub async fn find_service_by_key(&self, svc_key: &str) -> Result<Option<Service>, AppError> {
        sqlx::query_as::<_, Service>("SELECT * FROM services WHERE svc_key = $1")
            .bind(svc_key)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))
    }

    /// Insert a new service.
    pub async fn insert_service(&self, service: &Service) -> Result<(), AppError> {
        sqlx::query(
            r#"
            INSERT INTO services (svc_id, tenant_id, svc_key, svc_label, svc_state_code, created_utc)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
        )
        .bind(service.svc_id)
        .bind(service.tenant_id)
        .bind(&service.svc_key)
        .bind(&service.svc_label)
        .bind(&service.svc_state_code)
        .bind(service.created_utc)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))?;
        Ok(())
    }

    /// Find valid service secret.
    pub async fn find_valid_service_secret(
        &self,
        svc_id: Uuid,
    ) -> Result<Option<ServiceSecret>, AppError> {
        sqlx::query_as::<_, ServiceSecret>(
            "SELECT * FROM service_secrets WHERE svc_id = $1 AND revoked_utc IS NULL ORDER BY created_utc DESC LIMIT 1",
        )
        .bind(svc_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))
    }

    /// Insert a new service secret.
    pub async fn insert_service_secret(&self, secret: &ServiceSecret) -> Result<(), AppError> {
        sqlx::query(
            r#"
            INSERT INTO service_secrets (secret_id, svc_id, secret_hash_text, created_utc, revoked_utc)
            VALUES ($1, $2, $3, $4, $5)
            "#,
        )
        .bind(secret.secret_id)
        .bind(secret.svc_id)
        .bind(&secret.secret_hash_text)
        .bind(secret.created_utc)
        .bind(secret.revoked_utc)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))?;
        Ok(())
    }

    /// Revoke a service secret.
    pub async fn revoke_service_secret(&self, secret_id: Uuid) -> Result<(), AppError> {
        sqlx::query("UPDATE service_secrets SET revoked_utc = NOW() WHERE secret_id = $1")
            .bind(secret_id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))?;
        Ok(())
    }

    /// Get service permissions.
    pub async fn get_service_permissions(&self, svc_id: Uuid) -> Result<Vec<String>, AppError> {
        let rows: Vec<(String,)> =
            sqlx::query_as("SELECT perm_key FROM service_permissions WHERE svc_id = $1")
                .bind(svc_id)
                .fetch_all(&self.pool)
                .await
                .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))?;

        Ok(rows.into_iter().map(|(k,)| k).collect())
    }

    /// Insert service permission.
    pub async fn insert_service_permission(
        &self,
        svc_id: Uuid,
        perm_key: &str,
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO service_permissions (svc_id, perm_key) VALUES ($1, $2) ON CONFLICT DO NOTHING",
        )
        .bind(svc_id)
        .bind(perm_key)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))?;
        Ok(())
    }

    // ==================== Audit Event Operations ====================

    /// Insert an audit event.
    pub async fn insert_audit_event(&self, event: &AuditEvent) -> Result<(), AppError> {
        sqlx::query(
            r#"
            INSERT INTO audit_events (event_id, tenant_id, actor_user_id, actor_svc_id, event_type_code, target_type, target_id, event_data, ip_address, user_agent, created_utc)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            "#,
        )
        .bind(event.event_id)
        .bind(event.tenant_id)
        .bind(event.actor_user_id)
        .bind(event.actor_svc_id)
        .bind(&event.event_type_code)
        .bind(&event.target_type)
        .bind(event.target_id)
        .bind(&event.event_data)
        .bind(&event.ip_address)
        .bind(&event.user_agent)
        .bind(event.created_utc)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))?;
        Ok(())
    }

    /// Find audit events with filtering and pagination.
    #[allow(clippy::too_many_arguments)]
    pub async fn find_audit_events(
        &self,
        tenant_id: Uuid,
        actor_user_id: Option<Uuid>,
        action_key: Option<&str>,
        entity_kind: Option<&str>,
        entity_id: Option<Uuid>,
        from_utc: Option<chrono::DateTime<chrono::Utc>>,
        to_utc: Option<chrono::DateTime<chrono::Utc>>,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<AuditEvent>, i64), AppError> {
        // Build dynamic WHERE clause
        let mut conditions = vec!["tenant_id = $1".to_string()];
        let mut param_idx = 2;

        if actor_user_id.is_some() {
            conditions.push(format!("actor_user_id = ${}", param_idx));
            param_idx += 1;
        }
        if action_key.is_some() {
            conditions.push(format!("event_type_code = ${}", param_idx));
            param_idx += 1;
        }
        if entity_kind.is_some() {
            conditions.push(format!("target_type = ${}", param_idx));
            param_idx += 1;
        }
        if entity_id.is_some() {
            conditions.push(format!("target_id = ${}", param_idx));
            param_idx += 1;
        }
        if from_utc.is_some() {
            conditions.push(format!("created_utc >= ${}", param_idx));
            param_idx += 1;
        }
        if to_utc.is_some() {
            conditions.push(format!("created_utc <= ${}", param_idx));
            param_idx += 1;
        }

        let where_clause = conditions.join(" AND ");

        // Count query
        let count_query = format!("SELECT COUNT(*) FROM audit_events WHERE {}", where_clause);

        // Data query
        let data_query =
            format!(
            "SELECT * FROM audit_events WHERE {} ORDER BY created_utc DESC LIMIT ${} OFFSET ${}",
            where_clause, param_idx, param_idx + 1
        );

        // Build and execute count query
        let mut count_q = sqlx::query_as::<_, (i64,)>(&count_query).bind(tenant_id);
        if let Some(user_id) = actor_user_id {
            count_q = count_q.bind(user_id);
        }
        if let Some(action) = action_key {
            count_q = count_q.bind(action);
        }
        if let Some(kind) = entity_kind {
            count_q = count_q.bind(kind);
        }
        if let Some(eid) = entity_id {
            count_q = count_q.bind(eid);
        }
        if let Some(from) = from_utc {
            count_q = count_q.bind(from);
        }
        if let Some(to) = to_utc {
            count_q = count_q.bind(to);
        }

        let (total,) = count_q
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))?;

        // Build and execute data query
        let mut data_q = sqlx::query_as::<_, AuditEvent>(&data_query).bind(tenant_id);
        if let Some(user_id) = actor_user_id {
            data_q = data_q.bind(user_id);
        }
        if let Some(action) = action_key {
            data_q = data_q.bind(action);
        }
        if let Some(kind) = entity_kind {
            data_q = data_q.bind(kind);
        }
        if let Some(eid) = entity_id {
            data_q = data_q.bind(eid);
        }
        if let Some(from) = from_utc {
            data_q = data_q.bind(from);
        }
        if let Some(to) = to_utc {
            data_q = data_q.bind(to);
        }
        data_q = data_q.bind(limit).bind(offset);

        let events = data_q
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))?;

        Ok((events, total))
    }

    // ==================== Invitation Operations ====================

    /// Find invitation by token hash.
    pub async fn find_invitation_by_token_hash(
        &self,
        token_hash: &str,
    ) -> Result<Option<Invitation>, AppError> {
        sqlx::query_as::<_, Invitation>(
            "SELECT * FROM invitations WHERE token_hash = $1 AND state_code = 'pending'",
        )
        .bind(token_hash)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))
    }

    /// Insert an invitation.
    pub async fn insert_invitation(&self, invitation: &Invitation) -> Result<(), AppError> {
        sqlx::query(
            r#"
            INSERT INTO invitations (invitation_id, tenant_id, email, org_node_id, role_id, token_hash, state_code, expiry_utc, accepted_utc, created_by_user_id, created_utc)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            "#,
        )
        .bind(invitation.invitation_id)
        .bind(invitation.tenant_id)
        .bind(&invitation.email)
        .bind(invitation.org_node_id)
        .bind(invitation.role_id)
        .bind(&invitation.token_hash)
        .bind(&invitation.state_code)
        .bind(invitation.expiry_utc)
        .bind(invitation.accepted_utc)
        .bind(invitation.created_by_user_id)
        .bind(invitation.created_utc)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))?;
        Ok(())
    }

    /// Mark invitation as accepted.
    pub async fn accept_invitation(&self, invitation_id: Uuid) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE invitations SET state_code = 'accepted', accepted_utc = NOW() WHERE invitation_id = $1",
        )
        .bind(invitation_id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))?;
        Ok(())
    }

    // ==================== Visibility Grant Operations ====================

    /// Find visibility grants for a user.
    pub async fn find_visibility_grants_for_user(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<VisibilityGrant>, AppError> {
        sqlx::query_as::<_, VisibilityGrant>("SELECT * FROM visibility_grants WHERE user_id = $1")
            .bind(user_id)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))
    }

    /// Find active visibility grants for a user (within time bounds).
    pub async fn find_active_visibility_grants_for_user(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<VisibilityGrant>, AppError> {
        sqlx::query_as::<_, VisibilityGrant>(
            r#"
            SELECT * FROM visibility_grants
            WHERE user_id = $1
              AND start_utc <= NOW()
              AND (end_utc IS NULL OR end_utc > NOW())
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))
    }

    /// Find visibility grant by ID.
    pub async fn find_visibility_grant_by_id(
        &self,
        grant_id: Uuid,
    ) -> Result<Option<VisibilityGrant>, AppError> {
        sqlx::query_as::<_, VisibilityGrant>("SELECT * FROM visibility_grants WHERE grant_id = $1")
            .bind(grant_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))
    }

    /// Insert a visibility grant.
    pub async fn insert_visibility_grant(&self, grant: &VisibilityGrant) -> Result<(), AppError> {
        sqlx::query(
            r#"
            INSERT INTO visibility_grants (grant_id, tenant_id, user_id, org_node_id, access_scope_code, start_utc, end_utc)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
        )
        .bind(grant.grant_id)
        .bind(grant.tenant_id)
        .bind(grant.user_id)
        .bind(grant.org_node_id)
        .bind(&grant.access_scope_code)
        .bind(grant.start_utc)
        .bind(grant.end_utc)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))?;
        Ok(())
    }

    /// Revoke a visibility grant by setting end_utc to now.
    pub async fn revoke_visibility_grant(&self, grant_id: Uuid) -> Result<(), AppError> {
        sqlx::query("UPDATE visibility_grants SET end_utc = NOW() WHERE grant_id = $1 AND (end_utc IS NULL OR end_utc > NOW())")
            .bind(grant_id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))?;
        Ok(())
    }

    // ==================== OTP Code Operations ====================

    /// Insert an OTP code.
    pub async fn insert_otp_code(&self, otp: &OtpCode) -> Result<(), AppError> {
        sqlx::query(
            r#"
            INSERT INTO otp_codes (otp_id, tenant_id, destination_text, channel_code, purpose_code, code_hash_text, expiry_utc, consumed_utc, attempt_count, attempt_max, created_utc)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            "#,
        )
        .bind(otp.otp_id)
        .bind(otp.tenant_id)
        .bind(&otp.destination_text)
        .bind(&otp.channel_code)
        .bind(&otp.purpose_code)
        .bind(&otp.code_hash_text)
        .bind(otp.expiry_utc)
        .bind(otp.consumed_utc)
        .bind(otp.attempt_count)
        .bind(otp.attempt_max)
        .bind(otp.created_utc)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))?;
        Ok(())
    }

    /// Find OTP by ID.
    pub async fn find_otp_by_id(&self, otp_id: Uuid) -> Result<Option<OtpCode>, AppError> {
        sqlx::query_as::<_, OtpCode>("SELECT * FROM otp_codes WHERE otp_id = $1")
            .bind(otp_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))
    }

    /// Count recent OTPs for a destination (rate limiting).
    pub async fn count_recent_otps(
        &self,
        destination: &str,
        seconds: i64,
    ) -> Result<i64, AppError> {
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM otp_codes WHERE destination_text = $1 AND created_utc > NOW() - INTERVAL '1 second' * $2",
        )
        .bind(destination)
        .bind(seconds)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))?;
        Ok(row.0)
    }

    /// Increment OTP attempt count.
    pub async fn increment_otp_attempts(&self, otp_id: Uuid) -> Result<(), AppError> {
        sqlx::query("UPDATE otp_codes SET attempt_count = attempt_count + 1 WHERE otp_id = $1")
            .bind(otp_id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))?;
        Ok(())
    }

    /// Mark OTP as consumed.
    pub async fn consume_otp(&self, otp_id: Uuid) -> Result<(), AppError> {
        sqlx::query("UPDATE otp_codes SET consumed_utc = NOW() WHERE otp_id = $1")
            .bind(otp_id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))?;
        Ok(())
    }

    /// Mark email as verified for a user.
    pub async fn mark_email_verified(&self, user_id: Uuid) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE user_identities SET email_verified_flag = true WHERE user_id = $1 AND ident_provider_code = 'password'",
        )
        .bind(user_id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))?;
        Ok(())
    }

    /// Mark phone as verified for a user.
    pub async fn mark_phone_verified(&self, user_id: Uuid) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE user_identities SET phone_verified_flag = true WHERE user_id = $1 AND ident_provider_code = 'password'",
        )
        .bind(user_id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))?;
        Ok(())
    }
}
