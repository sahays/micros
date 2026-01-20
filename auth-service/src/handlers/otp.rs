//! OTP authentication handlers for auth-service v2.
//!
//! Implements OTP send/verify flows for:
//! - Passwordless login
//! - Email verification
//! - Phone verification
//! - Password reset

use axum::{
    extract::{Json, State},
    http::StatusCode,
};
use chrono::Utc;
use rand::Rng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::models::{OtpChannel, OtpCode, OtpPurpose, RefreshSession};
use crate::AppState;
use service_core::error::AppError;

// ============================================================================
// Request/Response DTOs
// ============================================================================

/// Request to send an OTP.
#[derive(Debug, Deserialize)]
pub struct SendOtpRequest {
    pub tenant_id: Uuid,
    pub destination: String, // email or phone
    pub channel: OtpChannel,
    pub purpose: OtpPurpose,
}

/// Response after sending OTP.
#[derive(Debug, Serialize)]
pub struct SendOtpResponse {
    pub otp_id: Uuid,
    pub expires_in: i64, // seconds
}

/// Request to verify an OTP.
#[derive(Debug, Deserialize)]
pub struct VerifyOtpRequest {
    pub otp_id: Uuid,
    pub code: String,
}

/// Response after verifying OTP for login.
#[derive(Debug, Serialize)]
pub struct VerifyOtpLoginResponse {
    pub user_id: Uuid,
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: i64,
}

/// Response after verifying OTP for verification.
#[derive(Debug, Serialize)]
pub struct VerifyOtpVerifyResponse {
    pub verified: bool,
    pub purpose: String,
}

/// Generic verify response that can be either type.
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum VerifyOtpResponse {
    Login(VerifyOtpLoginResponse),
    Verify(VerifyOtpVerifyResponse),
}

// ============================================================================
// Configuration
// ============================================================================

const OTP_LENGTH: usize = 6;
const OTP_EXPIRY_SECONDS: i64 = 300; // 5 minutes
const OTP_MAX_ATTEMPTS: i32 = 5;

// ============================================================================
// Handlers
// ============================================================================

/// Send an OTP to the specified destination - implementation.
///
/// This function contains the core OTP sending logic and can be called
/// from both REST handlers and gRPC services.
#[tracing::instrument(
    skip(state),
    fields(tenant_id = %req.tenant_id, channel = ?req.channel, purpose = ?req.purpose)
)]
pub async fn send_otp_impl(
    state: &AppState,
    req: SendOtpRequest,
) -> Result<SendOtpResponse, AppError> {
    // Validate tenant exists
    let _tenant = state
        .db
        .find_tenant_by_id(req.tenant_id)
        .await
        .map_err(|e| AppError::InternalError(anyhow::anyhow!("Database error: {}", e)))?
        .ok_or_else(|| AppError::NotFound(anyhow::anyhow!("Tenant not found")))?;

    // Validate destination format
    validate_destination(&req.destination, &req.channel)?;

    // Check rate limit (3 OTPs per destination per 15 minutes)
    let recent_count = state
        .db
        .count_recent_otps(&req.destination, 15 * 60)
        .await
        .map_err(|e| AppError::InternalError(anyhow::anyhow!("Database error: {}", e)))?;

    if recent_count >= 3 {
        return Err(AppError::TooManyRequests(
            "Too many OTP requests. Please try again later.".to_string(),
            Some(15 * 60), // Retry after 15 minutes
        ));
    }

    // For login purpose, verify user exists (email-based only for now)
    if req.purpose == OtpPurpose::Login {
        // Currently only email-based login is supported
        // SMS/WhatsApp login requires phone number lookup via user_identities
        if req.channel != OtpChannel::Email {
            return Err(AppError::BadRequest(anyhow::anyhow!(
                "Phone-based login not yet implemented. Use email channel."
            )));
        }

        let user = state
            .db
            .find_user_by_email_in_tenant(req.tenant_id, &req.destination)
            .await
            .map_err(|e| AppError::InternalError(anyhow::anyhow!("Database error: {}", e)))?;

        if user.is_none() {
            return Err(AppError::NotFound(anyhow::anyhow!("User not found")));
        }
    }

    // Generate OTP code
    let code = generate_otp(OTP_LENGTH);
    let code_hash = hash_otp(&code);

    // Create OTP record
    let otp = OtpCode::new(
        Some(req.tenant_id),
        req.destination.clone(),
        req.channel.clone(),
        req.purpose.clone(),
        code_hash,
        OTP_EXPIRY_SECONDS,
        OTP_MAX_ATTEMPTS,
    );

    let otp_id = otp.otp_id;

    state
        .db
        .insert_otp_code(&otp)
        .await
        .map_err(|e| AppError::InternalError(anyhow::anyhow!("Database error: {}", e)))?;

    // Send OTP via appropriate channel
    match req.channel {
        OtpChannel::Email => {
            send_otp_email(state, &req.destination, &code, &req.purpose).await?;
        }
        OtpChannel::Sms => {
            // TODO: Implement SMS sending via Twilio
            tracing::warn!(
                otp_id = %otp_id,
                channel = "sms",
                "SMS OTP sending not implemented"
            );
        }
        OtpChannel::Whatsapp => {
            // TODO: Implement WhatsApp sending
            tracing::warn!(
                otp_id = %otp_id,
                channel = "whatsapp",
                "WhatsApp OTP sending not implemented"
            );
        }
    }

    Ok(SendOtpResponse {
        otp_id,
        expires_in: OTP_EXPIRY_SECONDS,
    })
}

