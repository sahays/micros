use crate::grpc::proto::{
    notification_service_server::NotificationService, BatchNotification, BatchNotificationResult,
    GetNotificationRequest, GetNotificationResponse, ListNotificationsRequest,
    ListNotificationsResponse, Notification as ProtoNotification, NotificationChannel,
    NotificationStatus as ProtoNotificationStatus, PushPlatform as ProtoPushPlatform,
    SendBatchRequest, SendBatchResponse, SendEmailRequest, SendEmailResponse, SendPushRequest,
    SendPushResponse, SendSmsRequest, SendSmsResponse,
};
use crate::models::{Channel, Notification, NotificationStatus, PushPlatform};
use crate::services::{EmailMessage, ProviderError, PushMessage, SmsMessage};
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
fn channel_to_proto(channel: &Channel) -> i32 {
    match channel {
        Channel::Email => NotificationChannel::Email as i32,
        Channel::Sms => NotificationChannel::Sms as i32,
        Channel::Push => NotificationChannel::Push as i32,
    }
}

fn status_to_proto(status: &NotificationStatus) -> i32 {
    match status {
        NotificationStatus::Queued => ProtoNotificationStatus::Queued as i32,
        NotificationStatus::Sent => ProtoNotificationStatus::Sent as i32,
        NotificationStatus::Delivered => ProtoNotificationStatus::Delivered as i32,
        NotificationStatus::Failed => ProtoNotificationStatus::Failed as i32,
    }
}

fn proto_to_channel(channel: i32) -> Option<Channel> {
    match NotificationChannel::try_from(channel) {
        Ok(NotificationChannel::Email) => Some(Channel::Email),
        Ok(NotificationChannel::Sms) => Some(Channel::Sms),
        Ok(NotificationChannel::Push) => Some(Channel::Push),
        _ => None,
    }
}

fn proto_to_status(status: i32) -> Option<NotificationStatus> {
    match ProtoNotificationStatus::try_from(status) {
        Ok(ProtoNotificationStatus::Queued) => Some(NotificationStatus::Queued),
        Ok(ProtoNotificationStatus::Sent) => Some(NotificationStatus::Sent),
        Ok(ProtoNotificationStatus::Delivered) => Some(NotificationStatus::Delivered),
        Ok(ProtoNotificationStatus::Failed) => Some(NotificationStatus::Failed),
        _ => None,
    }
}

fn proto_to_push_platform(platform: i32) -> Option<PushPlatform> {
    match ProtoPushPlatform::try_from(platform) {
        Ok(ProtoPushPlatform::Fcm) => Some(PushPlatform::Fcm),
        Ok(ProtoPushPlatform::Apns) => Some(PushPlatform::Apns),
        _ => None,
    }
}

fn datetime_to_timestamp(dt: chrono::DateTime<chrono::Utc>) -> Timestamp {
    Timestamp {
        seconds: dt.timestamp(),
        nanos: dt.timestamp_subsec_nanos() as i32,
    }
}

