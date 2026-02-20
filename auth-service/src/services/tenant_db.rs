//! Tenant database operations.

use service_core::error::AppError;
use uuid::Uuid;

use crate::models::{AuditEvent, OrgAssignment, OrgNode, RefreshSession, Role, Tenant, User, UserIdentity};
use crate::services::database::Database;

/// All domain objects needed to create a tenant in a single transaction.
pub struct CreateTenantSetup {
    pub tenant: Tenant,
    pub root_org_node: OrgNode,
    pub admin_role: Role,
    pub cap_ids: Vec<Uuid>,
    pub admin_user: User,
    pub admin_identity: UserIdentity,
    pub assignment: OrgAssignment,
    pub refresh_session: RefreshSession,
    pub audit_event: AuditEvent,
}

impl Database {
    // ==================== Tenant Operations ====================

    /// Find tenant by ID.
    pub async fn find_tenant_by_id(&self, tenant_id: Uuid) -> Result<Option<Tenant>, AppError> {
        sqlx::query_as::<_, Tenant>("SELECT * FROM tenants WHERE tenant_id = $1")
            .bind(tenant_id)
            .fetch_optional(self.pool())
            .await
            .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))
    }

    /// Find tenant by slug.
    pub async fn find_tenant_by_slug(&self, slug: &str) -> Result<Option<Tenant>, AppError> {
        sqlx::query_as::<_, Tenant>("SELECT * FROM tenants WHERE tenant_slug = $1")
            .bind(slug)
            .fetch_optional(self.pool())
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
        .execute(self.pool())
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))?;
        Ok(())
    }

    /// Count total number of tenants.
    pub async fn count_tenants(&self) -> Result<i64, AppError> {
        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM tenants")
            .fetch_one(self.pool())
            .await
            .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))?;
        Ok(row.0)
    }

    /// Create a tenant with full setup in a single transaction.
    ///
    /// Inserts: tenant, root org node + closure self-entry, admin role,
    /// role-capability mappings, admin user + password identity,
    /// assignment, refresh session, and audit event.
    /// If any insert fails, the entire transaction rolls back.
    pub async fn create_tenant_with_setup(&self, setup: &CreateTenantSetup) -> Result<(), AppError> {
        let mut tx = self
            .pool()
            .begin()
            .await
            .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))?;

        // 1. Insert tenant
        sqlx::query(
            r#"
            INSERT INTO tenants (tenant_id, tenant_slug, tenant_label, tenant_state_code, created_utc)
            VALUES ($1, $2, $3, $4, $5)
            "#,
        )
        .bind(setup.tenant.tenant_id)
        .bind(&setup.tenant.tenant_slug)
        .bind(&setup.tenant.tenant_label)
        .bind(&setup.tenant.tenant_state_code)
        .bind(setup.tenant.created_utc)
        .execute(&mut *tx)
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))?;

        // 2. Insert root org node
        sqlx::query(
            r#"
            INSERT INTO org_nodes (org_node_id, tenant_id, node_type_code, node_label, parent_org_node_id, active_flag, created_utc)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
        )
        .bind(setup.root_org_node.org_node_id)
        .bind(setup.root_org_node.tenant_id)
        .bind(&setup.root_org_node.node_type_code)
        .bind(&setup.root_org_node.node_label)
        .bind(setup.root_org_node.parent_org_node_id)
        .bind(setup.root_org_node.active_flag)
        .bind(setup.root_org_node.created_utc)
        .execute(&mut *tx)
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))?;

        // 3. Insert closure table self-entry for root org node
        sqlx::query(
            r#"
            INSERT INTO org_node_paths (tenant_id, ancestor_org_node_id, descendant_org_node_id, depth_val)
            VALUES ($1, $2, $2, 0)
            "#,
        )
        .bind(setup.root_org_node.tenant_id)
        .bind(setup.root_org_node.org_node_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))?;

        // 4. Insert admin role
        sqlx::query(
            r#"
            INSERT INTO roles (role_id, tenant_id, role_label, created_utc)
            VALUES ($1, $2, $3, $4)
            "#,
        )
        .bind(setup.admin_role.role_id)
        .bind(setup.admin_role.tenant_id)
        .bind(&setup.admin_role.role_label)
        .bind(setup.admin_role.created_utc)
        .execute(&mut *tx)
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))?;

        // 5. Insert role-capability mappings
        for cap_id in &setup.cap_ids {
            sqlx::query(
                "INSERT INTO role_capabilities (role_id, cap_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
            )
            .bind(setup.admin_role.role_id)
            .bind(cap_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))?;
        }

        // 6. Insert admin user
        sqlx::query(
            r#"
            INSERT INTO users (user_id, tenant_id, email, email_verified, google_id, display_name, user_state_code, created_utc)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
        )
        .bind(setup.admin_user.user_id)
        .bind(setup.admin_user.tenant_id)
        .bind(&setup.admin_user.email)
        .bind(setup.admin_user.email_verified)
        .bind(&setup.admin_user.google_id)
        .bind(&setup.admin_user.display_name)
        .bind(&setup.admin_user.user_state_code)
        .bind(setup.admin_user.created_utc)
        .execute(&mut *tx)
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))?;

        // 7. Insert password identity
        sqlx::query(
            r#"
            INSERT INTO user_identities (ident_id, user_id, ident_provider_code, ident_hash, created_utc)
            VALUES ($1, $2, $3, $4, $5)
            "#,
        )
        .bind(setup.admin_identity.ident_id)
        .bind(setup.admin_identity.user_id)
        .bind(&setup.admin_identity.ident_provider_code)
        .bind(&setup.admin_identity.ident_hash)
        .bind(setup.admin_identity.created_utc)
        .execute(&mut *tx)
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))?;

        // 8. Insert assignment (user -> root org -> admin role)
        sqlx::query(
            r#"
            INSERT INTO org_assignments (assignment_id, tenant_id, user_id, org_node_id, role_id, start_utc, end_utc)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
        )
        .bind(setup.assignment.assignment_id)
        .bind(setup.assignment.tenant_id)
        .bind(setup.assignment.user_id)
        .bind(setup.assignment.org_node_id)
        .bind(setup.assignment.role_id)
        .bind(setup.assignment.start_utc)
        .bind(setup.assignment.end_utc)
        .execute(&mut *tx)
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))?;

        // 9. Insert refresh session
        sqlx::query(
            r#"
            INSERT INTO refresh_sessions (session_id, user_id, token_hash_text, expiry_utc, revoked_utc, created_utc)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
        )
        .bind(setup.refresh_session.session_id)
        .bind(setup.refresh_session.user_id)
        .bind(&setup.refresh_session.token_hash_text)
        .bind(setup.refresh_session.expiry_utc)
        .bind(setup.refresh_session.revoked_utc)
        .bind(setup.refresh_session.created_utc)
        .execute(&mut *tx)
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))?;

        // 10. Insert audit event
        sqlx::query(
            r#"
            INSERT INTO audit_events (event_id, tenant_id, actor_user_id, actor_svc_id, event_type_code, target_type, target_id, event_data, ip_address, user_agent, created_utc)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            "#,
        )
        .bind(setup.audit_event.event_id)
        .bind(setup.audit_event.tenant_id)
        .bind(setup.audit_event.actor_user_id)
        .bind(setup.audit_event.actor_svc_id)
        .bind(&setup.audit_event.event_type_code)
        .bind(&setup.audit_event.target_type)
        .bind(setup.audit_event.target_id)
        .bind(&setup.audit_event.event_data)
        .bind(&setup.audit_event.ip_address)
        .bind(&setup.audit_event.user_agent)
        .bind(setup.audit_event.created_utc)
        .execute(&mut *tx)
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))?;

        tx.commit()
            .await
            .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))?;

        Ok(())
    }
}
