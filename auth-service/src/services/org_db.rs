//! Org node database operations.

use service_core::error::AppError;
use uuid::Uuid;

use crate::models::OrgNode;
use crate::services::database::Database;

impl Database {
    // ==================== Org Node Operations ====================

    /// Find org node by ID.
    pub async fn find_org_node_by_id(
        &self,
        org_node_id: Uuid,
    ) -> Result<Option<OrgNode>, AppError> {
        sqlx::query_as::<_, OrgNode>("SELECT * FROM org_nodes WHERE org_node_id = $1")
            .bind(org_node_id)
            .fetch_optional(self.pool())
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
        .fetch_all(self.pool())
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
        .fetch_all(self.pool())
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))
    }

    /// Find ancestors of an org node (using closure table), including itself.
    pub async fn find_org_node_ancestor_ids(
        &self,
        org_node_id: Uuid,
    ) -> Result<Vec<Uuid>, AppError> {
        sqlx::query_scalar::<_, Uuid>(
            r#"
            SELECT p.ancestor_org_node_id
            FROM org_node_paths p
            WHERE p.descendant_org_node_id = $1
            "#,
        )
        .bind(org_node_id)
        .fetch_all(self.pool())
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))
    }

    /// Insert a new org node and update closure table.
    pub async fn insert_org_node(&self, node: &OrgNode) -> Result<(), AppError> {
        let mut tx = self
            .pool()
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
}
