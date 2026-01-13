use crate::services::auth_client::AuthClient;
use crate::utils::jwt::decode_jwt_claims;
use askama::Template;
use axum::{
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    response::{Html, IntoResponse, Redirect},
    Form,
};
use serde::Deserialize;
use std::sync::Arc;
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
    State(auth_client): State<Arc<AuthClient>>,
    session: Session,
    Form(payload): Form<LoginRequest>,
) -> impl IntoResponse {
    let response = auth_client
        .post(
            "/auth/login",
            serde_json::json!({
                "email": payload.email,
                "password": payload.password,
            }),
        )
        .await;

    match response {
        Ok(res) if res.status().is_success() => {
            let tokens: serde_json::Value = res.json().await.unwrap_or_default();

            let access_token = tokens["access_token"].as_str().unwrap_or_default();

            // Extract user_id and email from JWT for session storage and service propagation
            // We trust the token since it came from auth-service via HMAC-signed request
            match decode_jwt_claims(access_token) {
                Ok(claims) => {
                    // Store tokens and user context in session
                    session.insert("access_token", access_token).await.unwrap();
                    session
                        .insert(
                            "refresh_token",
                            tokens["refresh_token"].as_str().unwrap_or_default(),
                        )
                        .await
                        .unwrap();

                    // Store user_id and email for context propagation to other services
                    session.insert("user_id", &claims.sub).await.unwrap();
                    session.insert("email", &claims.email).await.unwrap();

                    tracing::info!(
                        user_id = %claims.sub,
                        email = %claims.email,
                        "User logged in successfully"
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
        _ => {
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
    State(auth_client): State<Arc<AuthClient>>,
    Form(payload): Form<RegisterRequest>,
) -> impl IntoResponse {
    let response = auth_client
        .post(
            "/auth/register",
            serde_json::json!({
                "email": payload.email,
                "password": payload.password,
            }),
        )
        .await;

    match response {
        Ok(res) if res.status().is_success() => {
            (StatusCode::OK, Html("<p class='text-emerald-500 text-sm'>Registration successful! Please check your email.</p>")).into_response()
        }
        _ => {
            (StatusCode::UNPROCESSABLE_ENTITY, Html("<p class='text-red-500 text-sm'>Registration failed. Email might already be in use.</p>")).into_response()
        }
    }
}

pub async fn logout_handler(
    State(auth_client): State<Arc<AuthClient>>,
    session: Session,
) -> impl IntoResponse {
    // Get access token before clearing session
    if let Some(access_token) = session.get::<String>("access_token").await.unwrap_or(None) {
        // Attempt to revoke token via auth-service
        // We don't fail the logout if this fails - just log the error
        if let Err(e) = auth_client
            .post(
                "/auth/logout",
                serde_json::json!({
                    "token": access_token
                }),
            )
            .await
        {
            tracing::error!("Failed to revoke token during logout: {}", e);
        } else {
            tracing::info!("Token revoked successfully");
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
pub async fn google_oauth_redirect(
    State(auth_client): State<Arc<AuthClient>>,
) -> impl IntoResponse {
    // Generate PKCE code_verifier and code_challenge
    // In production, store code_verifier in session
    // For now, auth-service handles the OAuth flow completely

    let auth_url = format!("{}/auth/social/google/login", auth_client.base_url());

    tracing::info!("Redirecting to Google OAuth via auth-service");

    Redirect::to(&auth_url)
}

/// Processes OAuth callback from Google
pub async fn google_oauth_callback(
    State(auth_client): State<Arc<AuthClient>>,
    Query(params): Query<OAuthCallbackParams>,
    session: Session,
) -> impl IntoResponse {
    tracing::info!("Processing Google OAuth callback");

    // Exchange authorization code for tokens via auth-service
    let response = auth_client
        .post(
            "/auth/social/google/callback",
            serde_json::json!({
                "code": params.code,
                "state": params.state,
            }),
        )
        .await;

    match response {
        Ok(res) if res.status().is_success() => {
            let tokens: serde_json::Value = res.json().await.unwrap_or_default();

            let access_token = tokens["access_token"].as_str().unwrap_or_default();

            // Extract user_id and email from JWT for session storage
            match decode_jwt_claims(access_token) {
                Ok(claims) => {
                    // Store tokens and user context in session
                    session.insert("access_token", access_token).await.unwrap();
                    session
                        .insert(
                            "refresh_token",
                            tokens["refresh_token"].as_str().unwrap_or_default(),
                        )
                        .await
                        .unwrap();

                    // Store user context
                    session.insert("user_id", &claims.sub).await.unwrap();
                    session.insert("email", &claims.email).await.unwrap();

                    // Store name and picture if available from OAuth
                    if let Some(name) = tokens["name"].as_str() {
                        session.insert("name", name).await.unwrap();
                    }
                    if let Some(picture) = tokens["picture"].as_str() {
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
        Ok(res) => {
            let status = res.status();
            tracing::error!("OAuth callback failed with status: {}", status);
            Redirect::to("/login?error=oauth_failed").into_response()
        }
        Err(e) => {
            tracing::error!("Failed to call auth-service OAuth callback: {}", e);
            Redirect::to("/login?error=service_error").into_response()
        }
    }
}
