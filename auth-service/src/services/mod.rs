pub mod admin;
pub mod auth;
mod database;
mod email;
pub mod error;
mod jwt;
pub mod metrics;
pub mod policy;
pub mod redis;
pub mod security_audit;

pub use auth::AuthService;
pub use database::MongoDb;
pub use email::{EmailProvider, EmailService, MockEmailService};
pub use error::ServiceError;
pub use jwt::{AccessTokenClaims, AppTokenClaims, JwtService, RefreshTokenClaims, TokenResponse};
pub use policy::{PolicyError, PolicyService};
pub use redis::{MockBlacklist, RedisService, TokenBlacklist};
pub use security_audit::{SecurityAuditLog, SecurityAuditService, SecurityEventType};
