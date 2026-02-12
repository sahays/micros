//! Invitation database operations.

use service_core::error::AppError;
use uuid::Uuid;

use crate::models::Invitation;
use crate::services::database::Database;

impl Database {
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
        .fetch_optional(self.pool())
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))
    }

    /// Insert an invitation.
    pub async fn insert_invitation(&self, invitation: &Invitation) -> Result<(), AppError> {
        sqlx::query(
            r#"
            INSERT INTO invitations (invitation_id, tenant_id, email, org_node_id, role_id, token_hash, state_code, expiry_utc, accepted_utc, created_by_user_id, created_utc, phone, verification_type, metadata_json)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
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
        .bind(&invitation.phone)
        .bind(&invitation.verification_type)
        .bind(&invitation.metadata_json)
        .execute(self.pool())
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
        .execute(self.pool())
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))?;
        Ok(())
    }
}
