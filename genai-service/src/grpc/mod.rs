pub mod genai_service;

pub use genai_service::GenaiGrpcService;

/// Include generated proto code.
pub mod proto {
    tonic::include_proto!("micros.genai.v1");

    pub const FILE_DESCRIPTOR_SET: &[u8] = tonic::include_file_descriptor_set!("genai_descriptor");
}

/// Document service proto (client only).
pub mod document_proto {
    tonic::include_proto!("micros.document.v1");
}
