//! PostgreSQL database service for auth-service v2.
//!
//! Uses sqlx with compile-time checked queries.

use service_core::error::AppError;
use sqlx::postgres::PgPool;

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
}
