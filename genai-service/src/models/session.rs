//! Session model for conversation context persistence.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A conversation session that maintains context across multiple requests.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// Unique session identifier.
    pub session_id: String,

    /// Optional human-readable title.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    /// System prompt for this session.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,

    /// Tenant ID for multi-tenancy.
    pub tenant_id: String,

    /// User ID who owns this session.
    pub user_id: String,

    /// Documents attached to this session.
    pub documents: Vec<SessionDocument>,

    /// Messages in this session.
    pub messages: Vec<SessionMessage>,

    /// Total number of messages.
    pub message_count: i32,

    /// Total input tokens consumed.
    pub total_input_tokens: i32,

    /// Total output tokens generated.
    pub total_output_tokens: i32,

    /// When the session was created.
    #[serde(with = "chrono::serde::ts_milliseconds")]
    pub created_at: DateTime<Utc>,

    /// When the session was last updated.
    #[serde(with = "chrono::serde::ts_milliseconds")]
    pub updated_at: DateTime<Utc>,
}

/// A document attached to a session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionDocument {
    /// Document ID from document-service.
    pub document_id: String,

    /// Signed URL for access.
    pub signed_url: String,

    /// MIME type.
    pub mime_type: String,

    /// Pre-extracted text content (if available).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_content: Option<String>,
}

/// A message in a session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMessage {
    /// Role: "user" or "assistant".
    pub role: String,

    /// Message content.
    pub content: String,

    /// Output format used (for assistant messages).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_format: Option<String>,

    /// When the message was created.
    #[serde(with = "chrono::serde::ts_milliseconds")]
    pub timestamp: DateTime<Utc>,
}

impl Session {
    /// Create a new session.
    pub fn new(
        tenant_id: String,
        user_id: String,
        title: Option<String>,
        system_prompt: Option<String>,
        documents: Vec<SessionDocument>,
    ) -> Self {
        let now = Utc::now();
        Self {
            session_id: uuid::Uuid::new_v4().to_string(),
            title,
            system_prompt,
            tenant_id,
            user_id,
            documents,
            messages: Vec::new(),
            message_count: 0,
            total_input_tokens: 0,
            total_output_tokens: 0,
            created_at: now,
            updated_at: now,
        }
    }

    /// Add a message to the session.
    pub fn add_message(&mut self, role: String, content: String, output_format: Option<String>) {
        self.messages.push(SessionMessage {
            role,
            content,
            output_format,
            timestamp: Utc::now(),
        });
        self.message_count = self.messages.len() as i32;
        self.updated_at = Utc::now();
    }

    /// Update token usage.
    pub fn add_usage(&mut self, input_tokens: i32, output_tokens: i32) {
        self.total_input_tokens += input_tokens;
        self.total_output_tokens += output_tokens;
        self.updated_at = Utc::now();
    }
}

impl SessionDocument {
    /// Create a new session document.
    pub fn new(
        document_id: String,
        signed_url: String,
        mime_type: String,
        text_content: Option<String>,
    ) -> Self {
        Self {
            document_id,
            signed_url,
            mime_type,
            text_content,
        }
    }
}
