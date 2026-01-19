use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::models::{Channel, Notification, NotificationStatus, PushPlatform};
use crate::services::{EmailMessage, ProviderError, PushMessage, SmsMessage};
use crate::startup::AppState;
use service_core::error::AppError;

#[derive(Debug, Deserialize)]
pub struct BatchNotificationRequest {
    pub channel: Channel,
    pub to: String,
    // Email fields
    pub subject: Option<String>,
    pub body_html: Option<String>,
    pub body_text: Option<String>,
    pub from_name: Option<String>,
    pub reply_to: Option<String>,
    // SMS field
    pub body: Option<String>,
    // Push fields
    pub device_token: Option<String>,
    pub platform: Option<PushPlatform>,
    pub title: Option<String>,
    pub data: Option<HashMap<String, String>>,
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
pub struct SendBatchRequest {
    pub notifications: Vec<BatchNotificationRequest>,
}

#[derive(Debug, Serialize)]
pub struct BatchNotificationResult {
    pub notification_id: String,
    pub status: String,
    pub channel: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SendBatchResponse {
    pub batch_id: String,
    pub notifications: Vec<BatchNotificationResult>,
}

#[tracing::instrument(skip(state, request))]
pub async fn send_batch(
    State(state): State<AppState>,
    Json(request): Json<SendBatchRequest>,
) -> Result<(StatusCode, Json<SendBatchResponse>), AppError> {
    if request.notifications.is_empty() {
        return Err(AppError::BadRequest(anyhow::anyhow!(
            "At least one notification must be provided"
        )));
    }

    if request.notifications.len() > 100 {
        return Err(AppError::BadRequest(anyhow::anyhow!(
            "Maximum 100 notifications per batch"
        )));
    }

    let batch_id = uuid::Uuid::new_v4().to_string();
    let mut results = Vec::with_capacity(request.notifications.len());

    for notification_request in request.notifications {
        let result = process_batch_notification(&state, &notification_request).await;
        results.push(result);
    }

    Ok((
        StatusCode::ACCEPTED,
        Json(SendBatchResponse {
            batch_id,
            notifications: results,
        }),
    ))
}

async fn process_batch_notification(
    state: &AppState,
    request: &BatchNotificationRequest,
) -> BatchNotificationResult {
    match request.channel {
        Channel::Email => process_email_notification(state, request).await,
        Channel::Sms => process_sms_notification(state, request).await,
        Channel::Push => process_push_notification(state, request).await,
    }
}

async fn process_email_notification(
    state: &AppState,
    request: &BatchNotificationRequest,
) -> BatchNotificationResult {
    let subject = match &request.subject {
        Some(s) => s.clone(),
        None => {
            return BatchNotificationResult {
                notification_id: String::new(),
                status: "failed".to_string(),
                channel: "email".to_string(),
                error: Some("Subject is required for email".to_string()),
            };
        }
    };

    if request.body_html.is_none() && request.body_text.is_none() {
        return BatchNotificationResult {
            notification_id: String::new(),
            status: "failed".to_string(),
            channel: "email".to_string(),
            error: Some("At least one of body_html or body_text is required".to_string()),
        };
    }

    let mut notification = Notification::new_email(
        request.to.clone(),
        subject.clone(),
        request.body_text.clone(),
        request.body_html.clone(),
        request.from_name.clone(),
        request.reply_to.clone(),
        request.metadata.clone(),
    );

    let notification_id = notification.notification_id.clone();

    if let Err(e) = state.db.insert(&notification).await {
        return BatchNotificationResult {
            notification_id,
            status: "failed".to_string(),
            channel: "email".to_string(),
            error: Some(format!("Database error: {}", e)),
        };
    }

    let email_message = EmailMessage {
        to: request.to.clone(),
        subject,
        body_text: request.body_text.clone(),
        body_html: request.body_html.clone(),
        from_name: request.from_name.clone(),
        reply_to: request.reply_to.clone(),
    };

    match state.email_provider.send(&email_message).await {
        Ok(response) => {
            notification.mark_sent(response.provider_id.clone());
            let _ = state
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
                status: "sent".to_string(),
                channel: "email".to_string(),
                error: None,
            }
        }
        Err(ProviderError::NotEnabled(_)) => {
            let _ = state
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
                status: "sent".to_string(),
                channel: "email".to_string(),
                error: None,
            }
        }
        Err(e) => {
            let error_msg = e.to_string();
            let _ = state
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
                status: "failed".to_string(),
                channel: "email".to_string(),
                error: Some(error_msg),
            }
        }
    }
}

