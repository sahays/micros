use crate::models::user::{AuthUser, UserProfile};
use crate::AppState;
use askama::Template;
use axum::{extract::State, response::IntoResponse};

#[derive(Template)]
#[template(path = "dashboard.html")]
pub struct DashboardTemplate {
    pub user: UserProfile,
    pub current_page: &'static str,
}

pub async fn dashboard_handler(
    State(state): State<AppState>,
    auth_user: AuthUser,
) -> impl IntoResponse {
    let response = state
        .auth_client
        .get_with_auth("/users/me", &auth_user.access_token)
        .await;

    match response {
        Ok(res) if res.status().is_success() => {
            let user: UserProfile = res.json().await.unwrap_or(UserProfile {
                email: auth_user.email,
                verified: false,
            });
            let template = DashboardTemplate {
                user,
                current_page: "dashboard",
            };
            template.into_response()
        }
        _ => {
            // If fetching profile fails, we can still show a basic dashboard with session data
            let user = UserProfile {
                email: auth_user.email,
                verified: false,
            };
            let template = DashboardTemplate {
                user,
                current_page: "dashboard",
            };
            template.into_response()
        }
    }
}
