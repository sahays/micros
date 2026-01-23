pub mod capability_check;
pub mod document_service;

pub use capability_check::{capabilities, CapabilityChecker};
pub use document_service::DocumentGrpcService;

// Include generated proto code
pub mod proto {
    tonic::include_proto!("micros.document.v1");

    pub const FILE_DESCRIPTOR_SET: &[u8] =
        tonic::include_file_descriptor_set!("document_descriptor");
}
