//! Billing plan model.

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Billing interval for plans.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BillingInterval {
    Daily,
    Weekly,
    Monthly,
    Quarterly,
    Annually,
}

impl BillingInterval {
    pub fn as_str(&self) -> &'static str {
        match self {
            BillingInterval::Daily => "daily",
            BillingInterval::Weekly => "weekly",
            BillingInterval::Monthly => "monthly",
            BillingInterval::Quarterly => "quarterly",
            BillingInterval::Annually => "annually",
        }
    }

    pub fn from_string(s: &str) -> Self {
        match s {
            "daily" => BillingInterval::Daily,
            "weekly" => BillingInterval::Weekly,
            "quarterly" => BillingInterval::Quarterly,
            "annually" => BillingInterval::Annually,
            _ => BillingInterval::Monthly,
        }
    }

    pub fn from_proto(value: i32) -> Self {
        match value {
            1 => BillingInterval::Daily,
            2 => BillingInterval::Weekly,
            3 => BillingInterval::Monthly,
            4 => BillingInterval::Quarterly,
            5 => BillingInterval::Annually,
            _ => BillingInterval::Monthly,
        }
    }

    pub fn to_proto(&self) -> i32 {
        match self {
            BillingInterval::Daily => 1,
            BillingInterval::Weekly => 2,
            BillingInterval::Monthly => 3,
            BillingInterval::Quarterly => 4,
            BillingInterval::Annually => 5,
        }
    }
}

/// Billing plan.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BillingPlan {
    pub plan_id: Uuid,
    pub tenant_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub billing_interval: String,
    pub interval_count: i32,
    pub base_price: Decimal,
    pub currency: String,
    pub tax_rate_id: Option<Uuid>,
    pub is_active: bool,
    pub is_archived: bool,
    pub metadata: Option<serde_json::Value>,
    pub created_utc: DateTime<Utc>,
    pub updated_utc: DateTime<Utc>,
}

/// Usage component within a plan.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct UsageComponent {
    pub component_id: Uuid,
    pub plan_id: Uuid,
    pub name: String,
    pub unit_name: String,
    pub unit_price: Decimal,
    pub included_units: i32,
    pub is_active: bool,
    pub created_utc: DateTime<Utc>,
}

/// Input for creating a plan.
#[derive(Debug, Clone)]
pub struct CreatePlan {
    pub tenant_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub billing_interval: BillingInterval,
    pub interval_count: i32,
    pub base_price: Decimal,
    pub currency: String,
    pub tax_rate_id: Option<Uuid>,
    pub metadata: Option<serde_json::Value>,
}

/// Input for creating a usage component.
#[derive(Debug, Clone)]
pub struct CreateUsageComponent {
    pub plan_id: Uuid,
    pub name: String,
    pub unit_name: String,
    pub unit_price: Decimal,
    pub included_units: i32,
}

/// Input for updating a plan.
#[derive(Debug, Clone, Default)]
pub struct UpdatePlan {
    pub name: Option<String>,
    pub description: Option<String>,
    pub base_price: Option<Decimal>,
    pub tax_rate_id: Option<Uuid>,
    pub metadata: Option<serde_json::Value>,
}

/// Filter parameters for listing plans.
#[derive(Debug, Clone, Default)]
pub struct ListPlansFilter {
    pub include_archived: bool,
    pub page_size: i32,
    pub page_token: Option<Uuid>,
}
