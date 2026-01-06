use crate::models::user::UserProfile;
use crate::services::auth_client::AuthClient;
use askama::Template;
use axum::{
    extract::State,
    response::{IntoResponse, Redirect},
};
use std::sync::Arc;
use tower_sessions::Session;

#[derive(Template)]
#[template(path = "dashboard.html")]
pub struct DashboardTemplate {
    pub user: UserProfile,
}

pub async fn dashboard_handler(
    State(auth_client): State<Arc<AuthClient>>,
    session: Session,
) -> impl IntoResponse {
    let access_token: String = session
        .get("access_token")
        .await
        .unwrap_or_default()
        .unwrap_or_default();

    if access_token.is_empty() {
        return Redirect::to("/login").into_response();
    }

    let response = auth_client.get_with_auth("/users/me", &access_token).await;

    match response {
        Ok(res) if res.status().is_success() => {
            let user: UserProfile = res.json().await.unwrap_or(UserProfile {
                email: "unknown".to_string(),
                verified: false,
            });
            let template = DashboardTemplate { user };
            template.into_response()
        }
        _ => Redirect::to("/login").into_response(),
    }
}
