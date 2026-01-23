//! Billing cycle model.

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Billing cycle status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BillingCycleStatus {
    Pending,
    Invoiced,
    Paid,
    Failed,
}

impl BillingCycleStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            BillingCycleStatus::Pending => "pending",
            BillingCycleStatus::Invoiced => "invoiced",
            BillingCycleStatus::Paid => "paid",
            BillingCycleStatus::Failed => "failed",
        }
    }

    pub fn from_string(s: &str) -> Self {
        match s {
            "invoiced" => BillingCycleStatus::Invoiced,
            "paid" => BillingCycleStatus::Paid,
            "failed" => BillingCycleStatus::Failed,
            _ => BillingCycleStatus::Pending,
        }
    }

    pub fn to_proto(&self) -> i32 {
        match self {
            BillingCycleStatus::Pending => 1,
            BillingCycleStatus::Invoiced => 2,
            BillingCycleStatus::Paid => 3,
            BillingCycleStatus::Failed => 4,
        }
    }

    pub fn from_proto(value: i32) -> Self {
        match value {
            1 => BillingCycleStatus::Pending,
            2 => BillingCycleStatus::Invoiced,
            3 => BillingCycleStatus::Paid,
            4 => BillingCycleStatus::Failed,
            _ => BillingCycleStatus::Pending,
        }
    }
}

/// Billing cycle.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BillingCycle {
    pub cycle_id: Uuid,
    pub subscription_id: Uuid,
    pub period_start: NaiveDate,
    pub period_end: NaiveDate,
    pub status: String,
    pub invoice_id: Option<Uuid>,
    pub created_utc: DateTime<Utc>,
    pub updated_utc: DateTime<Utc>,
}

/// Charge type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChargeType {
    Recurring,
    Usage,
    OneTime,
    Proration,
}

impl ChargeType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ChargeType::Recurring => "recurring",
            ChargeType::Usage => "usage",
            ChargeType::OneTime => "one_time",
            ChargeType::Proration => "proration",
        }
    }

    pub fn from_string(s: &str) -> Self {
        match s {
            "usage" => ChargeType::Usage,
            "one_time" => ChargeType::OneTime,
            "proration" => ChargeType::Proration,
            _ => ChargeType::Recurring,
        }
    }

    pub fn to_proto(&self) -> i32 {
        match self {
            ChargeType::Recurring => 1,
            ChargeType::Usage => 2,
            ChargeType::OneTime => 3,
            ChargeType::Proration => 4,
        }
    }

    pub fn from_proto(value: i32) -> Self {
        match value {
            1 => ChargeType::Recurring,
            2 => ChargeType::Usage,
            3 => ChargeType::OneTime,
            4 => ChargeType::Proration,
            _ => ChargeType::Recurring,
        }
    }
}

/// Charge within a billing cycle.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Charge {
    pub charge_id: Uuid,
    pub cycle_id: Uuid,
    pub charge_type: String,
    pub description: String,
    pub quantity: Decimal,
    pub unit_price: Decimal,
    pub amount: Decimal,
    pub is_prorated: bool,
    pub proration_factor: Option<Decimal>,
    pub component_id: Option<Uuid>,
    pub metadata: Option<serde_json::Value>,
    pub created_utc: DateTime<Utc>,
}

/// Input for creating a charge.
#[derive(Debug, Clone)]
pub struct CreateCharge {
    pub cycle_id: Uuid,
    pub charge_type: ChargeType,
    pub description: String,
    pub quantity: Decimal,
    pub unit_price: Decimal,
    pub amount: Decimal,
    pub is_prorated: bool,
    pub proration_factor: Option<Decimal>,
    pub component_id: Option<Uuid>,
    pub metadata: Option<serde_json::Value>,
}

/// Filter parameters for listing billing cycles.
#[derive(Debug, Clone, Default)]
pub struct ListBillingCyclesFilter {
    pub status: Option<BillingCycleStatus>,
    pub page_size: i32,
    pub page_token: Option<Uuid>,
}

/// Filter parameters for listing charges.
#[derive(Debug, Clone, Default)]
pub struct ListChargesFilter {
    pub charge_type: Option<ChargeType>,
    pub page_size: i32,
    pub page_token: Option<Uuid>,
}
