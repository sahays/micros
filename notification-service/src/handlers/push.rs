use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use validator::Validate;

use crate::models::{Notification, NotificationStatus, PushPlatform};
use crate::services::{ProviderError, PushMessage};
use crate::startup::AppState;
use service_core::error::AppError;

#[derive(Debug, Deserialize, Validate)]
pub struct SendPushRequest {
    #[validate(length(min = 1, message = "Device token cannot be empty"))]
    pub device_token: String,
    pub platform: PushPlatform,
    #[validate(length(min = 1, message = "Title cannot be empty"))]
    pub title: String,
    #[validate(length(min = 1, message = "Body cannot be empty"))]
    pub body: String,
    pub data: Option<HashMap<String, String>>,
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Serialize)]
pub struct SendPushResponse {
    pub notification_id: String,
    pub status: String,
    pub channel: String,
}

#[tracing::instrument(skip(state, request))]
pub async fn send_push(
    State(state): State<AppState>,
    Json(request): Json<SendPushRequest>,
) -> Result<(StatusCode, Json<SendPushResponse>), AppError> {
    request.validate()?;

    // Create notification record
    let mut notification = Notification::new_push(
        request.device_token.clone(),
        request.platform.clone(),
        request.title.clone(),
        request.body.clone(),
        request.data.clone(),
        request.metadata.clone(),
    );

    let notification_id = notification.notification_id.clone();

    // Save to database
    state.db.insert(&notification).await?;

    // Send push notification
    let push_message = PushMessage {
        device_token: request.device_token,
        platform: request.platform,
        title: request.title,
        body: request.body,
        data: request.data,
    };

    match state.push_provider.send(&push_message).await {
        Ok(response) => {
            notification.mark_sent(response.provider_id.clone());
            state
                .db
                .update_status(
                    &notification_id,
                    NotificationStatus::Sent,
                    response.provider_id.as_deref(),
                    None,
                )
                .await?;

            tracing::info!(
                notification_id = %notification_id,
                "Push notification sent successfully"
            );
        }
        Err(ProviderError::NotEnabled(msg)) => {
            // Provider not enabled - mark as sent (mock behavior)
            tracing::warn!(
                notification_id = %notification_id,
                "Push provider not enabled: {}. Marking as sent.",
                msg
            );
            notification.mark_sent(Some("mock".to_string()));
            state
                .db
                .update_status(
                    &notification_id,
                    NotificationStatus::Sent,
                    Some("mock"),
                    None,
                )
                .await?;
        }
        Err(e) => {
            let error_msg = e.to_string();
            notification.mark_failed(error_msg.clone());
            state
                .db
                .update_status(
                    &notification_id,
                    NotificationStatus::Failed,
                    None,
                    Some(&error_msg),
                )
                .await?;

            tracing::error!(
                notification_id = %notification_id,
                error = %error_msg,
                "Failed to send push notification"
            );

            return Err(AppError::InternalError(anyhow::anyhow!(error_msg)));
        }
    }

    Ok((
        StatusCode::ACCEPTED,
        Json(SendPushResponse {
            notification_id,
            status: notification.status.to_string(),
            channel: "push".to_string(),
        }),
    ))
}
