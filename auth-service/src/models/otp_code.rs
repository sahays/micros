//! OTP code model - one-time password verification.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// OTP purpose codes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OtpPurpose {
    EmailVerification,
    PasswordReset,
    TwoFactorAuth,
}

impl OtpPurpose {
    pub fn as_str(&self) -> &'static str {
        match self {
            OtpPurpose::EmailVerification => "email_verification",
            OtpPurpose::PasswordReset => "password_reset",
            OtpPurpose::TwoFactorAuth => "two_factor_auth",
        }
    }
}

/// OTP code entity.
#[derive(Debug, Clone, FromRow)]
pub struct OtpCode {
    pub otp_id: Uuid,
    pub user_id: Uuid,
    pub purpose_code: String,
    pub otp_hash: String,
    pub expiry_utc: DateTime<Utc>,
    pub used_utc: Option<DateTime<Utc>>,
    pub created_utc: DateTime<Utc>,
}

impl OtpCode {
    /// Create a new OTP code.
    pub fn new(
        user_id: Uuid,
        purpose: OtpPurpose,
        otp_hash: String,
        expiry_utc: DateTime<Utc>,
    ) -> Self {
        Self {
            otp_id: Uuid::new_v4(),
            user_id,
            purpose_code: purpose.as_str().to_string(),
            otp_hash,
            expiry_utc,
            used_utc: None,
            created_utc: Utc::now(),
        }
    }

    /// Check if OTP is still valid (not expired and not used).
    pub fn is_valid(&self) -> bool {
        let now = Utc::now();
        self.expiry_utc > now && self.used_utc.is_none()
    }

    /// Check if OTP has been used.
    pub fn is_used(&self) -> bool {
        self.used_utc.is_some()
    }

    /// Check if OTP has expired.
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expiry_utc
    }
}

/// Request to verify an OTP.
#[derive(Debug, Deserialize)]
pub struct VerifyOtpRequest {
    pub user_id: Uuid,
    pub otp_code: String,
    pub purpose: OtpPurpose,
}

/// Request to send an OTP.
#[derive(Debug, Deserialize)]
pub struct SendOtpRequest {
    pub email: String,
    pub purpose: OtpPurpose,
}