fn notification_to_proto(n: &Notification) -> ProtoNotification {
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
        let req = request.into_inner();

        // Validation
        if req.to.is_empty() {
            return Err(Status::invalid_argument("Recipient email is required"));
        }
        if req.subject.is_empty() {
            return Err(Status::invalid_argument("Subject is required"));
        }
        if req.body_html.is_none() && req.body_text.is_none() {
            return Err(Status::invalid_argument(
                "At least one of body_html or body_text is required",
            ));
        }

        // Create notification record
        let mut notification = Notification::new_email(
            req.to.clone(),
            req.subject.clone(),
            req.body_text.clone(),
            req.body_html.clone(),
            req.from_name.clone(),
            req.reply_to.clone(),
            req.metadata.clone(),
        );

        let notification_id = notification.notification_id.clone();

        // Save to database
        self.state
            .db
            .insert(&notification)
            .await
            .map_err(|e| Status::internal(format!("Database error: {}", e)))?;

        // Send email
        let email_message = EmailMessage {
            to: req.to,
            subject: req.subject,
            body_text: req.body_text,
            body_html: req.body_html,
            from_name: req.from_name,
            reply_to: req.reply_to,
        };

        match self.state.email_provider.send(&email_message).await {
            Ok(response) => {
                notification.mark_sent(response.provider_id.clone());
                let _ = self
                    .state
                    .db
                    .update_status(
                        &notification_id,
                        NotificationStatus::Sent,
                        response.provider_id.as_deref(),
                        None,
                    )
                    .await;

                tracing::info!(notification_id = %notification_id, "Email sent successfully");
            }
            Err(ProviderError::NotEnabled(msg)) => {
                tracing::warn!(
                    notification_id = %notification_id,
                    "Email provider not enabled: {}. Marking as sent.",
                    msg
                );
                notification.mark_sent(Some("mock".to_string()));
                let _ = self
                    .state
                    .db
                    .update_status(
                        &notification_id,
                        NotificationStatus::Sent,
                        Some("mock"),
                        None,
                    )
                    .await;
            }
            Err(e) => {
                let error_msg = e.to_string();
                notification.mark_failed(error_msg.clone());
                let _ = self
                    .state
                    .db
                    .update_status(
                        &notification_id,
                        NotificationStatus::Failed,
                        None,
                        Some(&error_msg),
                    )
                    .await;

                tracing::error!(
                    notification_id = %notification_id,
                    error = %error_msg,
                    "Failed to send email"
                );

                return Err(Status::internal(format!(
                    "Failed to send email: {}",
                    error_msg
                )));
            }
        }

        Ok(Response::new(SendEmailResponse {
            notification_id,
            status: notification.status.to_string(),
            channel: "email".to_string(),
        }))
    }

    #[tracing::instrument(skip(self, request))]
    async fn send_sms(
        &self,
        request: Request<SendSmsRequest>,
    ) -> Result<Response<SendSmsResponse>, Status> {
        let req = request.into_inner();

        // Validation
        if req.to.is_empty() || req.to.len() < 10 {
            return Err(Status::invalid_argument(
                "Phone number must be at least 10 characters",
            ));
        }
        if req.body.is_empty() || req.body.len() > 1600 {
            return Err(Status::invalid_argument(
                "SMS body must be between 1 and 1600 characters",
            ));
        }

        // Create notification record
        let mut notification =
            Notification::new_sms(req.to.clone(), req.body.clone(), req.metadata.clone());

        let notification_id = notification.notification_id.clone();

        // Save to database
        self.state
            .db
            .insert(&notification)
            .await
            .map_err(|e| Status::internal(format!("Database error: {}", e)))?;

        // Send SMS
        let sms_message = SmsMessage {
            to: req.to,
            body: req.body,
        };

        match self.state.sms_provider.send(&sms_message).await {
            Ok(response) => {
                notification.mark_sent(response.provider_id.clone());
                let _ = self
                    .state
                    .db
                    .update_status(
                        &notification_id,
                        NotificationStatus::Sent,
                        response.provider_id.as_deref(),
                        None,
                    )
                    .await;

                tracing::info!(notification_id = %notification_id, "SMS sent successfully");
            }
            Err(ProviderError::NotEnabled(msg)) => {
                tracing::warn!(
                    notification_id = %notification_id,
                    "SMS provider not enabled: {}. Marking as sent.",
                    msg
                );
                notification.mark_sent(Some("mock".to_string()));
                let _ = self
                    .state
                    .db
                    .update_status(
                        &notification_id,
                        NotificationStatus::Sent,
                        Some("mock"),
                        None,
                    )
                    .await;
            }
            Err(e) => {
                let error_msg = e.to_string();
                notification.mark_failed(error_msg.clone());
                let _ = self
                    .state
                    .db
                    .update_status(
                        &notification_id,
                        NotificationStatus::Failed,
                        None,
                        Some(&error_msg),
                    )
                    .await;

                tracing::error!(
                    notification_id = %notification_id,
                    error = %error_msg,
                    "Failed to send SMS"
                );

                return Err(Status::internal(format!(
                    "Failed to send SMS: {}",
                    error_msg
                )));
            }
        }

        Ok(Response::new(SendSmsResponse {
            notification_id,
            status: notification.status.to_string(),
            channel: "sms".to_string(),
        }))
    }

    #[tracing::instrument(skip(self, request))]
    async fn send_push(
        &self,
        request: Request<SendPushRequest>,
    ) -> Result<Response<SendPushResponse>, Status> {
        let req = request.into_inner();

        // Validation
        if req.device_token.is_empty() {
            return Err(Status::invalid_argument("Device token is required"));
        }
        if req.title.is_empty() {
            return Err(Status::invalid_argument("Title is required"));
        }
        if req.body.is_empty() {
            return Err(Status::invalid_argument("Body is required"));
        }

        let platform = proto_to_push_platform(req.platform)
            .ok_or_else(|| Status::invalid_argument("Invalid push platform"))?;

        // Create notification record
        let mut notification = Notification::new_push(
            req.device_token.clone(),
            platform.clone(),
            req.title.clone(),
            req.body.clone(),
            if req.data.is_empty() {
                None
            } else {
                Some(req.data.clone())
            },
            req.metadata.clone(),
        );

        let notification_id = notification.notification_id.clone();

        // Save to database
        self.state
            .db
            .insert(&notification)
            .await
            .map_err(|e| Status::internal(format!("Database error: {}", e)))?;

        // Send push notification
        let push_message = PushMessage {
            device_token: req.device_token,
            platform,
            title: req.title,
            body: req.body,
            data: if req.data.is_empty() {
                None
            } else {
                Some(req.data)
            },
        };

        match self.state.push_provider.send(&push_message).await {
            Ok(response) => {
                notification.mark_sent(response.provider_id.clone());
                let _ = self
                    .state
                    .db
                    .update_status(
                        &notification_id,
                        NotificationStatus::Sent,
                        response.provider_id.as_deref(),
                        None,
                    )
                    .await;

                tracing::info!(notification_id = %notification_id, "Push notification sent successfully");
            }
            Err(ProviderError::NotEnabled(msg)) => {
                tracing::warn!(
                    notification_id = %notification_id,
                    "Push provider not enabled: {}. Marking as sent.",
                    msg
                );
                notification.mark_sent(Some("mock".to_string()));
                let _ = self
                    .state
                    .db
                    .update_status(
                        &notification_id,
                        NotificationStatus::Sent,
                        Some("mock"),
                        None,
                    )
                    .await;
            }
            Err(e) => {
                let error_msg = e.to_string();
                notification.mark_failed(error_msg.clone());
                let _ = self
                    .state
                    .db
                    .update_status(
                        &notification_id,
                        NotificationStatus::Failed,
                        None,
                        Some(&error_msg),
                    )
                    .await;

                tracing::error!(
                    notification_id = %notification_id,
                    error = %error_msg,
                    "Failed to send push notification"
                );

                return Err(Status::internal(format!(
                    "Failed to send push notification: {}",
                    error_msg
                )));
            }
        }

        Ok(Response::new(SendPushResponse {
            notification_id,
            status: notification.status.to_string(),
            channel: "push".to_string(),
        }))
    }

    #[tracing::instrument(skip(self, request))]
    async fn send_batch(
        &self,
        request: Request<SendBatchRequest>,
    ) -> Result<Response<SendBatchResponse>, Status> {
        let req = request.into_inner();

        if req.notifications.is_empty() {
            return Err(Status::invalid_argument(
                "At least one notification must be provided",
            ));
        }

        if req.notifications.len() > 100 {
            return Err(Status::invalid_argument(
                "Maximum 100 notifications per batch",
            ));
        }

        let batch_id = uuid::Uuid::new_v4().to_string();
        let mut results = Vec::with_capacity(req.notifications.len());

        for notification_request in req.notifications {
            let result = self.process_batch_notification(&notification_request).await;
            results.push(result);
        }

        Ok(Response::new(SendBatchResponse { batch_id, results }))
    }

    #[tracing::instrument(skip(self, request))]
    async fn get_notification(
        &self,
        request: Request<GetNotificationRequest>,
    ) -> Result<Response<GetNotificationResponse>, Status> {
        let req = request.into_inner();

        if req.notification_id.is_empty() {
            return Err(Status::invalid_argument("Notification ID is required"));
        }

        let notification = self
            .state
            .db
            .find_by_id(&req.notification_id)
            .await
            .map_err(|e| Status::internal(format!("Database error: {}", e)))?;

        match notification {
            Some(n) => Ok(Response::new(GetNotificationResponse {
                notification: Some(notification_to_proto(&n)),
            })),
            None => Err(Status::not_found(format!(
                "Notification not found: {}",
                req.notification_id
            ))),
        }
    }

    #[tracing::instrument(skip(self, request))]
    async fn list_notifications(
        &self,
        request: Request<ListNotificationsRequest>,
    ) -> Result<Response<ListNotificationsResponse>, Status> {
        let req = request.into_inner();

        // Parse channel filter
        let channel = req.channel.and_then(proto_to_channel);

        // Parse status filter
        let status = req.status.and_then(proto_to_status);

        // Clamp page size
        let limit = (req.page_size as i64).clamp(1, 100);
        let offset = req
            .page_token
            .as_ref()
            .and_then(|t| t.parse::<u64>().ok())
            .unwrap_or(0);

        let notifications = self
            .state
            .db
            .list(channel, status, limit, offset)
            .await
            .map_err(|e| Status::internal(format!("Database error: {}", e)))?;

        let proto_notifications: Vec<ProtoNotification> =
            notifications.iter().map(notification_to_proto).collect();

        let next_offset = offset + proto_notifications.len() as u64;
        let next_page_token = if proto_notifications.len() == limit as usize {
            Some(next_offset.to_string())
        } else {
            None
        };

        Ok(Response::new(ListNotificationsResponse {
            notifications: proto_notifications,
            next_page_token,
            total_count: None, // Could add count query if needed
        }))
    }
}

