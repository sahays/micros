//! Usage tracking model for token consumption.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A record of token usage for a single request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageRecord {
    /// Request ID for correlation.
    pub request_id: String,

    /// Session ID (if request was part of a session).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,

    /// Tenant ID for multi-tenancy.
    pub tenant_id: String,

    /// User ID who made the request.
    pub user_id: String,

    /// Model that was used.
    pub model: String,

    /// Input tokens consumed.
    pub input_tokens: i32,

    /// Output tokens generated.
    pub output_tokens: i32,

    /// Total tokens (input + output).
    pub total_tokens: i32,

    /// Output format used.
    pub output_format: String,

    /// When the request was processed.
    #[serde(with = "chrono::serde::ts_milliseconds")]
    pub timestamp: DateTime<Utc>,

    /// Additional tags for categorization.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub tags: HashMap<String, String>,
}

impl UsageRecord {
    /// Create a new usage record.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        request_id: String,
        session_id: Option<String>,
        tenant_id: String,
        user_id: String,
        model: String,
        input_tokens: i32,
        output_tokens: i32,
        output_format: String,
        tags: HashMap<String, String>,
    ) -> Self {
        Self {
            request_id,
            session_id,
            tenant_id,
            user_id,
            model,
            input_tokens,
            output_tokens,
            total_tokens: input_tokens + output_tokens,
            output_format,
            timestamp: Utc::now(),
            tags,
        }
    }
}

/// Aggregated usage statistics.
#[derive(Debug, Clone, Default)]
pub struct UsageStats {
    /// Total input tokens.
    pub total_input_tokens: i64,

    /// Total output tokens.
    pub total_output_tokens: i64,

    /// Total tokens.
    pub total_tokens: i64,

    /// Total requests.
    pub total_requests: i32,

    /// Usage by model.
    pub by_model: HashMap<String, ModelUsage>,
}

/// Usage statistics for a single model.
#[derive(Debug, Clone, Default)]
pub struct ModelUsage {
    /// Model ID.
    pub model: String,

    /// Total tokens used.
    pub tokens: i64,

    /// Number of requests.
    pub requests: i32,
}

impl UsageStats {
    /// Aggregate usage records into statistics.
    pub fn from_records(records: &[UsageRecord]) -> Self {
        let mut stats = UsageStats::default();

        for record in records {
            stats.total_input_tokens += record.input_tokens as i64;
            stats.total_output_tokens += record.output_tokens as i64;
            stats.total_tokens += record.total_tokens as i64;
            stats.total_requests += 1;

            let model_usage = stats
                .by_model
                .entry(record.model.clone())
                .or_insert_with(|| ModelUsage {
                    model: record.model.clone(),
                    tokens: 0,
                    requests: 0,
                });

            model_usage.tokens += record.total_tokens as i64;
            model_usage.requests += 1;
        }

        stats
    }
}
