use crate::models::user::{AuthUser, UserProfile};
use crate::AppState;
use askama::Template;
use axum::{
    extract::{Query, State},
    response::IntoResponse,
};
use serde::Deserialize;

#[derive(Template)]
#[template(path = "pages/documents.html")]
pub struct DocumentsTemplate {
    pub user: UserProfile,
    pub current_page: &'static str,
    pub documents_json: String, // Pass as JSON string for Alpine.js
}

#[derive(Deserialize)]
pub struct ListParams {
    pub page: Option<u64>,
    pub status: Option<String>,
}

pub async fn list_documents_page(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Query(params): Query<ListParams>,
) -> impl IntoResponse {
    // Use user info from session (extracted during login)
    let user = UserProfile {
        email: auth_user.email.clone(),
        verified: true, // Users who can access this page are logged in
    };

    // Fetch documents from document-service using gRPC
    let page = params.page.map(|p| p as i32);

    let documents_json = match state
        .document_client
        .list_documents(&auth_user.user_id, page, Some(100))
        .await
    {
        Ok(list_response) => {
            // Convert to JSON for Alpine.js template
            serde_json::to_string(&list_response.documents).unwrap_or_else(|e| {
                tracing::error!("Failed to serialize documents: {}", e);
                "[]".to_string()
            })
        }
        Err(e) => {
            tracing::error!(
                user_id = %auth_user.user_id,
                error = %e,
                "Failed to fetch documents via gRPC"
            );
            "[]".to_string()
        }
    };

    DocumentsTemplate {
        user,
        current_page: "documents",
        documents_json,
    }
}
