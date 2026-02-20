use crate::grpc::capability_check::capabilities;
use crate::grpc::notification_service::proto_to_push_platform;
use crate::grpc::proto::{
    BatchNotification, BatchNotificationResult, NotificationChannel,
    NotificationStatus as ProtoNotificationStatus, SendBatchRequest, SendBatchResponse,
};
use crate::models::{Notification, NotificationStatus};
use crate::services::{EmailMessage, ProviderError, PushMessage, SmsMessage};
use crate::startup::AppState;
use tonic::{Request, Response, Status};

#[tracing::instrument(skip(state, request))]
pub async fn send_batch(
    state: &AppState,
    request: Request<SendBatchRequest>,
) -> Result<Response<SendBatchResponse>, Status> {
    // Capability check - derive tenant_id from auth context
    let auth = state
        .capability_checker
        .require_capability(&request, capabilities::NOTIFICATION_BATCH_SEND)
        .await?;
    let tenant_id = auth.tenant_id.clone();

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
        let result = process_batch_notification(state, &tenant_id, &notification_request).await;
        results.push(result);
    }

    Ok(Response::new(SendBatchResponse { batch_id, results }))
}

pub async fn process_batch_notification(
    state: &AppState,
    tenant_id: &str,
    notification: &BatchNotification,
) -> BatchNotificationResult {
    let channel = NotificationChannel::try_from(notification.channel);

    match channel {
        Ok(NotificationChannel::Email) => process_batch_email(state, tenant_id, notification).await,
        Ok(NotificationChannel::Sms) => process_batch_sms(state, tenant_id, notification).await,
        Ok(NotificationChannel::Push) => process_batch_push(state, tenant_id, notification).await,
        _ => BatchNotificationResult {
            notification_id: String::new(),
            status: ProtoNotificationStatus::Failed as i32,
            error: Some("Invalid or unspecified channel".to_string()),
        },
    }
}

async fn process_batch_email(
    state: &AppState,
    tenant_id: &str,
    batch: &BatchNotification,
) -> BatchNotificationResult {
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
        tenant_id.to_string(),
        email.to.clone(),
        email.subject.clone(),
        email.body_text.clone(),
        email.body_html.clone(),
        email.from_name.clone(),
        email.reply_to.clone(),
        email.metadata.clone(),
    );

    let notification_id = notification.notification_id.clone();

    if let Err(e) = state.db.insert(&notification).await {
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

                return BatchNotificationResult {
                    notification_id,
                    status: ProtoNotificationStatus::Sent as i32,
                    error: None,
                };
            }
            Err(ProviderError::NotEnabled(msg)) => {
                let error_msg = format!("Email provider not enabled: {}", msg);
                let _ = state
                    .db
                    .update_status(
                        &notification_id,
                        NotificationStatus::Failed,
                        None,
                        Some(&error_msg),
                    )
                    .await;

                return BatchNotificationResult {
                    notification_id,
                    status: ProtoNotificationStatus::Failed as i32,
                    error: Some(error_msg),
                };
            }
            Err(ref e)
                if matches!(e, ProviderError::SendFailed(_)) && attempt < max_attempts =>
            {
                tracing::warn!(
                    notification_id = %notification_id,
                    attempt = attempt,
                    error = %e,
                    "Batch email send failed, retrying"
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

    let error_msg = last_error.unwrap_or_else(|| "Unknown error".to_string());
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
        status: ProtoNotificationStatus::Failed as i32,
        error: Some(error_msg),
    }
}

async fn process_batch_sms(
    state: &AppState,
    tenant_id: &str,
    batch: &BatchNotification,
) -> BatchNotificationResult {
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

    let mut notification = Notification::new_sms(
        tenant_id.to_string(),
        sms.to.clone(),
        sms.body.clone(),
        sms.metadata.clone(),
    );

    let notification_id = notification.notification_id.clone();

    if let Err(e) = state.db.insert(&notification).await {
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
                status: ProtoNotificationStatus::Sent as i32,
                error: None,
            }
        }
        Err(ProviderError::NotEnabled(msg)) => {
            let error_msg = format!("SMS provider not enabled: {}", msg);
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
                status: ProtoNotificationStatus::Failed as i32,
                error: Some(error_msg),
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
                status: ProtoNotificationStatus::Failed as i32,
                error: Some(error_msg),
            }
        }
    }
}

async fn process_batch_push(
    state: &AppState,
    tenant_id: &str,
    batch: &BatchNotification,
) -> BatchNotificationResult {
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
        tenant_id.to_string(),
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

    if let Err(e) = state.db.insert(&notification).await {
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
                status: ProtoNotificationStatus::Sent as i32,
                error: None,
            }
        }
        Err(ProviderError::NotEnabled(msg)) => {
            let error_msg = format!("Push provider not enabled: {}", msg);
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
                status: ProtoNotificationStatus::Failed as i32,
                error: Some(error_msg),
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
                status: ProtoNotificationStatus::Failed as i32,
                error: Some(error_msg),
            }
        }
    }
}
