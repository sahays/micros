use crate::models::document::DocumentListResponse;
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
    // 1. Fetch User Profile
    let user_res = state
        .auth_client
        .get_with_auth("/users/me", &auth_user.access_token)
        .await;

    let user = match user_res {
        Ok(res) if res.status().is_success() => res.json().await.unwrap_or(UserProfile {
            email: auth_user.email.clone(),
            verified: false,
        }),
        _ => UserProfile {
            email: auth_user.email.clone(),
            verified: false,
        },
    };

    // 2. Fetch Documents from document-service using DocumentClient (HMAC signed)
    // Build query path with parameters
    let mut query_path = format!("/documents?page={}&page_size=100", params.page.unwrap_or(1));
    if let Some(status) = &params.status {
        query_path.push_str(&format!("&status={}", status));
    }

    // Use DocumentClient.get_document to make authenticated request
    // Note: We'll use reqwest directly here since DocumentClient.get_document expects a single document ID
    // In production, consider adding a list_documents() method to DocumentClient
    let doc_service_url = &state.document_client.settings.url;
    let client = reqwest::Client::new();

    let docs_res = client
        .get(format!("{}{}", doc_service_url, query_path))
        .header("X-User-ID", &auth_user.user_id)
        .send()
        .await;

    let documents_json = match docs_res {
        Ok(res) if res.status().is_success() => {
            let list: DocumentListResponse = res.json().await.unwrap_or(DocumentListResponse {
                documents: vec![],
                total: 0,
                page: 1,
                page_size: 20,
                total_pages: 0,
            });
            serde_json::to_string(&list.documents).unwrap_or_else(|e| {
                tracing::error!("Failed to serialize documents: {}", e);
                "[]".to_string()
            })
        }
        Ok(res) => {
            tracing::error!("Failed to fetch documents: {}", res.status());
            "[]".to_string()
        }
        Err(e) => {
            tracing::error!("Failed to connect to document-service: {}", e);
            "[]".to_string()
        }
    };

    DocumentsTemplate {
        user,
        current_page: "documents",
        documents_json,
    }
}