impl NotificationGrpcService {
    async fn process_batch_notification(
        &self,
        notification: &BatchNotification,
    ) -> BatchNotificationResult {
        let channel = NotificationChannel::try_from(notification.channel);

        match channel {
            Ok(NotificationChannel::Email) => self.process_batch_email(notification).await,
            Ok(NotificationChannel::Sms) => self.process_batch_sms(notification).await,
            Ok(NotificationChannel::Push) => self.process_batch_push(notification).await,
            _ => BatchNotificationResult {
                notification_id: String::new(),
                status: ProtoNotificationStatus::Failed as i32,
                error: Some("Invalid or unspecified channel".to_string()),
            },
        }
    }

    async fn process_batch_email(&self, batch: &BatchNotification) -> BatchNotificationResult {
        let email = match &batch.email {
            Some(e) => e,
            None => {
                return BatchNotificationResult {
                    notification_id: String::new(),
                    status: ProtoNotificationStatus::Failed as i32,
                    error: Some("Email request is required for email channel".to_string()),
                }
            }
        };

        if email.subject.is_empty() {
            return BatchNotificationResult {
                notification_id: String::new(),
                status: ProtoNotificationStatus::Failed as i32,
                error: Some("Subject is required for email".to_string()),
            };
        }

        if email.body_html.is_none() && email.body_text.is_none() {
            return BatchNotificationResult {
                notification_id: String::new(),
                status: ProtoNotificationStatus::Failed as i32,
                error: Some("At least one of body_html or body_text is required".to_string()),
            };
        }

        let mut notification = Notification::new_email(
            email.to.clone(),
            email.subject.clone(),
            email.body_text.clone(),
            email.body_html.clone(),
            email.from_name.clone(),
            email.reply_to.clone(),
            email.metadata.clone(),
        );

        let notification_id = notification.notification_id.clone();

        if let Err(e) = self.state.db.insert(&notification).await {
            return BatchNotificationResult {
                notification_id,
                status: ProtoNotificationStatus::Failed as i32,
                error: Some(format!("Database error: {}", e)),
            };
        }

        let email_message = EmailMessage {
            to: email.to.clone(),
            subject: email.subject.clone(),
            body_text: email.body_text.clone(),
            body_html: email.body_html.clone(),
            from_name: email.from_name.clone(),
            reply_to: email.reply_to.clone(),
        };

        match self.state.email_provider.send(&email_message).await {
            Ok(response) => {
                notification.mark_sent(response.provider_id.clone());
                let _ = self
                    .state
                    .db
                    .update_status(
                        &notification_id,
                        NotificationStatus::Sent,
                        response.provider_id.as_deref(),
                        None,
                    )
                    .await;

                BatchNotificationResult {
                    notification_id,
                    status: ProtoNotificationStatus::Sent as i32,
                    error: None,
                }
            }
            Err(ProviderError::NotEnabled(_)) => {
                let _ = self
                    .state
                    .db
                    .update_status(
                        &notification_id,
                        NotificationStatus::Sent,
                        Some("mock"),
                        None,
                    )
                    .await;

                BatchNotificationResult {
                    notification_id,
                    status: ProtoNotificationStatus::Sent as i32,
                    error: None,
                }
            }
            Err(e) => {
                let error_msg = e.to_string();
                let _ = self
                    .state
                    .db
                    .update_status(
                        &notification_id,
                        NotificationStatus::Failed,
                        None,
                        Some(&error_msg),
                    )
                    .await;

                BatchNotificationResult {
                    notification_id,
                    status: ProtoNotificationStatus::Failed as i32,
                    error: Some(error_msg),
                }
            }
        }
    }

