//! Billing run model.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Billing run type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BillingRunType {
    Scheduled,
    Manual,
    Single,
}

impl BillingRunType {
    pub fn as_str(&self) -> &'static str {
        match self {
            BillingRunType::Scheduled => "scheduled",
            BillingRunType::Manual => "manual",
            BillingRunType::Single => "single",
        }
    }

    pub fn from_string(s: &str) -> Self {
        match s {
            "manual" => BillingRunType::Manual,
            "single" => BillingRunType::Single,
            _ => BillingRunType::Scheduled,
        }
    }

    pub fn to_proto(&self) -> i32 {
        match self {
            BillingRunType::Scheduled => 1,
            BillingRunType::Manual => 2,
            BillingRunType::Single => 3,
        }
    }

    pub fn from_proto(value: i32) -> Self {
        match value {
            1 => BillingRunType::Scheduled,
            2 => BillingRunType::Manual,
            3 => BillingRunType::Single,
            _ => BillingRunType::Scheduled,
        }
    }
}

/// Billing run status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BillingRunStatus {
    Running,
    Completed,
    Failed,
}

impl BillingRunStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            BillingRunStatus::Running => "running",
            BillingRunStatus::Completed => "completed",
            BillingRunStatus::Failed => "failed",
        }
    }

    pub fn from_string(s: &str) -> Self {
        match s {
            "completed" => BillingRunStatus::Completed,
            "failed" => BillingRunStatus::Failed,
            _ => BillingRunStatus::Running,
        }
    }

    pub fn to_proto(&self) -> i32 {
        match self {
            BillingRunStatus::Running => 1,
            BillingRunStatus::Completed => 2,
            BillingRunStatus::Failed => 3,
        }
    }

    pub fn from_proto(value: i32) -> Self {
        match value {
            1 => BillingRunStatus::Running,
            2 => BillingRunStatus::Completed,
            3 => BillingRunStatus::Failed,
            _ => BillingRunStatus::Running,
        }
    }
}

/// Billing run.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BillingRun {
    pub run_id: Uuid,
    pub tenant_id: Uuid,
    pub run_type: String,
    pub status: String,
    pub started_utc: DateTime<Utc>,
    pub completed_utc: Option<DateTime<Utc>>,
    pub subscriptions_processed: i32,
    pub subscriptions_succeeded: i32,
    pub subscriptions_failed: i32,
    pub error_message: Option<String>,
}

/// Billing run result per subscription.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BillingRunResult {
    pub result_id: Uuid,
    pub run_id: Uuid,
    pub subscription_id: Uuid,
    pub status: String,
    pub invoice_id: Option<Uuid>,
    pub error_message: Option<String>,
    pub created_utc: DateTime<Utc>,
}

/// Filter parameters for listing billing runs.
#[derive(Debug, Clone, Default)]
pub struct ListBillingRunsFilter {
    pub status: Option<BillingRunStatus>,
    pub run_type: Option<BillingRunType>,
    pub page_size: i32,
    pub page_token: Option<Uuid>,
}
