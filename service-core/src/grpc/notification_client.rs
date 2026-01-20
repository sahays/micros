//! Notification service gRPC client for service-to-service communication.

use std::collections::HashMap;
use std::time::Duration;
use tonic::Request;
use tonic::transport::{Channel, Endpoint};

use super::proto::notification::notification_service_client::NotificationServiceClient;
use super::proto::notification::{
    GetNotificationRequest, GetNotificationResponse, ListNotificationsRequest,
    ListNotificationsResponse, NotificationChannel, NotificationStatus, PushPlatform,
    SendBatchRequest, SendBatchResponse, SendEmailRequest, SendEmailResponse, SendPushRequest,
    SendPushResponse, SendSmsRequest, SendSmsResponse,
};

/// Configuration for the notification service client.
#[derive(Clone, Debug)]
pub struct NotificationClientConfig {
    /// The gRPC endpoint of the notification service (e.g., "http://notification-service:50052").
    pub endpoint: String,
    /// Connection timeout.
    pub connect_timeout: Duration,
    /// Request timeout.
    pub request_timeout: Duration,
}

impl Default for NotificationClientConfig {
    fn default() -> Self {
        Self {
            endpoint: "http://localhost:50052".to_string(),
            connect_timeout: Duration::from_secs(5),
            request_timeout: Duration::from_secs(30),
        }
    }
}

/// Notification service client for calling notification-service via gRPC.
#[derive(Clone)]
pub struct NotificationClient {
    client: NotificationServiceClient<Channel>,
}

impl NotificationClient {
    /// Create a new notification client with the given configuration.
    pub async fn new(config: NotificationClientConfig) -> Result<Self, tonic::transport::Error> {
        let channel = Endpoint::from_shared(config.endpoint)?
            .connect_timeout(config.connect_timeout)
            .timeout(config.request_timeout)
            .connect()
            .await?;

        Ok(Self {
            client: NotificationServiceClient::new(channel),
        })
    }

    /// Create a new notification client connecting to the specified endpoint.
    pub async fn connect(endpoint: &str) -> Result<Self, tonic::transport::Error> {
        Self::new(NotificationClientConfig {
            endpoint: endpoint.to_string(),
            ..Default::default()
        })
        .await
    }

    // =========================================================================
    // Email
    // =========================================================================

    /// Send an email notification.
    #[allow(clippy::too_many_arguments)]
    pub async fn send_email(
        &mut self,
        to: String,
        subject: String,
        body_text: Option<String>,
        body_html: Option<String>,
        from_name: Option<String>,
        reply_to: Option<String>,
        metadata: HashMap<String, String>,
    ) -> Result<SendEmailResponse, tonic::Status> {
        let request = Request::new(SendEmailRequest {
            to,
            subject,
            body_text,
            body_html,
            from_name,
            reply_to,
            metadata,
        });
        let response = self.client.send_email(request).await?;
        Ok(response.into_inner())
    }

    /// Send an email notification with minimal parameters.
    pub async fn send_email_simple(
        &mut self,
        to: String,
        subject: String,
        body: String,
    ) -> Result<SendEmailResponse, tonic::Status> {
        self.send_email(to, subject, Some(body), None, None, None, HashMap::new())
            .await
    }

    // =========================================================================
    // SMS
    // =========================================================================

    /// Send an SMS notification.
    pub async fn send_sms(
        &mut self,
        to: String,
        body: String,
        metadata: HashMap<String, String>,
    ) -> Result<SendSmsResponse, tonic::Status> {
        let request = Request::new(SendSmsRequest { to, body, metadata });
        let response = self.client.send_sms(request).await?;
        Ok(response.into_inner())
    }

    /// Send an SMS notification with minimal parameters.
    pub async fn send_sms_simple(
        &mut self,
        to: String,
        body: String,
    ) -> Result<SendSmsResponse, tonic::Status> {
        self.send_sms(to, body, HashMap::new()).await
    }

    // =========================================================================
    // Push
    // =========================================================================

    /// Send a push notification.
    pub async fn send_push(
        &mut self,
        device_token: String,
        platform: PushPlatform,
        title: String,
        body: String,
        data: HashMap<String, String>,
        metadata: HashMap<String, String>,
    ) -> Result<SendPushResponse, tonic::Status> {
        let request = Request::new(SendPushRequest {
            device_token,
            platform: platform as i32,
            title,
            body,
            data,
            metadata,
        });
        let response = self.client.send_push(request).await?;
        Ok(response.into_inner())
    }

    /// Send a push notification with minimal parameters.
    pub async fn send_push_simple(
        &mut self,
        device_token: String,
        platform: PushPlatform,
        title: String,
        body: String,
    ) -> Result<SendPushResponse, tonic::Status> {
        self.send_push(
            device_token,
            platform,
            title,
            body,
            HashMap::new(),
            HashMap::new(),
        )
        .await
    }

    // =========================================================================
    // Batch
    // =========================================================================

    /// Send multiple notifications in a batch.
    pub async fn send_batch(
        &mut self,
        request: SendBatchRequest,
    ) -> Result<SendBatchResponse, tonic::Status> {
        let response = self.client.send_batch(Request::new(request)).await?;
        Ok(response.into_inner())
    }

    // =========================================================================
    // Status & Query
    // =========================================================================

    /// Get the status of a notification by ID.
    pub async fn get_notification(
        &mut self,
        notification_id: String,
    ) -> Result<GetNotificationResponse, tonic::Status> {
        let request = Request::new(GetNotificationRequest { notification_id });
        let response = self.client.get_notification(request).await?;
        Ok(response.into_inner())
    }

    /// List notifications with optional filters.
    #[allow(clippy::too_many_arguments)]
    pub async fn list_notifications(
        &mut self,
        channel: Option<NotificationChannel>,
        status: Option<NotificationStatus>,
        recipient: Option<String>,
        user_id: Option<String>,
        tenant_id: Option<String>,
        page_size: i32,
        page_token: Option<String>,
    ) -> Result<ListNotificationsResponse, tonic::Status> {
        let request = Request::new(ListNotificationsRequest {
            channel: channel.map(|c| c as i32),
            status: status.map(|s| s as i32),
            recipient,
            user_id,
            tenant_id,
            page_size,
            page_token,
        });
        let response = self.client.list_notifications(request).await?;
        Ok(response.into_inner())
    }
}

// Re-export notification proto types for convenience
pub use super::proto::notification::{
    BatchNotification, BatchNotificationResult, Notification as NotificationProto,
    NotificationChannel as NotificationChannelProto, NotificationStatus as NotificationStatusProto,
    PushPlatform as PushPlatformProto,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notification_client_config_default() {
        let config = NotificationClientConfig::default();
        assert_eq!(config.endpoint, "http://localhost:50052");
        assert_eq!(config.connect_timeout, Duration::from_secs(5));
        assert_eq!(config.request_timeout, Duration::from_secs(30));
    }
}
