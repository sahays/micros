//! Refresh session database operations.

use service_core::error::AppError;
use uuid::Uuid;

use crate::models::RefreshSession;
use crate::services::database::Database;

impl Database {
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
        .fetch_optional(self.pool())
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
        .execute(self.pool())
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))?;
        Ok(())
    }

    /// Revoke a refresh session.
    pub async fn revoke_refresh_session(&self, session_id: Uuid) -> Result<(), AppError> {
        sqlx::query("UPDATE refresh_sessions SET revoked_utc = NOW() WHERE session_id = $1")
            .bind(session_id)
            .execute(self.pool())
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
        .execute(self.pool())
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))?;
        Ok(())
    }
}
