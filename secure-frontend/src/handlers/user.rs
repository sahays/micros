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

pub async fn dashboard_handler(_state: State<AppState>, auth_user: AuthUser) -> impl IntoResponse {
    // Use user info from session (extracted during login)
    // This avoids an extra gRPC call since we already have the user data
    let user = UserProfile {
        email: auth_user.email,
        verified: true, // Users who can access this page are logged in
    };

    DashboardTemplate {
        user,
        current_page: "dashboard",
    }
}
