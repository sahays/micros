//! User and user identity database operations.

use service_core::error::AppError;
use uuid::Uuid;

use crate::models::{IdentProvider, User, UserIdentity};
use crate::services::database::Database;

impl Database {
    // ==================== User Operations ====================

    /// Find user by ID.
    pub async fn find_user_by_id(&self, user_id: Uuid) -> Result<Option<User>, AppError> {
        sqlx::query_as::<_, User>("SELECT * FROM users WHERE user_id = $1")
            .bind(user_id)
            .fetch_optional(self.pool())
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
        .fetch_optional(self.pool())
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
        .execute(self.pool())
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
            .execute(self.pool())
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
        .fetch_optional(self.pool())
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
        .execute(self.pool())
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
        .fetch_optional(self.pool())
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
        .execute(self.pool())
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))?;
        Ok(())
    }
}
