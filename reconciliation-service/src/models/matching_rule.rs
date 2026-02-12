use crate::grpc::proto;
use chrono::{DateTime, Utc};
use sqlx::FromRow;
use uuid::Uuid;

use super::datetime_to_timestamp;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchType {
    Contains,
    Exact,
    Regex,
    StartsWith,
    EndsWith,
}

impl MatchType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Contains => "contains",
            Self::Exact => "exact",
            Self::Regex => "regex",
            Self::StartsWith => "starts_with",
            Self::EndsWith => "ends_with",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "contains" => Self::Contains,
            "exact" => Self::Exact,
            "regex" => Self::Regex,
            "starts_with" => Self::StartsWith,
            "ends_with" => Self::EndsWith,
            _ => Self::Contains,
        }
    }

    pub fn from_proto(p: proto::MatchType) -> Self {
        match p {
            proto::MatchType::Contains => Self::Contains,
            proto::MatchType::Exact => Self::Exact,
            proto::MatchType::Regex => Self::Regex,
            proto::MatchType::StartsWith => Self::StartsWith,
            proto::MatchType::EndsWith => Self::EndsWith,
            _ => Self::Contains,
        }
    }
}

impl From<MatchType> for proto::MatchType {
    fn from(m: MatchType) -> Self {
        match m {
            MatchType::Contains => Self::Contains,
            MatchType::Exact => Self::Exact,
            MatchType::Regex => Self::Regex,
            MatchType::StartsWith => Self::StartsWith,
            MatchType::EndsWith => Self::EndsWith,
        }
    }
}

#[derive(Debug, Clone, FromRow)]
pub struct MatchingRule {
    pub rule_id: Uuid,
    pub tenant_id: Uuid,
    pub name: String,
    pub description_pattern: String,
    pub match_type: String,
    pub target_account_id: Option<Uuid>,
    pub priority: i32,
    pub is_active: bool,
    pub created_utc: DateTime<Utc>,
}

impl From<MatchingRule> for proto::MatchingRule {
    fn from(r: MatchingRule) -> Self {
        Self {
            rule_id: r.rule_id.to_string(),
            tenant_id: r.tenant_id.to_string(),
            name: r.name,
            description_pattern: r.description_pattern,
            match_type: proto::MatchType::from(MatchType::from_str(&r.match_type)).into(),
            target_account_id: r.target_account_id.map(|id| id.to_string()),
            priority: r.priority,
            is_active: r.is_active,
            created_utc: Some(datetime_to_timestamp(r.created_utc)),
        }
    }
}
