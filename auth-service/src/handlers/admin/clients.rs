use service_core::{
    axum::{
        extract::{Path, State},
        http::StatusCode,
        Json,
    },
    error::AppError,
};

use crate::{
    dtos::admin::{CreateClientRequest, CreateClientResponse, RotateSecretResponse},
    utils::ValidatedJson,
    AppState,
};

/// Create a new client
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
) -> Result<(StatusCode, Json<CreateClientResponse>), AppError> {
    let res = state.admin_service.create_client(req).await.map_err(|e| {
        tracing::error!(error = %e, "Failed to create client");
        e
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
) -> Result<(StatusCode, Json<RotateSecretResponse>), AppError> {
    let res = state
        .admin_service
        .rotate_client_secret(client_id.clone())
        .await
        .map_err(|e| {
            tracing::error!(error = %e, client_id = %client_id, "Failed to rotate client secret");
            e
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
) -> Result<(StatusCode, Json<serde_json::Value>), AppError> {
    state
        .admin_service
        .revoke_client(client_id.clone())
        .await
        .map_err(|e| {
            tracing::error!(error = %e, client_id = %client_id, "Failed to revoke client");
            e
        })?;
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "message": "Client revoked successfully"
        })),
    ))
}