/// Send an OTP to the specified destination.
///
/// POST /auth/otp/send
pub async fn send_otp(
    State(state): State<AppState>,
    Json(req): Json<SendOtpRequest>,
) -> Result<(StatusCode, Json<SendOtpResponse>), AppError> {
    let response = send_otp_impl(&state, req).await?;
    Ok((StatusCode::OK, Json(response)))
}

/// Verify an OTP code - implementation.
///
/// This function contains the core OTP verification logic and can be called
/// from both REST handlers and gRPC services.
#[tracing::instrument(skip(state, req), fields(otp_id = %req.otp_id))]
pub async fn verify_otp_impl(
    state: &AppState,
    req: VerifyOtpRequest,
) -> Result<VerifyOtpResponse, AppError> {
    // Find OTP record
    let otp = state
        .db
        .find_otp_by_id(req.otp_id)
        .await
        .map_err(|e| AppError::InternalError(anyhow::anyhow!("Database error: {}", e)))?
        .ok_or_else(|| AppError::NotFound(anyhow::anyhow!("OTP not found")))?;

    // Check if already consumed
    if otp.consumed_utc.is_some() {
        return Err(AppError::BadRequest(anyhow::anyhow!(
            "OTP has already been used"
        )));
    }

    // Check expiry
    if otp.expiry_utc < Utc::now() {
        return Err(AppError::BadRequest(anyhow::anyhow!("OTP has expired")));
    }

    // Check attempt count
    if otp.attempt_count >= otp.attempt_max {
        return Err(AppError::BadRequest(anyhow::anyhow!(
            "Maximum verification attempts exceeded"
        )));
    }

    // Increment attempt count
    state
        .db
        .increment_otp_attempts(otp.otp_id)
        .await
        .map_err(|e| AppError::InternalError(anyhow::anyhow!("Database error: {}", e)))?;

    // Verify code
    let code_hash = hash_otp(&req.code);
    if code_hash != otp.code_hash_text {
        return Err(AppError::AuthError(anyhow::anyhow!("Invalid OTP code")));
    }

    // Mark as consumed
    state
        .db
        .consume_otp(otp.otp_id)
        .await
        .map_err(|e| AppError::InternalError(anyhow::anyhow!("Database error: {}", e)))?;

    // Handle based on purpose
    let purpose = OtpPurpose::parse(&otp.purpose_code);
    match purpose {
        OtpPurpose::Login => {
            // Find user and generate tokens
            let tenant_id = otp
                .tenant_id
                .ok_or_else(|| AppError::BadRequest(anyhow::anyhow!("Tenant ID required")))?;

            let user = find_user_by_email(state, tenant_id, &otp.destination_text).await?;

            // Generate tokens
            let (access_token, refresh_token, refresh_token_id) = state
                .jwt
                .generate_token_pair(
                    &user.user_id.to_string(),
                    &tenant_id.to_string(),
                    "", // org_id filled from context
                    &user.email,
                )
                .map_err(|e| {
                    AppError::InternalError(anyhow::anyhow!("Token generation failed: {}", e))
                })?;

            // Store refresh session
            let refresh_hash = hash_otp(&refresh_token_id);
            let session = RefreshSession::new(
                user.user_id,
                refresh_hash,
                state.jwt.refresh_token_expiry_days(),
            );
            state
                .db
                .insert_refresh_session(&session)
                .await
                .map_err(|e| AppError::InternalError(anyhow::anyhow!("Database error: {}", e)))?;

            Ok(VerifyOtpResponse::Login(VerifyOtpLoginResponse {
                user_id: user.user_id,
                access_token,
                refresh_token,
                token_type: "Bearer".to_string(),
                expires_in: state.jwt.access_token_expiry_seconds(),
            }))
        }
        OtpPurpose::VerifyEmail => {
            // Mark email as verified
            if let Some(tenant_id) = otp.tenant_id {
                if let Ok(Some(user)) = state
                    .db
                    .find_user_by_email_in_tenant(tenant_id, &otp.destination_text)
                    .await
                {
                    state
                        .db
                        .mark_email_verified(user.user_id)
                        .await
                        .map_err(|e| {
                            AppError::InternalError(anyhow::anyhow!("Database error: {}", e))
                        })?;
                }
            }

            Ok(VerifyOtpResponse::Verify(VerifyOtpVerifyResponse {
                verified: true,
                purpose: "verify_email".to_string(),
            }))
        }
        OtpPurpose::VerifyPhone => {
            // Phone verification requires looking up user by phone in user_identities
            // For now, just return verified status - actual marking would need phone->user lookup
            tracing::warn!(
                destination = %otp.destination_text,
                "Phone verification completed but user lookup by phone not yet implemented"
            );

            Ok(VerifyOtpResponse::Verify(VerifyOtpVerifyResponse {
                verified: true,
                purpose: "verify_phone".to_string(),
            }))
        }
        OtpPurpose::ResetPassword => {
            // Return success - caller should use this to enable password reset
            Ok(VerifyOtpResponse::Verify(VerifyOtpVerifyResponse {
                verified: true,
                purpose: "reset_password".to_string(),
            }))
        }
    }
}

