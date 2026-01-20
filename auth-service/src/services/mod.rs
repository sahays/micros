//! Services layer for auth-service v2.
//!
//! Provides business logic services for authentication, authorization,
//! and other core functionality.

mod database;
mod email;
pub mod error;
mod jwt;
pub mod metrics;
mod notification_client;
pub mod redis;

pub use database::Database;
pub use email::{EmailProvider, EmailService, MockEmailService};
pub use error::ServiceError;
pub use jwt::{AccessTokenClaims, JwtService, RefreshTokenClaims, TokenResponse};
pub use notification_client::{NotificationClient, NotificationClientConfig};
pub use redis::{MockBlacklist, RedisService, TokenBlacklist};
