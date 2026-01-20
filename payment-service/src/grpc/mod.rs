pub mod payment_service;

pub use payment_service::PaymentGrpcService;

// Include generated proto code
pub mod proto {
    tonic::include_proto!("micros.payment.v1");

    pub const FILE_DESCRIPTOR_SET: &[u8] =
        tonic::include_file_descriptor_set!("payment_descriptor");
}