async fn process_sms_notification(
    state: &AppState,
    request: &BatchNotificationRequest,
) -> BatchNotificationResult {
    let body = match &request.body {
        Some(b) => b.clone(),
        None => {
            return BatchNotificationResult {
                notification_id: String::new(),
                status: "failed".to_string(),
                channel: "sms".to_string(),
                error: Some("Body is required for SMS".to_string()),
            };
        }
    };

    let mut notification =
        Notification::new_sms(request.to.clone(), body.clone(), request.metadata.clone());

    let notification_id = notification.notification_id.clone();

    if let Err(e) = state.db.insert(&notification).await {
        return BatchNotificationResult {
            notification_id,
            status: "failed".to_string(),
            channel: "sms".to_string(),
            error: Some(format!("Database error: {}", e)),
        };
    }

    let sms_message = SmsMessage {
        to: request.to.clone(),
        body,
    };

    match state.sms_provider.send(&sms_message).await {
        Ok(response) => {
            notification.mark_sent(response.provider_id.clone());
            let _ = state
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
                status: "sent".to_string(),
                channel: "sms".to_string(),
                error: None,
            }
        }
        Err(ProviderError::NotEnabled(_)) => {
            let _ = state
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
                status: "sent".to_string(),
                channel: "sms".to_string(),
                error: None,
            }
        }
        Err(e) => {
            let error_msg = e.to_string();
            let _ = state
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
                status: "failed".to_string(),
                channel: "sms".to_string(),
                error: Some(error_msg),
            }
        }
    }
}

async fn process_push_notification(
    state: &AppState,
    request: &BatchNotificationRequest,
) -> BatchNotificationResult {
    let device_token = match &request.device_token {
        Some(t) => t.clone(),
        None => {
            return BatchNotificationResult {
                notification_id: String::new(),
                status: "failed".to_string(),
                channel: "push".to_string(),
                error: Some("Device token is required for push".to_string()),
            };
        }
    };

    let platform = match &request.platform {
        Some(p) => p.clone(),
        None => {
            return BatchNotificationResult {
                notification_id: String::new(),
                status: "failed".to_string(),
                channel: "push".to_string(),
                error: Some("Platform is required for push".to_string()),
            };
        }
    };

    let title = match &request.title {
        Some(t) => t.clone(),
        None => {
            return BatchNotificationResult {
                notification_id: String::new(),
                status: "failed".to_string(),
                channel: "push".to_string(),
                error: Some("Title is required for push".to_string()),
            };
        }
    };

    let body = match &request.body {
        Some(b) => b.clone(),
        None => {
            return BatchNotificationResult {
                notification_id: String::new(),
                status: "failed".to_string(),
                channel: "push".to_string(),
                error: Some("Body is required for push".to_string()),
            };
        }
    };

    let mut notification = Notification::new_push(
        device_token.clone(),
        platform.clone(),
        title.clone(),
        body.clone(),
        request.data.clone(),
        request.metadata.clone(),
    );

    let notification_id = notification.notification_id.clone();

    if let Err(e) = state.db.insert(&notification).await {
        return BatchNotificationResult {
            notification_id,
            status: "failed".to_string(),
            channel: "push".to_string(),
            error: Some(format!("Database error: {}", e)),
        };
    }

    let push_message = PushMessage {
        device_token,
        platform,
        title,
        body,
        data: request.data.clone(),
    };

    match state.push_provider.send(&push_message).await {
        Ok(response) => {
            notification.mark_sent(response.provider_id.clone());
            let _ = state
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
                status: "sent".to_string(),
                channel: "push".to_string(),
                error: None,
            }
        }
        Err(ProviderError::NotEnabled(_)) => {
            let _ = state
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
                status: "sent".to_string(),
                channel: "push".to_string(),
                error: None,
            }
        }
        Err(e) => {
            let error_msg = e.to_string();
            let _ = state
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
                status: "failed".to_string(),
                channel: "push".to_string(),
                error: Some(error_msg),
            }
        }
    }
}
