use crate::models::user::{AuthUser, UserProfile};
use crate::services::auth_client::AuthClient;
use askama::Template;
use axum::{
    extract::{Multipart, State},
    response::{IntoResponse, Json},
};
use reqwest::multipart;
use serde_json::json;
use std::sync::Arc;

#[derive(Template)]
#[template(path = "pages/upload.html")]
pub struct UploadTemplate {
    pub user: UserProfile,
    pub current_page: &'static str,
}

pub async fn upload_page(
    State(auth_client): State<Arc<AuthClient>>,
    auth_user: AuthUser,
) -> impl IntoResponse {
    // Re-use dashboard logic to get user profile if needed, or just use auth_user
    let response = auth_client
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

pub async fn upload_handler(auth_user: AuthUser, mut multipart: Multipart) -> impl IntoResponse {
    // TODO: Get document-service URL from config
    let doc_service_url = "http://document-service:8002"; // Default for dev

    while let Ok(Some(field)) = multipart.next_field().await {
        let file_name = field.file_name().unwrap_or("unnamed").to_string();
        let content_type = field
            .content_type()
            .unwrap_or("application/octet-stream")
            .to_string();

        let data = match field.bytes().await {
            Ok(bytes) => bytes,
            Err(e) => {
                return Json(json!({ "error": format!("Failed to read file: {}", e) }))
                    .into_response()
            }
        };

        // Create multipart form for backend service
        let form = multipart::Form::new().part(
            "file",
            multipart::Part::bytes(data.to_vec())
                .file_name(file_name)
                .mime_str(&content_type)
                .unwrap(),
        );

        // Forward to document-service
        let client = reqwest::Client::new();
        let res = client
            .post(format!("{}/documents", doc_service_url))
            .header("X-User-ID", &auth_user.user_id)
            .multipart(form)
            .send()
            .await;

        match res {
            Ok(response) => {
                if !response.status().is_success() {
                    let status = response.status();
                    let text = response.text().await.unwrap_or_default();
                    tracing::error!("Upload failed: {} - {}", status, text);
                    return Json(json!({ "error": "Upload failed" })).into_response();
                }
            }
            Err(e) => {
                tracing::error!("Failed to connect to document-service: {}", e);
                return Json(json!({ "error": "Service unavailable" })).into_response();
            }
        }
    }

    Json(json!({ "status": "success" })).into_response()
}
