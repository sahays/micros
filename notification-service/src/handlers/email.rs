use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use validator::Validate;

use crate::models::{Notification, NotificationStatus};
use crate::services::{EmailMessage, ProviderError};
use crate::startup::AppState;
use service_core::error::AppError;

#[derive(Debug, Deserialize, Validate)]
pub struct SendEmailRequest {
    #[validate(email(message = "Invalid email address"))]
    pub to: String,
    #[validate(length(min = 1, message = "Subject cannot be empty"))]
    pub subject: String,
    pub body_html: Option<String>,
    pub body_text: Option<String>,
    pub from_name: Option<String>,
    #[validate(email(message = "Invalid reply-to email address"))]
    pub reply_to: Option<String>,
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Serialize)]
pub struct SendEmailResponse {
    pub notification_id: String,
    pub status: String,
    pub channel: String,
}

#[tracing::instrument(skip(state, request))]
pub async fn send_email(
    State(state): State<AppState>,
    Json(request): Json<SendEmailRequest>,
) -> Result<(StatusCode, Json<SendEmailResponse>), AppError> {
    request.validate()?;

    // Ensure at least one body is provided
    if request.body_html.is_none() && request.body_text.is_none() {
        return Err(AppError::BadRequest(anyhow::anyhow!(
            "At least one of body_html or body_text must be provided"
        )));
    }

    // Create notification record
    let mut notification = Notification::new_email(
        request.to.clone(),
        request.subject.clone(),
        request.body_text.clone(),
        request.body_html.clone(),
        request.from_name.clone(),
        request.reply_to.clone(),
        request.metadata.clone(),
    );

    let notification_id = notification.notification_id.clone();

    // Save to database
    state.db.insert(&notification).await?;

    // Send email
    let email_message = EmailMessage {
        to: request.to,
        subject: request.subject,
        body_text: request.body_text,
        body_html: request.body_html,
        from_name: request.from_name,
        reply_to: request.reply_to,
    };

    match state.email_provider.send(&email_message).await {
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
                "Email sent successfully"
            );
        }
        Err(ProviderError::NotEnabled(msg)) => {
            // Provider not enabled - mark as sent (mock behavior)
            tracing::warn!(
                notification_id = %notification_id,
                "Email provider not enabled: {}. Marking as sent.",
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
                "Failed to send email"
            );

            return Err(AppError::InternalError(anyhow::anyhow!(error_msg)));
        }
    }

    Ok((
        StatusCode::ACCEPTED,
        Json(SendEmailResponse {
            notification_id,
            status: notification.status.to_string(),
            channel: "email".to_string(),
        }),
    ))
}
