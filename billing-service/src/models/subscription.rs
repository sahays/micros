//! Subscription model.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Subscription status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubscriptionStatus {
    Trial,
    Active,
    Paused,
    Cancelled,
    Expired,
}

impl SubscriptionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            SubscriptionStatus::Trial => "trial",
            SubscriptionStatus::Active => "active",
            SubscriptionStatus::Paused => "paused",
            SubscriptionStatus::Cancelled => "cancelled",
            SubscriptionStatus::Expired => "expired",
        }
    }

    pub fn from_string(s: &str) -> Self {
        match s {
            "trial" => SubscriptionStatus::Trial,
            "paused" => SubscriptionStatus::Paused,
            "cancelled" => SubscriptionStatus::Cancelled,
            "expired" => SubscriptionStatus::Expired,
            _ => SubscriptionStatus::Active,
        }
    }

    pub fn to_proto(&self) -> i32 {
        match self {
            SubscriptionStatus::Trial => 1,
            SubscriptionStatus::Active => 2,
            SubscriptionStatus::Paused => 3,
            SubscriptionStatus::Cancelled => 4,
            SubscriptionStatus::Expired => 5,
        }
    }

    pub fn from_proto(value: i32) -> Self {
        match value {
            1 => SubscriptionStatus::Trial,
            2 => SubscriptionStatus::Active,
            3 => SubscriptionStatus::Paused,
            4 => SubscriptionStatus::Cancelled,
            5 => SubscriptionStatus::Expired,
            _ => SubscriptionStatus::Active,
        }
    }
}

/// Proration mode for plan changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProrationMode {
    Immediate,
    NextCycle,
    None,
}

impl ProrationMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            ProrationMode::Immediate => "immediate",
            ProrationMode::NextCycle => "next_cycle",
            ProrationMode::None => "none",
        }
    }

    pub fn from_string(s: &str) -> Self {
        match s {
            "next_cycle" => ProrationMode::NextCycle,
            "none" => ProrationMode::None,
            _ => ProrationMode::Immediate,
        }
    }

    pub fn to_proto(&self) -> i32 {
        match self {
            ProrationMode::Immediate => 1,
            ProrationMode::NextCycle => 2,
            ProrationMode::None => 3,
        }
    }

    pub fn from_proto(value: i32) -> Self {
        match value {
            1 => ProrationMode::Immediate,
            2 => ProrationMode::NextCycle,
            3 => ProrationMode::None,
            _ => ProrationMode::Immediate,
        }
    }
}

/// Subscription.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Subscription {
    pub subscription_id: Uuid,
    pub tenant_id: Uuid,
    pub customer_id: Uuid,
    pub plan_id: Uuid,
    pub status: String,
    pub billing_anchor_day: i32,
    pub start_date: NaiveDate,
    pub end_date: Option<NaiveDate>,
    pub trial_end_date: Option<NaiveDate>,
    pub current_period_start: NaiveDate,
    pub current_period_end: NaiveDate,
    pub proration_mode: String,
    pub pending_plan_id: Option<Uuid>,
    pub metadata: Option<serde_json::Value>,
    pub created_utc: DateTime<Utc>,
    pub updated_utc: DateTime<Utc>,
}

/// Input for creating a subscription.
#[derive(Debug, Clone)]
pub struct CreateSubscription {
    pub tenant_id: Uuid,
    pub customer_id: Uuid,
    pub plan_id: Uuid,
    pub billing_anchor_day: i32,
    pub start_date: NaiveDate,
    pub trial_end_date: Option<NaiveDate>,
    pub proration_mode: ProrationMode,
    pub metadata: Option<serde_json::Value>,
}

/// Filter parameters for listing subscriptions.
#[derive(Debug, Clone, Default)]
pub struct ListSubscriptionsFilter {
    pub customer_id: Option<Uuid>,
    pub status: Option<SubscriptionStatus>,
    pub plan_id: Option<Uuid>,
    pub page_size: i32,
    pub page_token: Option<Uuid>,
}
