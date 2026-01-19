use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::models::{Channel, NotificationStatus};
use crate::startup::AppState;
use service_core::error::AppError;

#[derive(Debug, Serialize)]
pub struct NotificationResponse {
    pub notification_id: String,
    pub channel: String,
    pub status: String,
    pub recipient: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub created_utc: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sent_utc: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delivered_utc: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failed_utc: Option<DateTime<Utc>>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
pub struct ListNotificationsQuery {
    pub channel: Option<String>,
    pub status: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: u64,
}

fn default_limit() -> i64 {
    50
}

#[derive(Debug, Serialize)]
pub struct ListNotificationsResponse {
    pub notifications: Vec<NotificationResponse>,
    pub limit: i64,
    pub offset: u64,
    pub count: usize,
}

#[tracing::instrument(skip(state))]
pub async fn get_notification(
    State(state): State<AppState>,
    Path(notification_id): Path<String>,
) -> Result<Json<NotificationResponse>, AppError> {
    let notification = state.db.find_by_id(&notification_id).await?;

    match notification {
        Some(n) => Ok(Json(NotificationResponse {
            notification_id: n.notification_id,
            channel: n.channel.to_string(),
            status: n.status.to_string(),
            recipient: n.recipient,
            subject: n.subject,
            provider_id: n.provider_id,
            error: n.error_message,
            created_utc: n.created_utc,
            sent_utc: n.sent_utc,
            delivered_utc: n.delivered_utc,
            failed_utc: n.failed_utc,
            metadata: n.metadata,
        })),
        None => Err(AppError::NotFound(anyhow::anyhow!(
            "Notification not found: {}",
            notification_id
        ))),
    }
}

#[tracing::instrument(skip(state))]
pub async fn list_notifications(
    State(state): State<AppState>,
    Query(query): Query<ListNotificationsQuery>,
) -> Result<Json<ListNotificationsResponse>, AppError> {
    // Parse channel filter
    let channel = match &query.channel {
        Some(c) => match c.to_lowercase().as_str() {
            "email" => Some(Channel::Email),
            "sms" => Some(Channel::Sms),
            "push" => Some(Channel::Push),
            _ => {
                return Err(AppError::BadRequest(anyhow::anyhow!(
                    "Invalid channel: {}. Must be one of: email, sms, push",
                    c
                )))
            }
        },
        None => None,
    };

    // Parse status filter
    let status = match &query.status {
        Some(s) => match s.to_lowercase().as_str() {
            "queued" => Some(NotificationStatus::Queued),
            "sent" => Some(NotificationStatus::Sent),
            "delivered" => Some(NotificationStatus::Delivered),
            "failed" => Some(NotificationStatus::Failed),
            _ => {
                return Err(AppError::BadRequest(anyhow::anyhow!(
                    "Invalid status: {}. Must be one of: queued, sent, delivered, failed",
                    s
                )))
            }
        },
        None => None,
    };

    // Clamp limit to reasonable range
    let limit = query.limit.clamp(1, 100);

    let notifications = state.db.list(channel, status, limit, query.offset).await?;

    let responses: Vec<NotificationResponse> = notifications
        .into_iter()
        .map(|n| NotificationResponse {
            notification_id: n.notification_id,
            channel: n.channel.to_string(),
            status: n.status.to_string(),
            recipient: n.recipient,
            subject: n.subject,
            provider_id: n.provider_id,
            error: n.error_message,
            created_utc: n.created_utc,
            sent_utc: n.sent_utc,
            delivered_utc: n.delivered_utc,
            failed_utc: n.failed_utc,
            metadata: n.metadata,
        })
        .collect();

    let count = responses.len();

    Ok(Json(ListNotificationsResponse {
        notifications: responses,
        limit,
        offset: query.offset,
        count,
    }))
}
