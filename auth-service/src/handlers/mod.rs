//! HTTP handlers for auth-service v2.

pub mod assignment;
pub mod auth;
pub mod authz;
pub mod context;
pub mod org;
pub mod role;
pub mod service;

pub use assignment::*;
pub use auth::*;
pub use authz::*;
pub use context::*;
pub use org::*;
pub use role::*;
pub use service::*;
