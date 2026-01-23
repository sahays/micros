//! Auth Service v2
//!
//! PostgreSQL-backed authentication and authorization service with:
//! - Capability-based authorization
//! - Org node hierarchy with closure table
//! - Time-bounded immutable assignments
//! - gRPC-only API (HTTP only for health checks)

pub mod config;
pub mod db;
pub mod grpc;
pub mod handlers;
pub mod models;
pub mod services;

use std::sync::Arc;

use crate::config::AuthConfig;
use crate::services::{Database, EmailProvider, JwtService, TokenBlacklist};

/// Application state shared across handlers.
#[derive(Clone)]
pub struct AppState {
    pub config: AuthConfig,
    pub db: Database,
    pub email: Arc<dyn EmailProvider>,
    pub jwt: JwtService,
    pub redis: Arc<dyn TokenBlacklist>,
}
