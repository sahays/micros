use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Redirect},
    Json,
};
use axum_extra::extract::cookie::{Cookie, CookieJar};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use mongodb::bson::doc;
use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::{
    dtos::{auth::GoogleCallbackQuery, ErrorResponse},
    AppState,
};

#[derive(Debug, Deserialize)]
struct GoogleTokenResponse {
    access_token: String,
}

#[derive(Debug, Deserialize)]
struct GoogleUserInfo {
    id: String,
    email: String,
    verified_email: bool,
    name: Option<String>,
    #[allow(dead_code)]
    picture: Option<String>,
}

pub async fn google_login(
    State(state): State<AppState>,
    jar: CookieJar,
) -> (CookieJar, impl IntoResponse) {
    let state_val = uuid::Uuid::new_v4().to_string();
    let code_verifier = {
        let mut rng = rand::thread_rng();
        let mut bytes = [0u8; 32];
        use rand::Rng;
        rng.fill(&mut bytes);
        URL_SAFE_NO_PAD.encode(bytes)
    };

    let code_challenge = {
        let mut hasher = Sha256::new();
        hasher.update(code_verifier.as_bytes());
        URL_SAFE_NO_PAD.encode(hasher.finalize())
    };

    let google_url = format!(
        "https://accounts.google.com/o/oauth2/v2/auth?client_id={}&redirect_uri={}&response_type=code&scope=openid%20email%20profile&state={}&code_challenge={}&code_challenge_method=S256",
        state.config.google.client_id,
        state.config.google.redirect_uri,
        state_val,
        code_challenge
    );

    let updated_jar = jar
        .add(
            Cookie::build(("oauth_state", state_val))
                .path("/")
                .http_only(true)
                .secure(true)
                .max_age(time::Duration::minutes(5))
                .build(),
        )
        .add(
            Cookie::build(("code_verifier", code_verifier))
                .path("/")
                .http_only(true)
                .secure(true)
                .max_age(time::Duration::minutes(5))
                .build(),
        );

    (updated_jar, Redirect::to(&google_url))
}

pub async fn google_callback(
    State(state): State<AppState>,
    jar: CookieJar,
    Query(query): Query<GoogleCallbackQuery>,
) -> Result<(CookieJar, impl IntoResponse), (StatusCode, Json<ErrorResponse>)> {
    // 1. Validate state
    let stored_state = jar.get("oauth_state").map(|c| c.value());
    if stored_state != Some(&query.state) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Invalid OAuth state".to_string(),
            }),
        ));
    }

    // 2. Get code verifier
    let code_verifier = jar.get("code_verifier").map(|c| c.value()).ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Missing code verifier".to_string(),
            }),
        )
    })?;

    // 3. Exchange code for access token
    let client = reqwest::Client::new();
    let token_res = client
        .post("https://oauth2.googleapis.com/token")
        .form(&[
            ("client_id", &state.config.google.client_id),
            ("client_secret", &state.config.google.client_secret),
            ("code", &query.code),
            ("code_verifier", &code_verifier.to_string()),
            ("grant_type", &"authorization_code".to_string()),
            ("redirect_uri", &state.config.google.redirect_uri),
        ])
        .send()
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to exchange Google code");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Authentication failed".to_string(),
                }),
            )
        })?;

    if !token_res.status().is_success() {
        let status = token_res.status();
        let err_body = token_res.text().await.unwrap_or_default();
        tracing::error!(status = %status, body = %err_body, "Google token exchange error");
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "Authentication failed".to_string(),
            }),
        ));
    }

    let token_data: GoogleTokenResponse = token_res.json().await.map_err(|e| {
        tracing::error!(error = %e, "Failed to parse Google token response");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "Internal server error".to_string(),
            }),
        )
    })?;

    // 4. Get user info from Google
    let user_info_res = client
        .get("https://www.googleapis.com/oauth2/v2/userinfo")
        .bearer_auth(token_data.access_token)
        .send()
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to fetch Google user info");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Authentication failed".to_string(),
                }),
            )
        })?;

    let user_info: GoogleUserInfo = user_info_res.json().await.map_err(|e| {
        tracing::error!(error = %e, "Failed to parse Google user info");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "Internal server error".to_string(),
            }),
        )
    })?;

    if !user_info.verified_email {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Google account email not verified".to_string(),
            }),
        ));
    }

    // 5. Find or create user in database
    let existing_user = state
        .db
        .users()
        .find_one(doc! { "email": &user_info.email }, None)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Database error finding user");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Internal server error".to_string(),
                }),
            )
        })?;

    let user = match existing_user {
        Some(u) => {
            // Update Google ID if not set
            if u.google_id.is_none() {
                state
                    .db
                    .users()
                    .update_one(
                        doc! { "_id": &u.id },
                        doc! { "$set": { "google_id": &user_info.id } },
                        None,
                    )
                    .await
                    .ok();
            }
            u
        }
        None => {
            // Create new user (social login users are auto-verified)
            let mut new_user = crate::models::User::new(
                user_info.email.clone(),
                "SOCIAL_AUTH".to_string(),
                user_info.name,
            );
            new_user.google_id = Some(user_info.id);
            new_user.verified = true;

            state
                .db
                .users()
                .insert_one(&new_user, None)
                .await
                .map_err(|e| {
                    tracing::error!(error = %e, "Database error creating social user");
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ErrorResponse {
                            error: "Internal server error".to_string(),
                        }),
                    )
                })?;
            new_user
        }
    };

    // 6. Generate tokens
    let (access_token, refresh_token_str, refresh_token_id) = state
        .jwt
        .generate_token_pair(&user.id, &user.email)
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to generate token pair");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Internal server error".to_string(),
                }),
            )
        })?;

    // Store refresh token
    use crate::models::RefreshToken;
    let refresh_token = RefreshToken::new_with_id(
        refresh_token_id,
        user.id.clone(),
        &refresh_token_str,
        state.config.jwt.refresh_token_expiry_days,
    );

    state
        .db
        .refresh_tokens()
        .insert_one(&refresh_token, None)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Database error storing refresh token");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Internal server error".to_string(),
                }),
            )
        })?;

    tracing::info!(user_id = %user.id, "User logged in via Google");

    // 7. Redirect to frontend with tokens (simplified for demo)
    // In production, you'd use secure cookies or a temporary auth code
    let redirect_url = format!(
        "{}?access_token={}&refresh_token={}",
        state.config.google.frontend_url, access_token, refresh_token_str
    );

    let updated_jar = jar
        .remove(Cookie::from("oauth_state"))
        .remove(Cookie::from("code_verifier"));

    Ok((updated_jar, Redirect::to(&redirect_url)))
}
