//! Services layer for auth-service v2.
//!
//! Provides business logic services for authentication, authorization,
//! and other core functionality.

mod assignment_db;
mod audit_db;
pub mod crypto;
mod database;
mod email;
pub mod error;
mod invitation_db;
mod jwt;
pub mod metrics;
mod notification_client;
mod org_db;
mod otp_db;
pub mod redis;
mod role_db;
mod session_db;
mod tenant_db;
mod user_db;

pub use crypto::hash_password;
pub use database::Database;
pub use email::{EmailProvider, EmailService, MockEmailService};
pub use error::ServiceError;
pub use jwt::{AccessTokenClaims, JwtService, RefreshTokenClaims, ResetTokenClaims, TokenResponse};
pub use notification_client::NotificationClient;
pub use redis::{MockBlacklist, RedisService, TokenBlacklist};
