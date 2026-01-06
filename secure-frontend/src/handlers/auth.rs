use crate::services::auth_client::AuthClient;
use askama::Template;
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{Html, IntoResponse},
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

            // Store tokens in session
            session
                .insert(
                    "access_token",
                    tokens["access_token"].as_str().unwrap_or_default(),
                )
                .await
                .unwrap();
            session
                .insert(
                    "refresh_token",
                    tokens["refresh_token"].as_str().unwrap_or_default(),
                )
                .await
                .unwrap();

            // HTMX Redirect to dashboard
            let mut headers = HeaderMap::new();
            headers.insert("HX-Redirect", "/dashboard".parse().unwrap());
            (StatusCode::OK, headers, "").into_response()
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

pub async fn logout_handler(session: Session) -> impl IntoResponse {
    session.clear().await;
    let mut headers = HeaderMap::new();
    headers.insert("HX-Redirect", "/".parse().unwrap());
    (StatusCode::OK, headers, "").into_response()
}
