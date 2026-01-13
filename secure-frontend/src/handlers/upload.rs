use crate::models::user::{AuthUser, UserProfile};
use crate::AppState;
use askama::Template;
use axum::{
    extract::{Multipart, State},
    response::{IntoResponse, Json},
};
use serde_json::json;

#[derive(Template)]
#[template(path = "pages/upload.html")]
pub struct UploadTemplate {
    pub user: UserProfile,
    pub current_page: &'static str,
}

pub async fn upload_page(State(state): State<AppState>, auth_user: AuthUser) -> impl IntoResponse {
    let response = state
        .auth_client
        .get_with_auth("/users/me", &auth_user.access_token)
        .await;

    let user = match response {
        Ok(res) if res.status().is_success() => res.json().await.unwrap_or(UserProfile {
            email: auth_user.email.clone(),
            verified: false,
        }),
        _ => UserProfile {
            email: auth_user.email.clone(),
            verified: false,
        },
    };

    UploadTemplate {
        user,
        current_page: "upload",
    }
}

pub async fn upload_handler(
    State(state): State<AppState>,
    auth_user: AuthUser,
    mut multipart: Multipart,
) -> impl IntoResponse {
    let mut uploaded_count = 0;
    let mut errors = Vec::new();

    // Process ALL files in the multipart form (fixes multi-file upload bug)
    while let Ok(Some(field)) = multipart.next_field().await {
        let file_name = field.file_name().unwrap_or("unnamed").to_string();
        let content_type = field
            .content_type()
            .unwrap_or("application/octet-stream")
            .to_string();

        // Read file data
        let data = match field.bytes().await {
            Ok(bytes) => bytes.to_vec(),
            Err(e) => {
                tracing::error!("Failed to read file {}: {}", file_name, e);
                errors.push(format!("{}: Failed to read file", file_name));
                continue; // Continue with next file instead of returning
            }
        };

        // Upload using DocumentClient with HMAC authentication
        match state
            .document_client
            .upload(&auth_user.user_id, &file_name, &content_type, data)
            .await
        {
            Ok(response) => {
                if response.status().is_success() {
                    uploaded_count += 1;
                    tracing::info!(
                        user_id = %auth_user.user_id,
                        file_name = %file_name,
                        "File uploaded successfully"
                    );
                } else {
                    let status = response.status();
                    let error_text = response
                        .text()
                        .await
                        .unwrap_or_else(|_| "Unknown error".to_string());
                    tracing::error!(
                        "Upload failed for {}: {} - {}",
                        file_name,
                        status,
                        error_text
                    );
                    errors.push(format!("{}: Upload failed ({})", file_name, status));
                }
            }
            Err(e) => {
                tracing::error!("Failed to upload {}: {}", file_name, e);
                errors.push(format!("{}: {}", file_name, e));
            }
        }
    }

    // Return comprehensive response with success count and any errors
    if errors.is_empty() {
        Json(json!({
            "status": "success",
            "uploaded": uploaded_count,
            "message": format!("{} file(s) uploaded successfully", uploaded_count)
        }))
        .into_response()
    } else if uploaded_count > 0 {
        // Partial success
        Json(json!({
            "status": "partial",
            "uploaded": uploaded_count,
            "errors": errors,
            "message": format!("{} file(s) uploaded, {} failed", uploaded_count, errors.len())
        }))
        .into_response()
    } else {
        // Complete failure
        Json(json!({
            "status": "error",
            "uploaded": 0,
            "errors": errors,
            "message": "All uploads failed"
        }))
        .into_response()
    }
}
