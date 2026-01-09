use mongodb::bson::doc;
use serde::Deserialize;
use service_core::{
    axum::{
        extract::{ConnectInfo, State},
        http::StatusCode,
        response::IntoResponse,
        Json,
    },
    error::AppError,
};
use std::net::SocketAddr;
use utoipa::ToSchema;
use validator::Validate;

use crate::{
    middleware::AuthUser,
    models::{AuditLog, VerificationToken},
    utils::{hash_password, verify_password, Password, PasswordHashString},
    AppState,
};

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct ChangePasswordRequest {
    #[schema(example = "oldpassword123")]
    pub current_password: String,
    #[validate(length(min = 8, message = "New password must be at least 8 characters"))]
    #[schema(example = "newpassword123", min_length = 8)]
    pub new_password: String,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct UpdateUserRequest {
    #[schema(example = "John Smith")]
    pub name: Option<String>,
    #[validate(email(message = "Invalid email format"))]
    #[schema(example = "john.smith@example.com")]
    pub email: Option<String>,
}

/// Get current user profile
#[utoipa::path(
    get,
    path = "/users/me",
    responses(
        (status = 200, description = "Current user profile returned", body = SanitizedUser),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 404, description = "User not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "User",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn get_me(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<impl IntoResponse, AppError> {
    let claims = user.0;

    let user = state
        .db
        .users()
        .find_one(doc! { "_id": &claims.sub }, None)
        .await?
        .ok_or_else(|| AppError::NotFound(anyhow::anyhow!("User not found")))?;

    Ok(Json(user.sanitized()))
}

/// Update current user profile
#[utoipa::path(
    patch,
    path = "/users/me",
    request_body = UpdateUserRequest,
    responses(
        (status = 200, description = "User profile updated successfully", body = SanitizedUser),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 409, description = "Email already in use", body = ErrorResponse),
        (status = 422, description = "Validation error", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "User",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn update_me(
    State(state): State<AppState>,
    user_claims: AuthUser,
    Json(req): Json<UpdateUserRequest>,
) -> Result<impl IntoResponse, AppError> {
    // 1. Validate request
    req.validate()?;

    let claims = user_claims.0;

    // 2. Fetch current user
    let user = state
        .db
        .users()
        .find_one(doc! { "_id": &claims.sub }, None)
        .await?
        .ok_or_else(|| AppError::NotFound(anyhow::anyhow!("User not found")))?;

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
                .await?;

            if existing.is_some() {
                return Err(AppError::Conflict(anyhow::anyhow!("Email already in use")));
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
        .await?;

    // 6. If email changed, trigger verification flow
    if email_changed {
        // We know email is Some because email_changed is only true when email is Some
        let new_email = req.email.ok_or_else(|| {
            AppError::InternalError(anyhow::anyhow!(
                "Internal server error: email missing after change check"
            ))
        })?;
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
            .ok();

        // Send verification email
        let base_url = format!("http://localhost:{}", state.config.common.port);
        state
            .email
            .send_verification_email(&new_email, &token, &base_url)
            .await
            .ok();
    }

    // 7. Return updated user
    let updated_user = state
        .db
        .users()
        .find_one(doc! { "_id": &user.id }, None)
        .await?
        .ok_or_else(|| AppError::InternalError(anyhow::anyhow!("User not found after update")))?;

    Ok(Json(updated_user.sanitized()))
}

/// Change current user password
#[utoipa::path(
    post,
    path = "/users/me/password",
    request_body = ChangePasswordRequest,
    responses(
        (status = 200, description = "Password changed successfully"),
        (status = 401, description = "Unauthorized or incorrect password", body = ErrorResponse),
        (status = 422, description = "Validation error", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "User",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn change_password(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    user_claims: AuthUser,
    Json(req): Json<ChangePasswordRequest>,
) -> Result<impl IntoResponse, AppError> {
    let ip_address = addr.to_string();
    // 1. Validate request
    req.validate()?;

    let claims = user_claims.0;

    // 2. Fetch user
    let user = state
        .db
        .users()
        .find_one(doc! { "_id": &claims.sub }, None)
        .await?
        .ok_or_else(|| AppError::NotFound(anyhow::anyhow!("User not found")))?;

    // 3. Verify current password
    verify_password(
        &Password::new(req.current_password),
        &PasswordHashString::new(user.password_hash.clone()),
    )
    .map_err(|_| AppError::AuthError(anyhow::anyhow!("Incorrect current password")))?;

    // 4. Hash new password
    let new_password_hash = hash_password(&Password::new(req.new_password))?;

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
        .await?;

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
        .ok();

    // Audit log password change
    let audit_log = AuditLog::new(
        "password_change".to_string(),
        Some(user.id.clone()),
        "/users/me/password".to_string(),
        "POST".to_string(),
        StatusCode::OK.as_u16(),
        ip_address.clone(),
    );
    let db = state.db.clone();
    tokio::spawn(async move {
        let _ = db.audit_logs().insert_one(audit_log, None).await;
    });

    Ok(Json(serde_json::json!({
        "message": "Password changed successfully. All other sessions have been logged out."
    })))
}
