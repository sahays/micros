use service_core::error::AppError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ServiceError {
    #[error("Database error: {0}")]
    Database(#[from] mongodb::error::Error),

    #[error("Redis error: {0}")]
    Redis(#[from] redis::RedisError),

    #[error("Internal error: {0}")]
    InternalString(String),

    #[error("Internal server error: {0}")]
    Internal(#[from] anyhow::Error),

    #[error("Invalid credentials")]
    InvalidCredentials,

    #[error("User already exists")]
    UserAlreadyExists,

    #[error("Email already registered")]
    EmailAlreadyRegistered,

    #[error("Invalid token")]
    InvalidToken,

    #[error("Token expired")]
    TokenExpired,

    #[error("User not found")]
    UserNotFound,

    #[error("Email error: {0}")]
    EmailError(String),

    #[error("Validation error: {0}")]
    ValidationError(String),
}

impl From<ServiceError> for AppError {
    fn from(err: ServiceError) -> Self {
        match err {
            ServiceError::Database(e) => AppError::DatabaseError(e),
            ServiceError::Redis(e) => AppError::RedisError(e),
            ServiceError::Internal(e) => AppError::InternalError(e),
            ServiceError::InternalString(e) => AppError::InternalError(anyhow::anyhow!(e)),
            ServiceError::InvalidCredentials => AppError::AuthError(anyhow::anyhow!("Invalid credentials")),
            ServiceError::UserAlreadyExists => AppError::Conflict(anyhow::anyhow!("User already exists")),
            ServiceError::EmailAlreadyRegistered => AppError::Conflict(anyhow::anyhow!("Email already registered")),
            ServiceError::InvalidToken => AppError::BadRequest(anyhow::anyhow!("Invalid token")),
            ServiceError::TokenExpired => AppError::BadRequest(anyhow::anyhow!("Token expired")),
            ServiceError::UserNotFound => AppError::NotFound(anyhow::anyhow!("User not found")),
            ServiceError::EmailError(e) => AppError::EmailError(e),
            ServiceError::ValidationError(e) => AppError::BadRequest(anyhow::anyhow!(e)),
        }
    }
}