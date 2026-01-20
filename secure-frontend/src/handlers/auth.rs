use crate::utils::jwt::decode_jwt_claims;
use crate::AppState;
use askama::Template;
use axum::{
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    response::{Html, IntoResponse, Redirect},
    Form,
};
use serde::Deserialize;
use tower_sessions::Session;

#[derive(Template)]
#[template(path = "login.html")]
pub struct LoginTemplate {}

#[derive(Template)]
#[template(path = "register.html")]
pub struct RegisterTemplate {}

#[derive(Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
}

pub async fn login_page() -> impl IntoResponse {
    LoginTemplate {}
}

pub async fn register_page() -> impl IntoResponse {
    RegisterTemplate {}
}

pub async fn login_handler(
    State(state): State<AppState>,
    session: Session,
    Form(payload): Form<LoginRequest>,
) -> impl IntoResponse {
    // Use gRPC client for login
    let result = state
        .auth_client
        .login(&payload.email, &payload.password)
        .await;

    match result {
        Ok(login_result) => {
            // Extract user_id and email from JWT for session storage
            match decode_jwt_claims(&login_result.access_token) {
                Ok(claims) => {
                    // Store tokens and user context in session
                    session
                        .insert("access_token", &login_result.access_token)
                        .await
                        .unwrap();
                    session
                        .insert("refresh_token", &login_result.refresh_token)
                        .await
                        .unwrap();

                    // Store user_id and email for context propagation to other services
                    session.insert("user_id", &claims.sub).await.unwrap();
                    session.insert("email", &claims.email).await.unwrap();

                    tracing::info!(
                        user_id = %claims.sub,
                        email = %claims.email,
                        "User logged in successfully via gRPC"
                    );

                    // HTMX Redirect to dashboard
                    let mut headers = HeaderMap::new();
                    headers.insert("HX-Redirect", "/dashboard".parse().unwrap());
                    (StatusCode::OK, headers, "").into_response()
                }
                Err(e) => {
                    tracing::error!("Failed to decode JWT claims: {}", e);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Html("<p class='text-red-500 text-sm'>Authentication error</p>"),
                    )
                        .into_response()
                }
            }
        }
        Err(e) => {
            tracing::warn!(email = %payload.email, error = %e, "Login failed");
            // Return error fragment for HTMX
            (
                StatusCode::UNPROCESSABLE_ENTITY,
                Html("<p class='text-red-500 text-sm'>Invalid email or password</p>"),
            )
                .into_response()
        }
    }
}

pub async fn register_handler(
    State(state): State<AppState>,
    Form(payload): Form<RegisterRequest>,
) -> impl IntoResponse {
    // Use gRPC client for registration
    let result = state
        .auth_client
        .register(&payload.email, &payload.password, None)
        .await;

    match result {
        Ok(_) => {
            (StatusCode::OK, Html("<p class='text-emerald-500 text-sm'>Registration successful! Please check your email.</p>")).into_response()
        }
        Err(e) => {
            let error_msg = e.to_string();
            tracing::warn!(email = %payload.email, error = %error_msg, "Registration failed");

            // Try to extract useful error message
            let display_msg = if error_msg.contains("already exists") || error_msg.contains("already in use") {
                "Email is already registered. Try logging in instead."
            } else if error_msg.contains("password") {
                "Password must be at least 8 characters."
            } else if error_msg.contains("email") || error_msg.contains("invalid") {
                "Please enter a valid email address."
            } else {
                "Registration failed. Please try again."
            };

            (StatusCode::UNPROCESSABLE_ENTITY, Html(format!("<p class='text-red-500 text-sm'>{}</p>", display_msg))).into_response()
        }
    }
}

pub async fn logout_handler(State(state): State<AppState>, session: Session) -> impl IntoResponse {
    // Get refresh token before clearing session (gRPC logout uses refresh_token)
    if let Some(refresh_token) = session.get::<String>("refresh_token").await.unwrap_or(None) {
        // Attempt to revoke token via gRPC
        // We don't fail the logout if this fails - just log the error
        if let Err(e) = state.auth_client.logout(&refresh_token).await {
            tracing::error!("Failed to revoke token during logout: {}", e);
        } else {
            tracing::info!("Token revoked successfully via gRPC");
        }
    }

    // Clear session regardless of token revocation result
    session.clear().await;

    // HTMX redirect to home page
    let mut headers = HeaderMap::new();
    headers.insert("HX-Redirect", "/".parse().unwrap());
    (StatusCode::OK, headers, "").into_response()
}

// Google OAuth handlers

#[derive(Deserialize)]
pub struct OAuthCallbackParams {
    pub code: String,
    pub state: Option<String>,
}

/// Initiates Google OAuth flow by redirecting to auth-service
pub async fn google_oauth_redirect(State(state): State<AppState>) -> impl IntoResponse {
    // Generate PKCE code_verifier and code_challenge
    // In production, store code_verifier in session
    // For now, auth-service handles the OAuth flow completely

    // Use public_url for browser redirect (not internal Docker service name)
    let auth_url = format!(
        "{}/auth/social/google/login",
        state.auth_client.public_url()
    );

    tracing::info!(
        "Redirecting to Google OAuth via auth-service at {}",
        auth_url
    );

    Redirect::to(&auth_url)
}

/// Processes OAuth callback from Google
pub async fn google_oauth_callback(
    State(state): State<AppState>,
    Query(params): Query<OAuthCallbackParams>,
    session: Session,
) -> impl IntoResponse {
    tracing::info!("Processing Google OAuth callback");

    // Exchange authorization code for tokens via HTTP (OAuth requires HTTP for browser flows)
    let result = state
        .auth_client
        .oauth_callback(&params.code, params.state.as_deref())
        .await;

    match result {
        Ok(oauth_result) => {
            // Extract user_id and email from JWT for session storage
            match decode_jwt_claims(&oauth_result.access_token) {
                Ok(claims) => {
                    // Store tokens and user context in session
                    session
                        .insert("access_token", &oauth_result.access_token)
                        .await
                        .unwrap();
                    session
                        .insert("refresh_token", &oauth_result.refresh_token)
                        .await
                        .unwrap();

                    // Store user context
                    session.insert("user_id", &claims.sub).await.unwrap();
                    session.insert("email", &claims.email).await.unwrap();

                    // Store name and picture if available from OAuth
                    if let Some(name) = &oauth_result.name {
                        session.insert("name", name).await.unwrap();
                    }
                    if let Some(picture) = &oauth_result.picture {
                        session.insert("picture", picture).await.unwrap();
                    }

                    tracing::info!(
                        user_id = %claims.sub,
                        email = %claims.email,
                        "User logged in via Google OAuth successfully"
                    );

                    // Redirect to dashboard
                    Redirect::to("/dashboard").into_response()
                }
                Err(e) => {
                    tracing::error!("Failed to decode JWT claims from OAuth: {}", e);
                    Redirect::to("/login?error=auth_failed").into_response()
                }
            }
        }
        Err(e) => {
            tracing::error!("OAuth callback failed: {}", e);
            Redirect::to("/login?error=oauth_failed").into_response()
        }
    }
}
