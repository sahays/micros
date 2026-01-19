use chrono::{DateTime, Utc};
use mongodb::bson::oid::ObjectId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Channel {
    Email,
    Sms,
    Push,
}

impl std::fmt::Display for Channel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Channel::Email => write!(f, "email"),
            Channel::Sms => write!(f, "sms"),
            Channel::Push => write!(f, "push"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum NotificationStatus {
    Queued,
    Sent,
    Delivered,
    Failed,
}

impl std::fmt::Display for NotificationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NotificationStatus::Queued => write!(f, "queued"),
            NotificationStatus::Sent => write!(f, "sent"),
            NotificationStatus::Delivered => write!(f, "delivered"),
            NotificationStatus::Failed => write!(f, "failed"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum PushPlatform {
    Fcm,
    Apns,
}

impl std::fmt::Display for PushPlatform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PushPlatform::Fcm => write!(f, "fcm"),
            PushPlatform::Apns => write!(f, "apns"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub notification_id: String,
    pub channel: Channel,
    pub status: NotificationStatus,
    pub recipient: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body_html: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reply_to: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub platform: Option<PushPlatform>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub push_title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub push_data: Option<HashMap<String, String>>,
    #[serde(default)]
    pub metadata: HashMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    #[serde(with = "mongodb::bson::serde_helpers::chrono_datetime_as_bson_datetime")]
    pub created_utc: DateTime<Utc>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "opt_chrono_datetime_as_bson_datetime"
    )]
    pub sent_utc: Option<DateTime<Utc>>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "opt_chrono_datetime_as_bson_datetime"
    )]
    pub delivered_utc: Option<DateTime<Utc>>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "opt_chrono_datetime_as_bson_datetime"
    )]
    pub failed_utc: Option<DateTime<Utc>>,
}

// Helper module for optional DateTime<Utc> as BSON DateTime
mod opt_chrono_datetime_as_bson_datetime {
    use chrono::{DateTime, Utc};
    use mongodb::bson;
    use serde::{self, Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S>(date: &Option<DateTime<Utc>>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match date {
            Some(dt) => {
                let bson_dt = bson::DateTime::from_chrono(*dt);
                bson_dt.serialize(serializer)
            }
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<DateTime<Utc>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let opt: Option<bson::DateTime> = Option::deserialize(deserializer)?;
        Ok(opt.map(|dt| dt.to_chrono()))
    }
}

impl Notification {
    pub fn new_email(
        recipient: String,
        subject: String,
        body_text: Option<String>,
        body_html: Option<String>,
        from_name: Option<String>,
        reply_to: Option<String>,
        metadata: HashMap<String, String>,
    ) -> Self {
        Self {
            id: None,
            notification_id: uuid::Uuid::new_v4().to_string(),
            channel: Channel::Email,
            status: NotificationStatus::Queued,
            recipient,
            subject: Some(subject),
            body: body_text,
            body_html,
            from_name,
            reply_to,
            platform: None,
            push_title: None,
            push_data: None,
            metadata,
            provider_id: None,
            error_message: None,
            created_utc: Utc::now(),
            sent_utc: None,
            delivered_utc: None,
            failed_utc: None,
        }
    }

    pub fn new_sms(recipient: String, body: String, metadata: HashMap<String, String>) -> Self {
        Self {
            id: None,
            notification_id: uuid::Uuid::new_v4().to_string(),
            channel: Channel::Sms,
            status: NotificationStatus::Queued,
            recipient,
            subject: None,
            body: Some(body),
            body_html: None,
            from_name: None,
            reply_to: None,
            platform: None,
            push_title: None,
            push_data: None,
            metadata,
            provider_id: None,
            error_message: None,
            created_utc: Utc::now(),
            sent_utc: None,
            delivered_utc: None,
            failed_utc: None,
        }
    }

    pub fn new_push(
        device_token: String,
        platform: PushPlatform,
        title: String,
        body: String,
        data: Option<HashMap<String, String>>,
        metadata: HashMap<String, String>,
    ) -> Self {
        Self {
            id: None,
            notification_id: uuid::Uuid::new_v4().to_string(),
            channel: Channel::Push,
            status: NotificationStatus::Queued,
            recipient: device_token,
            subject: None,
            body: Some(body),
            body_html: None,
            from_name: None,
            reply_to: None,
            platform: Some(platform),
            push_title: Some(title),
            push_data: data,
            metadata,
            provider_id: None,
            error_message: None,
            created_utc: Utc::now(),
            sent_utc: None,
            delivered_utc: None,
            failed_utc: None,
        }
    }

    pub fn mark_sent(&mut self, provider_id: Option<String>) {
        self.status = NotificationStatus::Sent;
        self.sent_utc = Some(Utc::now());
        self.provider_id = provider_id;
    }

    pub fn mark_delivered(&mut self) {
        self.status = NotificationStatus::Delivered;
        self.delivered_utc = Some(Utc::now());
    }

    pub fn mark_failed(&mut self, error: String) {
        self.status = NotificationStatus::Failed;
        self.failed_utc = Some(Utc::now());
        self.error_message = Some(error);
    }
}
