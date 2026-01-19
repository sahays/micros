use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use validator::Validate;

use crate::models::{Notification, NotificationStatus};
use crate::services::{ProviderError, SmsMessage};
use crate::startup::AppState;
use service_core::error::AppError;

#[derive(Debug, Deserialize, Validate)]
pub struct SendSmsRequest {
    #[validate(length(min = 10, message = "Phone number must be at least 10 characters"))]
    pub to: String,
    #[validate(length(
        min = 1,
        max = 1600,
        message = "SMS body must be between 1 and 1600 characters"
    ))]
    pub body: String,
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Serialize)]
pub struct SendSmsResponse {
    pub notification_id: String,
    pub status: String,
    pub channel: String,
}

#[tracing::instrument(skip(state, request))]
pub async fn send_sms(
    State(state): State<AppState>,
    Json(request): Json<SendSmsRequest>,
) -> Result<(StatusCode, Json<SendSmsResponse>), AppError> {
    request.validate()?;

    // Create notification record
    let mut notification = Notification::new_sms(
        request.to.clone(),
        request.body.clone(),
        request.metadata.clone(),
    );

    let notification_id = notification.notification_id.clone();

    // Save to database
    state.db.insert(&notification).await?;

    // Send SMS
    let sms_message = SmsMessage {
        to: request.to,
        body: request.body,
    };

    match state.sms_provider.send(&sms_message).await {
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
                "SMS sent successfully"
            );
        }
        Err(ProviderError::NotEnabled(msg)) => {
            // Provider not enabled - mark as sent (mock behavior)
            tracing::warn!(
                notification_id = %notification_id,
                "SMS provider not enabled: {}. Marking as sent.",
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
                "Failed to send SMS"
            );

            return Err(AppError::InternalError(anyhow::anyhow!(error_msg)));
        }
    }

    Ok((
        StatusCode::ACCEPTED,
        Json(SendSmsResponse {
            notification_id,
            status: notification.status.to_string(),
            channel: "sms".to_string(),
        }),
    ))
}