    async fn process_batch_sms(&self, batch: &BatchNotification) -> BatchNotificationResult {
        let sms = match &batch.sms {
            Some(s) => s,
            None => {
                return BatchNotificationResult {
                    notification_id: String::new(),
                    status: ProtoNotificationStatus::Failed as i32,
                    error: Some("SMS request is required for SMS channel".to_string()),
                }
            }
        };

        if sms.body.is_empty() {
            return BatchNotificationResult {
                notification_id: String::new(),
                status: ProtoNotificationStatus::Failed as i32,
                error: Some("Body is required for SMS".to_string()),
            };
        }

        let mut notification =
            Notification::new_sms(sms.to.clone(), sms.body.clone(), sms.metadata.clone());

        let notification_id = notification.notification_id.clone();

        if let Err(e) = self.state.db.insert(&notification).await {
            return BatchNotificationResult {
                notification_id,
                status: ProtoNotificationStatus::Failed as i32,
                error: Some(format!("Database error: {}", e)),
            };
        }

        let sms_message = SmsMessage {
            to: sms.to.clone(),
            body: sms.body.clone(),
        };

        match self.state.sms_provider.send(&sms_message).await {
            Ok(response) => {
                notification.mark_sent(response.provider_id.clone());
                let _ = self
                    .state
                    .db
                    .update_status(
                        &notification_id,
                        NotificationStatus::Sent,
                        response.provider_id.as_deref(),
                        None,
                    )
                    .await;

                BatchNotificationResult {
                    notification_id,
                    status: ProtoNotificationStatus::Sent as i32,
                    error: None,
                }
            }
            Err(ProviderError::NotEnabled(_)) => {
                let _ = self
                    .state
                    .db
                    .update_status(
                        &notification_id,
                        NotificationStatus::Sent,
                        Some("mock"),
                        None,
                    )
                    .await;

                BatchNotificationResult {
                    notification_id,
                    status: ProtoNotificationStatus::Sent as i32,
                    error: None,
                }
            }
            Err(e) => {
                let error_msg = e.to_string();
                let _ = self
                    .state
                    .db
                    .update_status(
                        &notification_id,
                        NotificationStatus::Failed,
                        None,
                        Some(&error_msg),
                    )
                    .await;

                BatchNotificationResult {
                    notification_id,
                    status: ProtoNotificationStatus::Failed as i32,
                    error: Some(error_msg),
                }
            }
        }
    }

