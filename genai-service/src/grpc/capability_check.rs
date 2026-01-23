//! Capability definitions for genai-service.
//!
//! Re-exports shared capability infrastructure from service-core and
//! provides service-specific capability constants.

pub use service_core::grpc::{
    extract_bearer_token, extract_org_node_id, AuthContext, CapabilityChecker, CapabilityMetadata,
};

/// GenAI service capabilities.
pub mod capabilities {
    /// Execute AI prompts (Process, ProcessStream).
    pub const GENAI_PROCESS: &str = "genai:process";

    /// Create conversation sessions.
    pub const GENAI_SESSION_CREATE: &str = "genai.session:create";

    /// View session details.
    pub const GENAI_SESSION_READ: &str = "genai.session:read";

    /// Delete sessions.
    pub const GENAI_SESSION_DELETE: &str = "genai.session:delete";

    /// Query usage statistics.
    pub const GENAI_USAGE_READ: &str = "genai.usage:read";

    /// List available models.
    pub const GENAI_MODELS_READ: &str = "genai.models:read";
}
