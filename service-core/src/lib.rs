//! service-core: Shared infrastructure for micros microservices.
pub mod config;
pub mod error;
pub mod middleware;
pub mod observability;
pub mod utils;

pub use async_trait;
pub use axum;
pub use mongodb;
pub use serde;
pub use serde_json;
pub use tokio;
pub use tower;
pub use tower_http;
pub use tracing;
pub use validator;
