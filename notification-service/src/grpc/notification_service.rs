use crate::grpc::proto::{
    notification_service_server::NotificationService, GetNotificationRequest,
    GetNotificationResponse, ListNotificationsRequest, ListNotificationsResponse,
    Notification as ProtoNotification, NotificationChannel,
    NotificationStatus as ProtoNotificationStatus, PushPlatform as ProtoPushPlatform,
    SendBatchRequest, SendBatchResponse, SendEmailRequest, SendEmailResponse, SendPushRequest,
    SendPushResponse, SendSmsRequest, SendSmsResponse,
};
use crate::grpc::{batch, queries, send};
use crate::models::{Channel, Notification, NotificationStatus, PushPlatform};
use crate::startup::AppState;
use prost_types::Timestamp;
use tonic::{Request, Response, Status};

pub struct NotificationGrpcService {
    state: AppState,
}

impl NotificationGrpcService {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }
}

// Conversion helpers
pub fn channel_to_proto(channel: &Channel) -> i32 {
    match channel {
        Channel::Email => NotificationChannel::Email as i32,
        Channel::Sms => NotificationChannel::Sms as i32,
        Channel::Push => NotificationChannel::Push as i32,
    }
}

pub fn status_to_proto(status: &NotificationStatus) -> i32 {
    match status {
        NotificationStatus::Queued => ProtoNotificationStatus::Queued as i32,
        NotificationStatus::Sent => ProtoNotificationStatus::Sent as i32,
        NotificationStatus::Delivered => ProtoNotificationStatus::Delivered as i32,
        NotificationStatus::Failed => ProtoNotificationStatus::Failed as i32,
    }
}

pub fn proto_to_channel(channel: i32) -> Option<Channel> {
    match NotificationChannel::try_from(channel) {
        Ok(NotificationChannel::Email) => Some(Channel::Email),
        Ok(NotificationChannel::Sms) => Some(Channel::Sms),
        Ok(NotificationChannel::Push) => Some(Channel::Push),
        _ => None,
    }
}

pub fn proto_to_status(status: i32) -> Option<NotificationStatus> {
    match ProtoNotificationStatus::try_from(status) {
        Ok(ProtoNotificationStatus::Queued) => Some(NotificationStatus::Queued),
        Ok(ProtoNotificationStatus::Sent) => Some(NotificationStatus::Sent),
        Ok(ProtoNotificationStatus::Delivered) => Some(NotificationStatus::Delivered),
        Ok(ProtoNotificationStatus::Failed) => Some(NotificationStatus::Failed),
        _ => None,
    }
}

pub fn proto_to_push_platform(platform: i32) -> Option<PushPlatform> {
    match ProtoPushPlatform::try_from(platform) {
        Ok(ProtoPushPlatform::Fcm) => Some(PushPlatform::Fcm),
        Ok(ProtoPushPlatform::Apns) => Some(PushPlatform::Apns),
        _ => None,
    }
}

pub fn datetime_to_timestamp(dt: chrono::DateTime<chrono::Utc>) -> Timestamp {
    Timestamp {
        seconds: dt.timestamp(),
        nanos: dt.timestamp_subsec_nanos() as i32,
    }
}

pub fn notification_to_proto(n: &Notification) -> ProtoNotification {
    ProtoNotification {
        notification_id: n.notification_id.clone(),
        channel: channel_to_proto(&n.channel),
        status: status_to_proto(&n.status),
        recipient: n.recipient.clone(),
        subject: n.subject.clone(),
        body: n.body.clone(),
        metadata: n.metadata.clone(),
        provider_id: n.provider_id.clone(),
        error_message: n.error_message.clone(),
        created_at: Some(datetime_to_timestamp(n.created_utc)),
        sent_at: n.sent_utc.map(datetime_to_timestamp),
        delivered_at: n.delivered_utc.map(datetime_to_timestamp),
        failed_at: n.failed_utc.map(datetime_to_timestamp),
    }
}

#[tonic::async_trait]
impl NotificationService for NotificationGrpcService {
    #[tracing::instrument(skip(self, request))]
    async fn send_email(
        &self,
        request: Request<SendEmailRequest>,
    ) -> Result<Response<SendEmailResponse>, Status> {
        send::send_email(&self.state, request).await
    }

    #[tracing::instrument(skip(self, request))]
    async fn send_sms(
        &self,
        request: Request<SendSmsRequest>,
    ) -> Result<Response<SendSmsResponse>, Status> {
        send::send_sms(&self.state, request).await
    }

    #[tracing::instrument(skip(self, request))]
    async fn send_push(
        &self,
        request: Request<SendPushRequest>,
    ) -> Result<Response<SendPushResponse>, Status> {
        send::send_push(&self.state, request).await
    }

    #[tracing::instrument(skip(self, request))]
    async fn send_batch(
        &self,
        request: Request<SendBatchRequest>,
    ) -> Result<Response<SendBatchResponse>, Status> {
        batch::send_batch(&self.state, request).await
    }

    #[tracing::instrument(skip(self, request))]
    async fn get_notification(
        &self,
        request: Request<GetNotificationRequest>,
    ) -> Result<Response<GetNotificationResponse>, Status> {
        queries::get_notification(&self.state, request).await
    }

    #[tracing::instrument(skip(self, request))]
    async fn list_notifications(
        &self,
        request: Request<ListNotificationsRequest>,
    ) -> Result<Response<ListNotificationsResponse>, Status> {
        queries::list_notifications(&self.state, request).await
    }
}
