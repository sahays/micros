use crate::models::user::AuthUser;
use crate::AppState;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use chrono::Utc;
use secrecy::ExposeSecret;
use serde::{Deserialize, Serialize};
use service_core::utils::signature::generate_document_signature;

/// Query parameters for generating signed URLs
#[derive(Debug, Deserialize)]
pub struct SignedUrlParams {
    /// Time-to-live in seconds (default: 3600 = 1 hour)
    pub ttl: Option<i64>,
}

/// Response containing a signed shareable URL
#[derive(Debug, Serialize)]
pub struct SignedUrlResponse {
    pub url: String,
    pub expires_at: i64,
    pub expires_in_seconds: i64,
}

/// Download a document by proxying to document-service
///
/// This endpoint proxies the authenticated user's request to document-service
/// with proper HMAC authentication. The document-service validates ownership.
pub async fn download_document(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(document_id): Path<String>,
) -> Result<Response, StatusCode> {
    tracing::info!(
        user_id = %auth_user.user_id,
        document_id = %document_id,
        "Document download request"
    );

    // Use DocumentClient to download with HMAC authentication
    let response = state
        .document_client
        .download_document(&auth_user.user_id, &document_id)
        .await
        .map_err(|e| {
            tracing::error!(
                user_id = %auth_user.user_id,
                document_id = %document_id,
                error = %e,
                "Failed to download document"
            );
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Check if document-service returned success
    if !response.status().is_success() {
        let status = response.status();
        tracing::warn!(
            user_id = %auth_user.user_id,
            document_id = %document_id,
            status = %status,
            "Document download failed"
        );
        return Err(status);
    }

    // Get content from document-service
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/octet-stream")
        .to_string();

    let content_disposition = response
        .headers()
        .get("content-disposition")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("attachment")
        .to_string();

    let file_data = response.bytes().await.map_err(|e| {
        tracing::error!(
            user_id = %auth_user.user_id,
            document_id = %document_id,
            error = %e,
            "Failed to read document bytes"
        );
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    tracing::info!(
        user_id = %auth_user.user_id,
        document_id = %document_id,
        size = file_data.len(),
        "Document download completed"
    );

    // Return file to user
    Ok((
        StatusCode::OK,
        [
            ("content-type", content_type),
            ("content-disposition", content_disposition),
        ],
        file_data,
    )
        .into_response())
}

/// Generate a signed shareable URL for a document
///
/// Creates a time-limited signed URL that can be shared with others.
/// The URL bypasses authentication but expires after the specified TTL.
///
/// **Security:** The signed URL uses HMAC-SHA256 signature with document ID and expiration.
/// Document-service validates the signature before serving the file.
pub async fn generate_signed_url(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(document_id): Path<String>,
    Query(params): Query<SignedUrlParams>,
) -> Result<impl IntoResponse, StatusCode> {
    let ttl_seconds = params.ttl.unwrap_or(3600).clamp(60, 86400); // 1 min to 24 hours
    let expires = Utc::now().timestamp() + ttl_seconds;

    tracing::info!(
        user_id = %auth_user.user_id,
        document_id = %document_id,
        ttl_seconds = ttl_seconds,
        expires = expires,
        "Generating signed URL"
    );

    // First, verify the user owns this document by fetching its status
    let status_response = state
        .document_client
        .get_document(&auth_user.user_id, &document_id)
        .await
        .map_err(|e| {
            tracing::error!(
                user_id = %auth_user.user_id,
                document_id = %document_id,
                error = %e,
                "Failed to verify document ownership"
            );
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    if !status_response.status().is_success() {
        tracing::warn!(
            user_id = %auth_user.user_id,
            document_id = %document_id,
            status = %status_response.status(),
            "Document not found or access denied"
        );
        return Err(status_response.status());
    }

    // Generate signature using service-core utilities
    let signature = generate_document_signature(
        &document_id,
        expires,
        state
            .document_client
            .settings
            .signing_secret
            .expose_secret(),
    )
    .map_err(|e| {
        tracing::error!(
            error = %e,
            "Failed to generate document signature"
        );
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Construct signed URL pointing directly to document-service
    // This allows the URL to work even if the BFF is down
    let signed_url = format!(
        "{}/documents/{}/content?signature={}&expires={}",
        state.document_client.settings.url, document_id, signature, expires
    );

    tracing::info!(
        user_id = %auth_user.user_id,
        document_id = %document_id,
        expires = expires,
        "Signed URL generated successfully"
    );

    Ok(Json(SignedUrlResponse {
        url: signed_url,
        expires_at: expires,
        expires_in_seconds: ttl_seconds,
    }))
}

/// Download via signed URL (direct access without authentication)
///
/// This endpoint validates the signature and proxies to document-service.
/// Used for shareable links that don't require login.
pub async fn download_with_signature(
    State(state): State<AppState>,
    Path(document_id): Path<String>,
    Query(params): Query<SignedUrlQuery>,
) -> Result<Response, StatusCode> {
    tracing::info!(
        document_id = %document_id,
        expires = params.expires,
        "Signed URL download request"
    );

    // Validate signature using service-core utilities
    service_core::utils::signature::validate_document_signature(
        &document_id,
        &params.signature,
        params.expires,
        state
            .document_client
            .settings
            .signing_secret
            .expose_secret(),
    )
    .map_err(|e| {
        tracing::warn!(
            document_id = %document_id,
            error = %e,
            "Signed URL validation failed"
        );
        StatusCode::UNAUTHORIZED
    })?;

    // Forward to document-service with signature
    let url = format!(
        "{}/documents/{}/content?signature={}&expires={}",
        state.document_client.settings.url, document_id, params.signature, params.expires
    );

    let response = reqwest::Client::new().get(&url).send().await.map_err(|e| {
        tracing::error!(
            document_id = %document_id,
            error = %e,
            "Failed to fetch document from document-service"
        );
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    if !response.status().is_success() {
        tracing::warn!(
            document_id = %document_id,
            status = %response.status(),
            "Document-service returned error"
        );
        return Err(response.status());
    }

    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/octet-stream")
        .to_string();

    let content_disposition = response
        .headers()
        .get("content-disposition")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("attachment")
        .to_string();

    let file_data = response.bytes().await.map_err(|e| {
        tracing::error!(
            document_id = %document_id,
            error = %e,
            "Failed to read document bytes"
        );
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    tracing::info!(
        document_id = %document_id,
        size = file_data.len(),
        "Signed URL download completed"
    );

    Ok((
        StatusCode::OK,
        [
            ("content-type", content_type),
            ("content-disposition", content_disposition),
        ],
        file_data,
    )
        .into_response())
}

#[derive(Debug, Deserialize)]
pub struct SignedUrlQuery {
    pub signature: String,
    pub expires: i64,
}
