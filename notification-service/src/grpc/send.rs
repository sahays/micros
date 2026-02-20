use crate::grpc::capability_check::capabilities;
use crate::grpc::notification_service::proto_to_push_platform;
use crate::grpc::proto::{
    SendEmailRequest, SendEmailResponse, SendPushRequest, SendPushResponse, SendSmsRequest,
    SendSmsResponse,
};
use crate::models::Notification;
use crate::models::NotificationStatus;
use crate::services::{EmailMessage, ProviderError, PushMessage, SmsMessage};
use crate::startup::AppState;
use tonic::{Request, Response, Status};

#[tracing::instrument(skip(state, request))]
pub async fn send_email(
    state: &AppState,
    request: Request<SendEmailRequest>,
) -> Result<Response<SendEmailResponse>, Status> {
    // Capability check - derive tenant_id from auth context
    let auth = state
        .capability_checker
        .require_capability(&request, capabilities::NOTIFICATION_EMAIL_SEND)
        .await?;
    let tenant_id = auth.tenant_id.clone();

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
        tenant_id.clone(),
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
    state
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

    // Send with retry (up to 2 retries for transient failures)
    let max_attempts = 3;
    let mut last_error = None;
    for attempt in 1..=max_attempts {
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

                tracing::info!(notification_id = %notification_id, "Email sent successfully");
                last_error = None;
                break;
            }
            Err(ProviderError::NotEnabled(msg)) => {
                let error_msg = format!("Email provider not enabled: {}", msg);
                notification.mark_failed(error_msg.clone());
                let _ = state
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
                    "Email provider not enabled: {}",
                    msg
                );

                return Err(Status::failed_precondition(error_msg));
            }
            Err(ref e)
                if matches!(e, ProviderError::SendFailed(_)) && attempt < max_attempts =>
            {
                tracing::warn!(
                    notification_id = %notification_id,
                    attempt = attempt,
                    error = %e,
                    "Email send failed, retrying"
                );
                last_error = Some(e.to_string());
                tokio::time::sleep(std::time::Duration::from_millis(500 * attempt as u64)).await;
            }
            Err(e) => {
                last_error = Some(e.to_string());
                break;
            }
        }
    }

    if let Some(error_msg) = last_error {
        notification.mark_failed(error_msg.clone());
        let _ = state
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
            "Failed to send email after {} attempts",
            max_attempts
        );

        return Err(Status::internal(format!(
            "Failed to send email: {}",
            error_msg
        )));
    }

    Ok(Response::new(SendEmailResponse {
        notification_id,
        status: notification.status.to_string(),
        channel: "email".to_string(),
    }))
}

#[tracing::instrument(skip(state, request))]
pub async fn send_sms(
    state: &AppState,
    request: Request<SendSmsRequest>,
) -> Result<Response<SendSmsResponse>, Status> {
    // Capability check - derive tenant_id from auth context
    let auth = state
        .capability_checker
        .require_capability(&request, capabilities::NOTIFICATION_SMS_SEND)
        .await?;
    let tenant_id = auth.tenant_id.clone();

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
    let mut notification = Notification::new_sms(
        tenant_id.clone(),
        req.to.clone(),
        req.body.clone(),
        req.metadata.clone(),
    );

    let notification_id = notification.notification_id.clone();

    // Save to database
    state
        .db
        .insert(&notification)
        .await
        .map_err(|e| Status::internal(format!("Database error: {}", e)))?;

    // Send SMS
    let sms_message = SmsMessage {
        to: req.to,
        body: req.body,
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

            tracing::info!(notification_id = %notification_id, "SMS sent successfully");
        }
        Err(ProviderError::NotEnabled(msg)) => {
            let error_msg = format!("SMS provider not enabled: {}", msg);
            notification.mark_failed(error_msg.clone());
            let _ = state
                .db
                .update_status(
                    &notification_id,
                    NotificationStatus::Failed,
                    None,
                    Some(&error_msg),
                )
                .await;

            tracing::error!(notification_id = %notification_id, "{}", error_msg);

            return Err(Status::failed_precondition(error_msg));
        }
        Err(e) => {
            let error_msg = e.to_string();
            notification.mark_failed(error_msg.clone());
            let _ = state
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

#[tracing::instrument(skip(state, request))]
pub async fn send_push(
    state: &AppState,
    request: Request<SendPushRequest>,
) -> Result<Response<SendPushResponse>, Status> {
    // Capability check - derive tenant_id from auth context
    let auth = state
        .capability_checker
        .require_capability(&request, capabilities::NOTIFICATION_PUSH_SEND)
        .await?;
    let tenant_id = auth.tenant_id.clone();

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
        tenant_id.clone(),
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
    state
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

            tracing::info!(notification_id = %notification_id, "Push notification sent successfully");
        }
        Err(ProviderError::NotEnabled(msg)) => {
            let error_msg = format!("Push provider not enabled: {}", msg);
            notification.mark_failed(error_msg.clone());
            let _ = state
                .db
                .update_status(
                    &notification_id,
                    NotificationStatus::Failed,
                    None,
                    Some(&error_msg),
                )
                .await;

            tracing::error!(notification_id = %notification_id, "{}", error_msg);

            return Err(Status::failed_precondition(error_msg));
        }
        Err(e) => {
            let error_msg = e.to_string();
            notification.mark_failed(error_msg.clone());
            let _ = state
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
