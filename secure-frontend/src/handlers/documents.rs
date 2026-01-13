use crate::models::document::DocumentListResponse;
use crate::models::user::{AuthUser, UserProfile};
use crate::services::auth_client::AuthClient;
use askama::Template;
use axum::{
    extract::{Query, State},
    response::IntoResponse,
};
use serde::Deserialize;
use std::sync::Arc;

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
    State(auth_client): State<Arc<AuthClient>>,
    auth_user: AuthUser,
    Query(params): Query<ListParams>,
) -> impl IntoResponse {
    // 1. Fetch User Profile (reusing pattern)
    let user_res = auth_client
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

    // 2. Fetch Documents from document-service
    // We'll use reqwest directly since AuthClient is specific to auth-service
    // TODO: Abstract this into a ServiceClient or extend AuthClient
    let doc_service_url = "http://document-service:8002"; // Default
    let client = reqwest::Client::new();

    let mut url = format!(
        "{}/documents?page={}&page_size=100",
        doc_service_url,
        params.page.unwrap_or(1)
    );
    if let Some(status) = &params.status {
        url.push_str(&format!("&status={}", status));
    }

    let docs_res = client
        .get(&url)
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
            serde_json::to_string(&list.documents).unwrap_or("[]".to_string())
        }
        _ => "[]".to_string(),
    };

    DocumentsTemplate {
        user,
        current_page: "documents",
        documents_json,
    }
}
