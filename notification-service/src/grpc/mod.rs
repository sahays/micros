pub mod notification_service;

pub use notification_service::NotificationGrpcService;

// Include generated proto code
pub mod proto {
    tonic::include_proto!("micros.notification.v1");

    pub const FILE_DESCRIPTOR_SET: &[u8] =
        tonic::include_file_descriptor_set!("notification_descriptor");
}
