use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use mongodb::bson::doc;
use serde::Deserialize;
use validator::Validate;

use crate::{
    middleware::AuthUser,
    models::VerificationToken,
    utils::{hash_password, verify_password, Password, PasswordHashString},
    AppState,
};

#[derive(Debug, Deserialize, Validate)]
pub struct ChangePasswordRequest {
    pub current_password: String,
    #[validate(length(min = 8, message = "New password must be at least 8 characters"))]
    pub new_password: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateUserRequest {
    pub name: Option<String>,
    #[validate(email(message = "Invalid email format"))]
    pub email: Option<String>,
}

#[axum::debug_handler]
pub async fn get_me(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let claims = user.0;

    let user = state
        .db
        .users()
        .find_one(doc! { "_id": &claims.sub }, None)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Database error finding user");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "Internal server error" })),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "error": "User not found" })),
            )
        })?;

    Ok(Json(user.sanitized()))
}

#[axum::debug_handler]
pub async fn update_me(
    State(state): State<AppState>,
    user_claims: AuthUser,
    Json(req): Json<UpdateUserRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    // 1. Validate request
    req.validate().map_err(|e| {
        (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(serde_json::json!({ "error": format!("Validation error: {}", e) })),
        )
    })?;

    let claims = user_claims.0;

    // 2. Fetch current user
    let user = state
        .db
        .users()
        .find_one(doc! { "_id": &claims.sub }, None)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Database error finding user");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "Internal server error" })),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "error": "User not found" })),
            )
        })?;

    let mut update_doc = doc! {};
    let mut email_changed = false;

    // 3. Handle email change
    if let Some(ref new_email) = req.email {
        if new_email != &user.email {
            // Check uniqueness
            let existing = state
                .db
                .users()
                .find_one(doc! { "email": new_email }, None)
                .await
                .map_err(|e| {
                    tracing::error!(error = %e, "Database error checking email uniqueness");
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(serde_json::json!({ "error": "Internal server error" })),
                    )
                })?;

            if existing.is_some() {
                return Err((
                    StatusCode::CONFLICT,
                    Json(serde_json::json!({ "error": "Email already in use" })),
                ));
            }

            update_doc.insert("email", new_email);
            update_doc.insert("verified", false);
            email_changed = true;
        }
    }

    // 4. Handle name change
    if let Some(new_name) = req.name {
        update_doc.insert("name", new_name);
    }

    if update_doc.is_empty() {
        return Ok(Json(user.sanitized()));
    }

    update_doc.insert("updated_at", chrono::Utc::now());

    // 5. Update database
    state
        .db
        .users()
        .update_one(doc! { "_id": &user.id }, doc! { "$set": update_doc }, None)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Database error updating user");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "Internal server error" })),
            )
        })?;

    // 6. If email changed, trigger verification flow
    if email_changed {
        let new_email = req.email.unwrap();
        // Generate verification token
        let token = {
            let mut rng = rand::thread_rng();
            use rand::Rng;
            let token_bytes: [u8; 32] = rng.gen();
            hex::encode(token_bytes)
        };

        let verification_token =
            VerificationToken::new_email_verification(user.id.clone(), token.clone());

        state
            .db
            .verification_tokens()
            .insert_one(&verification_token, None)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Database error creating verification token");
                // Non-fatal for the update response, but should be logged
            })
            .ok();

        // Send verification email
        let base_url = format!("http://localhost:{}", state.config.port);
        state
            .email
            .send_verification_email(&new_email, &token, &base_url)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to send verification email");
            })
            .ok();
    }

    // 7. Return updated user
    let updated_user = state
        .db
        .users()
        .find_one(doc! { "_id": &user.id }, None)
        .await
        .unwrap() // Safe after successful update
        .unwrap();

    Ok(Json(updated_user.sanitized()))
}

#[axum::debug_handler]
pub async fn change_password(
    State(state): State<AppState>,
    user_claims: AuthUser,
    Json(req): Json<ChangePasswordRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    // 1. Validate request
    req.validate().map_err(|e| {
        (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(serde_json::json!({ "error": format!("Validation error: {}", e) })),
        )
    })?;

    let claims = user_claims.0;

    // 2. Fetch user
    let user = state
        .db
        .users()
        .find_one(doc! { "_id": &claims.sub }, None)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Database error finding user");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "Internal server error" })),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "error": "User not found" })),
            )
        })?;

    // 3. Verify current password
    verify_password(
        &Password::new(req.current_password),
        &PasswordHashString::new(user.password_hash.clone()),
    )
    .map_err(|_| {
        (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({ "error": "Incorrect current password" })),
        )
    })?;

    // 4. Hash new password
    let new_password_hash = hash_password(&Password::new(req.new_password)).map_err(|e| {
        tracing::error!(error = %e, "Password hashing error");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": "Internal server error" })),
        )
    })?;

    // 5. Update password and invalidate refresh tokens
    // Update password
    state
        .db
        .users()
        .update_one(
            doc! { "_id": &user.id },
            doc! {
                "$set": {
                    "password_hash": new_password_hash.into_string(),
                    "updated_at": chrono::Utc::now()
                }
            },
            None,
        )
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Database error updating password");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "Internal server error" })),
            )
        })?;

    // Invalidate all refresh tokens
    state
        .db
        .refresh_tokens()
        .update_many(
            doc! { "user_id": &user.id },
            doc! { "$set": { "revoked": true } },
            None,
        )
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Database error revoking refresh tokens");
            // Non-fatal for the response
        })
        .ok();

    Ok(Json(serde_json::json!({
        "message": "Password changed successfully. All other sessions have been logged out."
    })))
}
