use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};

use crate::{
    dtos::{admin::CreateServiceAccountRequest, ErrorResponse},
    utils::ValidatedJson,
    AppState,
};

/// Create a new service account
#[utoipa::path(
    post,
    path = "/auth/admin/services",
    request_body = CreateServiceAccountRequest,
    responses(
        (status = 201, description = "Service account created successfully", body = CreateServiceAccountResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 422, description = "Validation error", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Admin",
    security(
        ("admin_api_key" = [])
    )
)]
pub async fn create_service_account(
    State(state): State<AppState>,
    ValidatedJson(req): ValidatedJson<CreateServiceAccountRequest>,
) -> Result<impl IntoResponse, Response> {
    let res = state
        .admin_service
        .create_service_account(req, &state.config.environment)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: e.to_string(),
                }),
            )
                .into_response()
        })?;

    Ok((StatusCode::CREATED, Json(res)))
}

/// Rotate service account API key
#[utoipa::path(
    post,
    path = "/auth/admin/services/{service_id}/rotate",
    params(
        ("service_id" = String, Path, description = "Service ID to rotate")
    ),
    responses(
        (status = 200, description = "Key rotated successfully", body = RotateServiceKeyResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 404, description = "Service account not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Admin",
    security(
        ("admin_api_key" = [])
    )
)]
pub async fn rotate_service_key(
    State(state): State<AppState>,
    Path(service_id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let res = state
        .admin_service
        .rotate_service_key(service_id, &state.config.environment)
        .await
        .map_err(|e| {
            let status = match &e {
                crate::services::ServiceError::UserNotFound => StatusCode::NOT_FOUND,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            };
            (
                status,
                Json(ErrorResponse {
                    error: e.to_string(),
                }),
            )
        })?;

    Ok((StatusCode::OK, Json(res)))
}

/// Revoke a service account
#[utoipa::path(
    delete,
    path = "/auth/admin/services/{service_id}",
    params(
        ("service_id" = String, Path, description = "Service ID to revoke")
    ),
    responses(
        (status = 200, description = "Service account revoked successfully"),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 404, description = "Service account not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Admin",
    security(
        ("admin_api_key" = [])
    )
)]
pub async fn revoke_service_account(
    State(state): State<AppState>,
    Path(service_id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    state
        .admin_service
        .revoke_service_account(service_id)
        .await
        .map_err(|e| {
            let status = match &e {
                crate::services::ServiceError::UserNotFound => StatusCode::NOT_FOUND,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            };
            (
                status,
                Json(ErrorResponse {
                    error: e.to_string(),
                }),
            )
        })?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "message": "Service account revoked successfully"
        })),
    ))
}

/// Get audit logs for a service account
#[utoipa::path(
    get,
    path = "/auth/admin/services/{service_id}/audit-log",
    params(
        ("service_id" = String, Path, description = "Service ID to fetch logs for")
    ),
    responses(
        (status = 200, description = "Audit logs returned", body = [AuditLog]),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Admin",
    security(
        ("admin_api_key" = [])
    )
)]
pub async fn get_service_audit_log(
    State(state): State<AppState>,
    Path(service_id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let res = state
        .admin_service
        .get_service_audit_log(service_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: e.to_string(),
                }),
            )
        })?;

    Ok((StatusCode::OK, Json(res)))
}
