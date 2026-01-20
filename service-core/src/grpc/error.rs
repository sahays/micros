//! Error conversion utilities between `AppError` and `tonic::Status`.
//!
//! Provides bidirectional conversion following the mapping defined in the gRPC migration epic:
//!
//! | AppError | gRPC Status |
//! |----------|-------------|
//! | `ValidationError` | `INVALID_ARGUMENT` |
//! | `NotFound` | `NOT_FOUND` |
//! | `Unauthorized` | `UNAUTHENTICATED` |
//! | `Forbidden` | `PERMISSION_DENIED` |
//! | `Conflict` | `ALREADY_EXISTS` |
//! | `TooManyRequests` | `RESOURCE_EXHAUSTED` |
//! | `InternalError` | `INTERNAL` |
//! | `ServiceUnavailable` | `UNAVAILABLE` |

use tonic::{Code, Status};

use crate::error::AppError;

/// Extension trait for converting types into `tonic::Status`.
pub trait IntoStatus {
    /// Convert into a `tonic::Status`.
    fn into_status(self) -> Status;
}

impl IntoStatus for AppError {
    fn into_status(self) -> Status {
        match self {
            AppError::ValidationError(err) => {
                Status::invalid_argument(format!("Validation error: {}", err))
            }
            AppError::BadRequest(err) => Status::invalid_argument(err.to_string()),
            AppError::NotFound(err) => Status::not_found(err.to_string()),
            AppError::Unauthorized(err) => Status::unauthenticated(err.to_string()),
            AppError::Forbidden(err) => Status::permission_denied(err.to_string()),
            AppError::AuthError(err) => Status::unauthenticated(err.to_string()),
            AppError::Conflict(err) => Status::already_exists(err.to_string()),
            AppError::TooManyRequests(msg, retry_after) => {
                let mut status = Status::resource_exhausted(msg);
                if let Some(seconds) = retry_after {
                    // Add retry-after as metadata
                    if let Ok(value) = seconds.to_string().parse() {
                        status.metadata_mut().insert("retry-after", value);
                    }
                }
                status
            }
            AppError::InternalError(err) => {
                // Log the full error but don't expose it to clients
                tracing::error!(error = %err, "Internal error");
                Status::internal("Internal server error")
            }
            AppError::BadGateway(msg) => Status::unavailable(format!("Bad gateway: {}", msg)),
            AppError::ServiceUnavailable => Status::unavailable("Service unavailable"),
            AppError::DatabaseError(err) => {
                tracing::error!(error = %err, "Database error");
                Status::internal("Database error")
            }
            AppError::RedisError(err) => {
                tracing::error!(error = %err, "Redis error");
                Status::internal("Cache error")
            }
            AppError::InvalidToken(err) => {
                Status::unauthenticated(format!("Invalid token: {}", err))
            }
            AppError::EmailError(msg) => {
                tracing::error!(error = %msg, "Email error");
                Status::internal("Email service error")
            }
            AppError::ConfigError(err) => {
                tracing::error!(error = %err, "Configuration error");
                Status::internal("Configuration error")
            }
        }
    }
}

impl From<AppError> for Status {
    fn from(err: AppError) -> Self {
        err.into_status()
    }
}

/// Convert a `tonic::Status` back to an `AppError`.
/// This is useful when a gRPC client receives an error and needs to propagate it.
impl From<Status> for AppError {
    fn from(status: Status) -> Self {
        match status.code() {
            Code::InvalidArgument => AppError::BadRequest(anyhow::anyhow!("{}", status.message())),
            Code::NotFound => AppError::NotFound(anyhow::anyhow!("{}", status.message())),
            Code::Unauthenticated => {
                AppError::Unauthorized(anyhow::anyhow!("{}", status.message()))
            }
            Code::PermissionDenied => AppError::Forbidden(anyhow::anyhow!("{}", status.message())),
            Code::AlreadyExists => AppError::Conflict(anyhow::anyhow!("{}", status.message())),
            Code::ResourceExhausted => {
                let retry_after = status
                    .metadata()
                    .get("retry-after")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|s| s.parse().ok());
                AppError::TooManyRequests(status.message().to_string(), retry_after)
            }
            Code::Unavailable => AppError::ServiceUnavailable,
            Code::Internal | Code::Unknown | Code::DataLoss => {
                AppError::InternalError(anyhow::anyhow!("{}", status.message()))
            }
            Code::Aborted | Code::Cancelled | Code::DeadlineExceeded => {
                AppError::InternalError(anyhow::anyhow!("Request failed: {}", status.message()))
            }
            Code::FailedPrecondition | Code::OutOfRange => {
                AppError::BadRequest(anyhow::anyhow!("{}", status.message()))
            }
            Code::Unimplemented => {
                AppError::InternalError(anyhow::anyhow!("Not implemented: {}", status.message()))
            }
            Code::Ok => {
                // This shouldn't happen, but handle it gracefully
                AppError::InternalError(anyhow::anyhow!("Unexpected OK status as error"))
            }
        }
    }
}

/// Result type alias for gRPC handlers.
pub type GrpcResult<T> = Result<tonic::Response<T>, Status>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_error_to_status() {
        let err = AppError::NotFound(anyhow::anyhow!("User not found"));
        let status: Status = err.into();
        assert_eq!(status.code(), Code::NotFound);
        assert!(status.message().contains("User not found"));
    }

    #[test]
    fn test_status_to_app_error() {
        let status = Status::not_found("Resource not found");
        let err: AppError = status.into();
        match err {
            AppError::NotFound(e) => assert!(e.to_string().contains("Resource not found")),
            _ => panic!("Expected NotFound error"),
        }
    }

    #[test]
    fn test_rate_limit_error() {
        let err = AppError::TooManyRequests("Rate limit exceeded".to_string(), Some(60));
        let status: Status = err.into();
        assert_eq!(status.code(), Code::ResourceExhausted);
        assert_eq!(
            status
                .metadata()
                .get("retry-after")
                .unwrap()
                .to_str()
                .unwrap(),
            "60"
        );
    }
}
