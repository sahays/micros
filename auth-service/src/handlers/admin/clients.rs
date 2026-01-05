use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};

use crate::{
    dtos::{admin::CreateClientRequest, ErrorResponse},
    utils::ValidatedJson,
    AppState,
};

/// Create a new OAuth client
#[utoipa::path(
    post,
    path = "/auth/admin/clients",
    request_body = CreateClientRequest,
    responses(
        (status = 201, description = "Client created successfully", body = CreateClientResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 422, description = "Validation error", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Admin",
    security(
        ("admin_api_key" = [])
    )
)]
pub async fn create_client(
    State(state): State<AppState>,
    ValidatedJson(req): ValidatedJson<CreateClientRequest>,
) -> Result<impl IntoResponse, Response> {
    let res = state.admin_service.create_client(req).await.map_err(|e| {
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

/// Rotate client secrets
#[utoipa::path(
    post,
    path = "/auth/admin/clients/{client_id}/rotate",
    params(
        ("client_id" = String, Path, description = "Client ID to rotate")
    ),
    responses(
        (status = 200, description = "Secret rotated successfully", body = RotateSecretResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 404, description = "Client not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Admin",
    security(
        ("admin_api_key" = [])
    )
)]
pub async fn rotate_client_secret(
    State(state): State<AppState>,
    Path(client_id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let res = state
        .admin_service
        .rotate_client_secret(client_id)
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

/// Revoke a client
#[utoipa::path(
    delete,
    path = "/auth/admin/clients/{client_id}",
    params(
        ("client_id" = String, Path, description = "Client ID to revoke")
    ),
    responses(
        (status = 200, description = "Client revoked successfully"),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 404, description = "Client not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Admin",
    security(
        ("admin_api_key" = [])
    )
)]
pub async fn revoke_client(
    State(state): State<AppState>,
    Path(client_id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    state
        .admin_service
        .revoke_client(client_id)
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
            "message": "Client revoked successfully"
        })),
    ))
}