    async fn process_batch_push(&self, batch: &BatchNotification) -> BatchNotificationResult {
        let push = match &batch.push {
            Some(p) => p,
            None => {
                return BatchNotificationResult {
                    notification_id: String::new(),
                    status: ProtoNotificationStatus::Failed as i32,
                    error: Some("Push request is required for push channel".to_string()),
                }
            }
        };

        if push.device_token.is_empty() {
            return BatchNotificationResult {
                notification_id: String::new(),
                status: ProtoNotificationStatus::Failed as i32,
                error: Some("Device token is required for push".to_string()),
            };
        }

        if push.title.is_empty() {
            return BatchNotificationResult {
                notification_id: String::new(),
                status: ProtoNotificationStatus::Failed as i32,
                error: Some("Title is required for push".to_string()),
            };
        }

        if push.body.is_empty() {
            return BatchNotificationResult {
                notification_id: String::new(),
                status: ProtoNotificationStatus::Failed as i32,
                error: Some("Body is required for push".to_string()),
            };
        }

        let platform = match proto_to_push_platform(push.platform) {
            Some(p) => p,
            None => {
                return BatchNotificationResult {
                    notification_id: String::new(),
                    status: ProtoNotificationStatus::Failed as i32,
                    error: Some("Invalid push platform".to_string()),
                }
            }
        };

        let mut notification = Notification::new_push(
            push.device_token.clone(),
            platform.clone(),
            push.title.clone(),
            push.body.clone(),
            if push.data.is_empty() {
                None
            } else {
                Some(push.data.clone())
            },
            push.metadata.clone(),
        );

        let notification_id = notification.notification_id.clone();

        if let Err(e) = self.state.db.insert(&notification).await {
            return BatchNotificationResult {
                notification_id,
                status: ProtoNotificationStatus::Failed as i32,
                error: Some(format!("Database error: {}", e)),
            };
        }

        let push_message = PushMessage {
            device_token: push.device_token.clone(),
            platform,
            title: push.title.clone(),
            body: push.body.clone(),
            data: if push.data.is_empty() {
                None
            } else {
                Some(push.data.clone())
            },
        };

        match self.state.push_provider.send(&push_message).await {
            Ok(response) => {
                notification.mark_sent(response.provider_id.clone());
                let _ = self
                    .state
                    .db
                    .update_status(
                        &notification_id,
                        NotificationStatus::Sent,
                        response.provider_id.as_deref(),
                        None,
                    )
                    .await;

                BatchNotificationResult {
                    notification_id,
                    status: ProtoNotificationStatus::Sent as i32,
                    error: None,
                }
            }
            Err(ProviderError::NotEnabled(_)) => {
                let _ = self
                    .state
                    .db
                    .update_status(
                        &notification_id,
                        NotificationStatus::Sent,
                        Some("mock"),
                        None,
                    )
                    .await;

                BatchNotificationResult {
                    notification_id,
                    status: ProtoNotificationStatus::Sent as i32,
                    error: None,
                }
            }
            Err(e) => {
                let error_msg = e.to_string();
                let _ = self
                    .state
                    .db
                    .update_status(
                        &notification_id,
                        NotificationStatus::Failed,
                        None,
                        Some(&error_msg),
                    )
                    .await;

                BatchNotificationResult {
                    notification_id,
                    status: ProtoNotificationStatus::Failed as i32,
                    error: Some(error_msg),
                }
            }
        }
    }
}
