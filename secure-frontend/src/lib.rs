pub mod config;
pub mod handlers;
pub mod middleware;
pub mod models;
pub mod services;
pub mod startup;
pub mod utils;

use services::{auth_client::AuthClient, document_client::DocumentClient};
use std::sync::Arc;

/// Shared application state containing service clients
#[derive(Clone)]
pub struct AppState {
    pub auth_client: Arc<AuthClient>,
    pub document_client: Arc<DocumentClient>,
}

impl AppState {
    pub fn new(auth_client: Arc<AuthClient>, document_client: Arc<DocumentClient>) -> Self {
        Self {
            auth_client,
            document_client,
        }
    }
}
