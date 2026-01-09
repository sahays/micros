pub mod admin;
pub mod auth;

use service_core::serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ErrorResponse {
    #[schema(example = "Invalid email or password")]
    pub error: String,
}
