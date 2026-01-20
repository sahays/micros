use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Validation error: {0}")]
    ValidationError(#[from] validator::ValidationErrors),

    #[error("Bad request: {0}")]
    BadRequest(anyhow::Error),

    #[error("Not found: {0}")]
    NotFound(anyhow::Error),

    #[error("Unauthorized: {0}")]
    Unauthorized(anyhow::Error),

    #[error("Forbidden: {0}")]
    Forbidden(anyhow::Error),

    #[error("Authentication error: {0}")]
    AuthError(anyhow::Error),

    #[error("Conflict: {0}")]
    Conflict(anyhow::Error),

    #[error("Too many requests: {0}")]
    TooManyRequests(String, Option<u64>),

    #[error("Internal server error: {0}")]
    InternalError(#[from] anyhow::Error),

    #[error("Bad Gateway: {0}")]
    BadGateway(String),

    #[error("Service Unavailable")]
    ServiceUnavailable,

    #[error("Database error: {0}")]
    DatabaseError(anyhow::Error),

    #[error("Redis error: {0}")]
    RedisError(#[from] redis::RedisError),

    #[error("Invalid token: {0}")]
    InvalidToken(#[from] jsonwebtoken::errors::Error),

    #[error("Email error: {0}")]
    EmailError(String),

    #[error("Configuration error: {0}")]
    ConfigError(anyhow::Error),
}

impl From<config::ConfigError> for AppError {
    fn from(err: config::ConfigError) -> Self {
        AppError::ConfigError(anyhow::Error::new(err))
    }
}

impl From<lettre::error::Error> for AppError {
    fn from(err: lettre::error::Error) -> Self {
        AppError::EmailError(err.to_string())
    }
}

impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        AppError::InternalError(anyhow::Error::new(err))
    }
}

impl From<mongodb::error::Error> for AppError {
    fn from(err: mongodb::error::Error) -> Self {
        AppError::DatabaseError(anyhow::Error::new(err))
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        #[derive(Serialize)]
        struct ErrorResponse {
            error: String,
            #[serde(skip_serializing_if = "Option::is_none")]
            details: Option<String>,
        }

        let (status, error_message, details, retry_after) = match self {
            AppError::ValidationError(err) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "Validation error".to_string(),
                Some(err.to_string()),
                None,
            ),
            AppError::BadRequest(err) => (StatusCode::BAD_REQUEST, err.to_string(), None, None),
            AppError::NotFound(err) => (StatusCode::NOT_FOUND, err.to_string(), None, None),
            AppError::Unauthorized(err) => (StatusCode::UNAUTHORIZED, err.to_string(), None, None),
            AppError::Forbidden(err) => (StatusCode::FORBIDDEN, err.to_string(), None, None),
            AppError::AuthError(err) => (StatusCode::UNAUTHORIZED, err.to_string(), None, None),
            AppError::Conflict(err) => (StatusCode::CONFLICT, err.to_string(), None, None),
            AppError::TooManyRequests(msg, retry) => {
                (StatusCode::TOO_MANY_REQUESTS, msg, None, retry)
            }
            AppError::InternalError(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal server error".to_string(),
                Some(format!("{:#?}", err)),
                None,
            ),
            AppError::BadGateway(msg) => (
                StatusCode::BAD_GATEWAY,
                format!("Bad Gateway: {}", msg),
                None,
                None,
            ),
            AppError::ServiceUnavailable => (
                StatusCode::SERVICE_UNAVAILABLE,
                "Service unavailable".to_string(),
                None,
                None,
            ),
            AppError::DatabaseError(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Database error".to_string(),
                Some(err.to_string()),
                None,
            ),
            AppError::RedisError(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Cache error".to_string(),
                Some(err.to_string()),
                None,
            ),
            AppError::InvalidToken(err) => (
                StatusCode::UNAUTHORIZED,
                "Invalid token".to_string(),
                Some(err.to_string()),
                None,
            ),
            AppError::EmailError(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Email error".to_string(),
                Some(msg),
                None,
            ),
            AppError::ConfigError(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Configuration error".to_string(),
                Some(err.to_string()),
                None,
            ),
        };

        let mut res = (
            status,
            Json(ErrorResponse {
                error: error_message,
                details,
            }),
        )
            .into_response();

        if let Some(retry) = retry_after {
            res.headers_mut()
                .insert(axum::http::header::RETRY_AFTER, retry.into());
        }

        res
    }
}
