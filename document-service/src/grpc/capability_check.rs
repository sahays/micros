//! Capability definitions for document-service.
//!
//! Re-exports shared capability infrastructure from service-core and
//! provides service-specific capability constants.

pub use service_core::grpc::{
    extract_bearer_token, extract_org_node_id, AuthContext, CapabilityChecker, CapabilityMetadata,
};

/// Document service capabilities.
pub mod capabilities {
    /// Upload new documents.
    pub const DOCUMENT_UPLOAD: &str = "document:upload";

    /// Download document content.
    pub const DOCUMENT_DOWNLOAD: &str = "document:download";

    /// View document metadata.
    pub const DOCUMENT_READ: &str = "document:read";

    /// Delete documents.
    pub const DOCUMENT_DELETE: &str = "document:delete";

    /// Trigger document processing.
    pub const DOCUMENT_PROCESS: &str = "document:process";

    /// Generate signed URLs.
    pub const DOCUMENT_SIGNED_URL: &str = "document.signed_url:generate";
}
