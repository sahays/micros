use axum::{
    async_trait,
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
    response::{IntoResponse, Redirect, Response},
};
use serde::{Deserialize, Serialize};
use tower_sessions::Session;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserProfile {
    pub email: String,
    pub verified: bool,
}

impl UserProfile {
    pub fn name(&self) -> String {
        self.email.split('@').next().unwrap_or("User").to_string()
    }

    pub fn initials(&self) -> String {
        let name = self.name();
        if name.len() >= 2 {
            name[0..2].to_uppercase()
        } else if !name.is_empty() {
            name[0..1].to_uppercase()
        } else {
            "U".to_string()
        }
    }
}

/// Authenticated user context extracted from session
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub user_id: String,
    pub email: String,
    pub name: Option<String>,
    pub picture: Option<String>,
    pub access_token: String,
}

#[async_trait]
impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        // Extract session from request extensions
        let session = Session::from_request_parts(parts, state)
            .await
            .map_err(|_| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to extract session",
                )
                    .into_response()
            })?;

        // Get required fields from session
        let access_token: Option<String> = session.get("access_token").await.unwrap_or(None);
        let user_id: Option<String> = session.get("user_id").await.unwrap_or(None);
        let email: Option<String> = session.get("email").await.unwrap_or(None);

        // Check if user is authenticated
        match (access_token, user_id, email) {
            (Some(token), Some(uid), Some(email_val)) => {
                // Optional fields
                let name: Option<String> = session.get("name").await.unwrap_or(None);
                let picture: Option<String> = session.get("picture").await.unwrap_or(None);

                Ok(AuthUser {
                    user_id: uid,
                    email: email_val,
                    name,
                    picture,
                    access_token: token,
                })
            }
            _ => {
                // Redirect to login if not authenticated
                Err(Redirect::to("/login").into_response())
            }
        }
    }
}

impl AuthUser {
    /// Returns the user ID for propagating to backend services
    pub fn user_id_header(&self) -> (&str, &str) {
        ("X-User-ID", &self.user_id)
    }
}
