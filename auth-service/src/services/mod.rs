//! Services layer for auth-service v2.
//!
//! Provides business logic services for authentication, authorization,
//! and other core functionality.

pub mod crypto;
mod database;
mod email;
pub mod error;
mod jwt;
pub mod metrics;
mod notification_client;
pub mod redis;

pub use crypto::hash_password;
pub use database::Database;
pub use email::{EmailProvider, EmailService, MockEmailService};
pub use error::ServiceError;
pub use jwt::{AccessTokenClaims, JwtService, RefreshTokenClaims, ResetTokenClaims, TokenResponse};
pub use notification_client::NotificationClient;
pub use redis::{MockBlacklist, RedisService, TokenBlacklist};
