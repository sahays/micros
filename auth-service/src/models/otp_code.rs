//! OTP code model - one-time password verification.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// OTP delivery channel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OtpChannel {
    Email,
    Sms,
    Whatsapp,
}

impl OtpChannel {
    pub fn as_str(&self) -> &'static str {
        match self {
            OtpChannel::Email => "email",
            OtpChannel::Sms => "sms",
            OtpChannel::Whatsapp => "whatsapp",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "sms" => OtpChannel::Sms,
            "whatsapp" => OtpChannel::Whatsapp,
            _ => OtpChannel::Email,
        }
    }
}

/// OTP purpose codes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OtpPurpose {
    Login,
    VerifyEmail,
    VerifyPhone,
    ResetPassword,
}

impl OtpPurpose {
    pub fn as_str(&self) -> &'static str {
        match self {
            OtpPurpose::Login => "login",
            OtpPurpose::VerifyEmail => "verify_email",
            OtpPurpose::VerifyPhone => "verify_phone",
            OtpPurpose::ResetPassword => "reset_password",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "login" => OtpPurpose::Login,
            "verify_phone" => OtpPurpose::VerifyPhone,
            "reset_password" => OtpPurpose::ResetPassword,
            _ => OtpPurpose::VerifyEmail,
        }
    }
}

/// OTP code entity.
#[derive(Debug, Clone, FromRow)]
pub struct OtpCode {
    pub otp_id: Uuid,
    pub tenant_id: Option<Uuid>,
    pub destination_text: String,
    pub channel_code: String,
    pub purpose_code: String,
    pub code_hash_text: String,
    pub expiry_utc: DateTime<Utc>,
    pub consumed_utc: Option<DateTime<Utc>>,
    pub attempt_count: i32,
    pub attempt_max: i32,
    pub created_utc: DateTime<Utc>,
}

impl OtpCode {
    /// Create a new OTP code.
    pub fn new(
        tenant_id: Option<Uuid>,
        destination: String,
        channel: OtpChannel,
        purpose: OtpPurpose,
        code_hash: String,
        expiry_seconds: i64,
        max_attempts: i32,
    ) -> Self {
        Self {
            otp_id: Uuid::new_v4(),
            tenant_id,
            destination_text: destination,
            channel_code: channel.as_str().to_string(),
            purpose_code: purpose.as_str().to_string(),
            code_hash_text: code_hash,
            expiry_utc: Utc::now() + Duration::seconds(expiry_seconds),
            consumed_utc: None,
            attempt_count: 0,
            attempt_max: max_attempts,
            created_utc: Utc::now(),
        }
    }

    /// Check if OTP is still valid (not expired and not consumed).
    pub fn is_valid(&self) -> bool {
        let now = Utc::now();
        self.expiry_utc > now
            && self.consumed_utc.is_none()
            && self.attempt_count < self.attempt_max
    }

    /// Check if OTP has been consumed.
    pub fn is_consumed(&self) -> bool {
        self.consumed_utc.is_some()
    }

    /// Check if OTP has expired.
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expiry_utc
    }

    /// Check if max attempts exceeded.
    pub fn is_locked_out(&self) -> bool {
        self.attempt_count >= self.attempt_max
    }

    /// Get channel as enum.
    pub fn channel(&self) -> OtpChannel {
        OtpChannel::parse(&self.channel_code)
    }

    /// Get purpose as enum.
    pub fn purpose(&self) -> OtpPurpose {
        OtpPurpose::parse(&self.purpose_code)
    }
}
