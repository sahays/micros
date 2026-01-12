pub mod documents;
pub mod health;

pub use documents::{get_document_status, process_document, upload_document};
pub use health::health_check;
