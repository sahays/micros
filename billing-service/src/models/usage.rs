//! Usage record model.

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Usage record.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct UsageRecord {
    pub record_id: Uuid,
    pub subscription_id: Uuid,
    pub component_id: Uuid,
    pub idempotency_key: String,
    pub quantity: Decimal,
    pub timestamp: DateTime<Utc>,
    pub cycle_id: Option<Uuid>,
    pub is_invoiced: bool,
    pub metadata: Option<serde_json::Value>,
    pub created_utc: DateTime<Utc>,
}

/// Input for recording usage.
#[derive(Debug, Clone)]
pub struct RecordUsage {
    pub subscription_id: Uuid,
    pub component_id: Uuid,
    pub quantity: Decimal,
    pub timestamp: DateTime<Utc>,
    pub idempotency_key: String,
    pub metadata: Option<serde_json::Value>,
}

/// Filter parameters for listing usage records.
#[derive(Debug, Clone, Default)]
pub struct ListUsageFilter {
    pub component_id: Option<Uuid>,
    pub cycle_id: Option<Uuid>,
    pub is_invoiced: Option<bool>,
    pub page_size: i32,
    pub page_token: Option<Uuid>,
}

/// Usage summary for a component.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageComponentSummary {
    pub component_id: Uuid,
    pub name: String,
    pub total_quantity: Decimal,
    pub included_units: i32,
    pub billable_units: Decimal,
    pub amount: Decimal,
}
