//! OTP code database operations.

use service_core::error::AppError;
use uuid::Uuid;

use crate::models::OtpCode;
use crate::services::database::Database;

impl Database {
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
        .execute(self.pool())
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))?;
        Ok(())
    }

    /// Find OTP by ID.
    pub async fn find_otp_by_id(&self, otp_id: Uuid) -> Result<Option<OtpCode>, AppError> {
        sqlx::query_as::<_, OtpCode>("SELECT * FROM otp_codes WHERE otp_id = $1")
            .bind(otp_id)
            .fetch_optional(self.pool())
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
        .fetch_one(self.pool())
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))?;
        Ok(row.0)
    }

    /// Increment OTP attempt count.
    pub async fn increment_otp_attempts(&self, otp_id: Uuid) -> Result<(), AppError> {
        sqlx::query("UPDATE otp_codes SET attempt_count = attempt_count + 1 WHERE otp_id = $1")
            .bind(otp_id)
            .execute(self.pool())
            .await
            .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))?;
        Ok(())
    }

    /// Mark OTP as consumed.
    pub async fn consume_otp(&self, otp_id: Uuid) -> Result<(), AppError> {
        sqlx::query("UPDATE otp_codes SET consumed_utc = NOW() WHERE otp_id = $1")
            .bind(otp_id)
            .execute(self.pool())
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
        .execute(self.pool())
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
        .execute(self.pool())
        .await
        .map_err(|e| AppError::DatabaseError(anyhow::anyhow!(e)))?;
        Ok(())
    }
}