/// Verify an OTP code.
///
/// POST /auth/otp/verify
pub async fn verify_otp(
    State(state): State<AppState>,
    Json(req): Json<VerifyOtpRequest>,
) -> Result<Json<VerifyOtpResponse>, AppError> {
    let response = verify_otp_impl(&state, req).await?;
    Ok(Json(response))
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Generate a random numeric OTP.
fn generate_otp(length: usize) -> String {
    let mut rng = rand::thread_rng();
    (0..length)
        .map(|_| rng.gen_range(0..10).to_string())
        .collect()
}

/// Hash an OTP code for storage.
fn hash_otp(code: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(code.as_bytes());
    hex::encode(hasher.finalize())
}

/// Validate destination format based on channel.
fn validate_destination(destination: &str, channel: &OtpChannel) -> Result<(), AppError> {
    match channel {
        OtpChannel::Email => {
            if !destination.contains('@') || !destination.contains('.') {
                return Err(AppError::BadRequest(anyhow::anyhow!(
                    "Invalid email format"
                )));
            }
        }
        OtpChannel::Sms | OtpChannel::Whatsapp => {
            if !destination.starts_with('+') || destination.len() < 10 {
                return Err(AppError::BadRequest(anyhow::anyhow!(
                    "Invalid phone format. Use E.164 format (+1234567890)"
                )));
            }
        }
    }
    Ok(())
}

/// Send OTP via email.
///
/// In production, this sends the actual email. Currently logs for testing.
async fn send_otp_email(
    _state: &AppState,
    _email: &str,
    _code: &str,
    purpose: &OtpPurpose,
) -> Result<(), AppError> {
    // TODO: Use the actual email service in production
    // _state.email.send_otp(_email, _code, purpose).await?;
    let subject = match purpose {
        OtpPurpose::Login => "Your login code",
        OtpPurpose::VerifyEmail => "Verify your email",
        OtpPurpose::VerifyPhone => "Verify your phone",
        OtpPurpose::ResetPassword => "Reset your password",
    };

    // Use the email service - for now just log since we're using mock in tests
    // Note: Never log OTP codes - they are sensitive tokens
    tracing::info!(
        subject = %subject,
        purpose = ?purpose,
        "OTP email sent"
    );

    // In production, use the actual email service
    // _state.email.send_otp_email(email, code, subject).await?;

    Ok(())
}

/// Find user by email in a tenant.
async fn find_user_by_email(
    state: &AppState,
    tenant_id: Uuid,
    email: &str,
) -> Result<crate::models::User, AppError> {
    state
        .db
        .find_user_by_email_in_tenant(tenant_id, email)
        .await
        .map_err(|e| AppError::InternalError(anyhow::anyhow!("Database error: {}", e)))?
        .ok_or_else(|| AppError::NotFound(anyhow::anyhow!("User not found")))
}
