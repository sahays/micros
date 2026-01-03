use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use mongodb::bson::doc;
use rand::Rng;
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::{
    models::{User, VerificationToken},
    utils::{hash_password, Password},
    AppState,
};

#[derive(Debug, Deserialize, Validate)]
pub struct RegisterRequest {
    #[validate(email(message = "Invalid email format"))]
    pub email: String,

    #[validate(length(min = 8, message = "Password must be at least 8 characters"))]
    pub password: String,

    pub name: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RegisterResponse {
    pub user_id: String,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

pub async fn register(
    State(state): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    // Validate request
    req.validate()
        .map_err(|e| {
            (
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(ErrorResponse {
                    error: format!("Validation error: {}", e),
                }),
            )
        })?;

    // Check if user already exists
    let existing_user = state
        .db
        .users()
        .find_one(doc! { "email": &req.email }, None)
        .await
        .map_err(|e| {
            tracing::error!("Database error checking existing user: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Internal server error".to_string(),
                }),
            )
        })?;

    if existing_user.is_some() {
        return Err((
            StatusCode::CONFLICT,
            Json(ErrorResponse {
                error: "Email already registered".to_string(),
            }),
        ));
    }

    // Hash password
    let password_hash = hash_password(&Password::new(req.password))
        .map_err(|e| {
            tracing::error!("Password hashing error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Internal server error".to_string(),
                }),
            )
        })?;

    // Create user
    let user = User::new(req.email.clone(), password_hash.into_string(), req.name);

    state
        .db
        .users()
        .insert_one(&user, None)
        .await
        .map_err(|e| {
            tracing::error!("Database error creating user: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Internal server error".to_string(),
                }),
            )
        })?;

    tracing::info!("User registered: {}", user.id);

    // Generate verification token
    let token = generate_random_token();
    let verification_token = VerificationToken::new_email_verification(user.id.clone(), token.clone());

    state
        .db
        .verification_tokens()
        .insert_one(&verification_token, None)
        .await
        .map_err(|e| {
            tracing::error!("Database error creating verification token: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Internal server error".to_string(),
                }),
            )
        })?;

    // Send verification email
    let base_url = format!("http://localhost:{}", state.config.port);
    state
        .email
        .send_verification_email(&req.email, &token, &base_url)
        .await
        .map_err(|e| {
            tracing::error!("Failed to send verification email: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to send verification email".to_string(),
                }),
            )
        })?;

    Ok((
        StatusCode::CREATED,
        Json(RegisterResponse {
            user_id: user.id,
            message: "Registration successful. Please check your email to verify your account.".to_string(),
        }),
    ))
}

#[derive(Debug, Deserialize)]
pub struct VerifyRequest {
    pub token: String,
}

#[derive(Debug, Serialize)]
pub struct VerifyResponse {
    pub message: String,
}

pub async fn verify_email(
    State(state): State<AppState>,
    axum::extract::Query(req): axum::extract::Query<VerifyRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    // Find verification token
    let verification_token = state
        .db
        .verification_tokens()
        .find_one(doc! { "token": &req.token }, None)
        .await
        .map_err(|e| {
            tracing::error!("Database error finding verification token: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Internal server error".to_string(),
                }),
            )
        })?;

    let verification_token = verification_token.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "Invalid or expired verification token".to_string(),
            }),
        )
    })?;

    // Check if token is expired
    if verification_token.is_expired() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Verification token has expired".to_string(),
            }),
        ));
    }

    // Update user as verified
    let result = state
        .db
        .users()
        .update_one(
            doc! { "_id": &verification_token.user_id },
            doc! { "$set": { "verified": true } },
            None,
        )
        .await
        .map_err(|e| {
            tracing::error!("Database error updating user: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Internal server error".to_string(),
                }),
            )
        })?;

    if result.matched_count == 0 {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "User not found".to_string(),
            }),
        ));
    }

    // Delete used token
    state
        .db
        .verification_tokens()
        .delete_one(doc! { "_id": &verification_token.id }, None)
        .await
        .map_err(|e| {
            tracing::error!("Database error deleting verification token: {}", e);
            // Don't fail the request if token deletion fails
        })
        .ok();

    tracing::info!("Email verified for user: {}", verification_token.user_id);

    Ok((
        StatusCode::OK,
        Json(VerifyResponse {
            message: "Email verified successfully".to_string(),
        }),
    ))
}

fn generate_random_token() -> String {
    let mut rng = rand::thread_rng();
    let token_bytes: [u8; 32] = rng.gen();
    hex::encode(token_bytes)
}
