use crate::dtos::ErrorResponse;
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
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

impl IntoResponse for ServiceError {
    fn into_response(self) -> Response {
        let (status, error_message) = match &self {
            ServiceError::Database(_)
            | ServiceError::Redis(_)
            | ServiceError::Internal(_)
            | ServiceError::InternalString(_)
            | ServiceError::EmailError(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal server error".to_string(),
            ),
            ServiceError::InvalidCredentials => (StatusCode::UNAUTHORIZED, self.to_string()),
            ServiceError::UserAlreadyExists | ServiceError::EmailAlreadyRegistered => {
                (StatusCode::CONFLICT, self.to_string())
            }
            ServiceError::InvalidToken
            | ServiceError::TokenExpired
            | ServiceError::ValidationError(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            ServiceError::UserNotFound => (StatusCode::NOT_FOUND, self.to_string()),
        };

        if status == StatusCode::INTERNAL_SERVER_ERROR {
            tracing::error!(error = %self, "Service error");
        }

        (
            status,
            Json(ErrorResponse {
                error: error_message,
            }),
        )
            .into_response()
    }
}
