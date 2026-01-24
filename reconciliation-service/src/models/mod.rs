//! Domain models for reconciliation-service.

#![allow(clippy::should_implement_trait)]

use crate::grpc::proto;
use chrono::{DateTime, NaiveDate, Utc};
use prost_types::Timestamp;
use rust_decimal::Decimal;
use sqlx::FromRow;
use uuid::Uuid;

// ============================================================================
// Bank Account Models
// ============================================================================

#[derive(Debug, Clone, FromRow)]
pub struct BankAccount {
    pub bank_account_id: Uuid,
    pub tenant_id: Uuid,
    pub ledger_account_id: Uuid,
    pub bank_name: String,
    pub account_number_masked: String,
    pub currency: String,
    pub last_reconciled_date: Option<NaiveDate>,
    pub last_reconciled_balance: Option<Decimal>,
    pub created_utc: DateTime<Utc>,
    pub updated_utc: DateTime<Utc>,
}

impl From<BankAccount> for proto::BankAccount {
    fn from(a: BankAccount) -> Self {
        Self {
            bank_account_id: a.bank_account_id.to_string(),
            tenant_id: a.tenant_id.to_string(),
            ledger_account_id: a.ledger_account_id.to_string(),
            bank_name: a.bank_name,
            account_number_masked: a.account_number_masked,
            currency: a.currency,
            last_reconciled_date: a.last_reconciled_date.map(|d| d.to_string()),
            last_reconciled_balance: a.last_reconciled_balance.map(|b| b.to_string()),
            created_utc: Some(datetime_to_timestamp(a.created_utc)),
            updated_utc: Some(datetime_to_timestamp(a.updated_utc)),
        }
    }
}

// ============================================================================
// Statement Models
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatementStatus {
    Uploaded,
    Extracting,
    Staged,
    Committed,
    Reconciling,
    Reconciled,
    Failed,
    Abandoned,
}

impl StatementStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Uploaded => "uploaded",
            Self::Extracting => "extracting",
            Self::Staged => "staged",
            Self::Committed => "committed",
            Self::Reconciling => "reconciling",
            Self::Reconciled => "reconciled",
            Self::Failed => "failed",
            Self::Abandoned => "abandoned",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "uploaded" => Self::Uploaded,
            "extracting" => Self::Extracting,
            "staged" => Self::Staged,
            "committed" => Self::Committed,
            "reconciling" => Self::Reconciling,
            "reconciled" => Self::Reconciled,
            "failed" => Self::Failed,
            "abandoned" => Self::Abandoned,
            _ => Self::Uploaded,
        }
    }
}

impl From<StatementStatus> for proto::StatementStatus {
    fn from(s: StatementStatus) -> Self {
        match s {
            StatementStatus::Uploaded => Self::Uploaded,
            StatementStatus::Extracting => Self::Extracting,
            StatementStatus::Staged => Self::Staged,
            StatementStatus::Committed => Self::Committed,
            StatementStatus::Reconciling => Self::Reconciling,
            StatementStatus::Reconciled => Self::Reconciled,
            StatementStatus::Failed => Self::Failed,
            StatementStatus::Abandoned => Self::Abandoned,
        }
    }
}

#[derive(Debug, Clone, FromRow)]
pub struct BankStatement {
    pub statement_id: Uuid,
    pub bank_account_id: Uuid,
    pub tenant_id: Uuid,
    pub document_id: Option<Uuid>,
    pub period_start: NaiveDate,
    pub period_end: NaiveDate,
    pub opening_balance: Decimal,
    pub closing_balance: Decimal,
    pub status: String,
    pub error_message: Option<String>,
    pub extraction_confidence: Option<f64>,
    pub created_utc: DateTime<Utc>,
    pub updated_utc: DateTime<Utc>,
}

impl From<BankStatement> for proto::BankStatement {
    fn from(s: BankStatement) -> Self {
        Self {
            statement_id: s.statement_id.to_string(),
            bank_account_id: s.bank_account_id.to_string(),
            tenant_id: s.tenant_id.to_string(),
            document_id: s.document_id.map(|d| d.to_string()),
            period_start: s.period_start.to_string(),
            period_end: s.period_end.to_string(),
            opening_balance: s.opening_balance.to_string(),
            closing_balance: s.closing_balance.to_string(),
            status: proto::StatementStatus::from(StatementStatus::from_str(&s.status)).into(),
            error_message: s.error_message,
            extraction_confidence: s.extraction_confidence,
            created_utc: Some(datetime_to_timestamp(s.created_utc)),
            updated_utc: Some(datetime_to_timestamp(s.updated_utc)),
        }
    }
}

// ============================================================================
// Transaction Models
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransactionStatus {
    Staged,
    Unmatched,
    Matched,
    ManuallyMatched,
    Excluded,
}

impl TransactionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Staged => "staged",
            Self::Unmatched => "unmatched",
            Self::Matched => "matched",
            Self::ManuallyMatched => "manually_matched",
            Self::Excluded => "excluded",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "staged" => Self::Staged,
            "unmatched" => Self::Unmatched,
            "matched" => Self::Matched,
            "manually_matched" => Self::ManuallyMatched,
            "excluded" => Self::Excluded,
            _ => Self::Staged,
        }
    }
}

impl From<TransactionStatus> for proto::TransactionStatus {
    fn from(s: TransactionStatus) -> Self {
        match s {
            TransactionStatus::Staged => Self::Staged,
            TransactionStatus::Unmatched => Self::Unmatched,
            TransactionStatus::Matched => Self::Matched,
            TransactionStatus::ManuallyMatched => Self::ManuallyMatched,
            TransactionStatus::Excluded => Self::Excluded,
        }
    }
}

#[derive(Debug, Clone, FromRow)]
pub struct BankTransaction {
    pub transaction_id: Uuid,
    pub statement_id: Uuid,
    pub tenant_id: Uuid,
    pub transaction_date: NaiveDate,
    pub description: String,
    pub reference: Option<String>,
    pub amount: Decimal,
    pub running_balance: Option<Decimal>,
    pub status: String,
    pub extraction_confidence: Option<f64>,
    pub is_modified: bool,
    pub created_utc: DateTime<Utc>,
}

impl From<BankTransaction> for proto::StagedTransaction {
    fn from(t: BankTransaction) -> Self {
        Self {
            transaction_id: t.transaction_id.to_string(),
            statement_id: t.statement_id.to_string(),
            tenant_id: t.tenant_id.to_string(),
            transaction_date: t.transaction_date.to_string(),
            description: t.description,
            reference: t.reference,
            amount: t.amount.to_string(),
            running_balance: t.running_balance.map(|b| b.to_string()),
            status: proto::TransactionStatus::from(TransactionStatus::from_str(&t.status)).into(),
            extraction_confidence: t.extraction_confidence,
            is_modified: t.is_modified,
            created_utc: Some(datetime_to_timestamp(t.created_utc)),
        }
    }
}

// ============================================================================
// Matching Rule Models
// ============================================================================

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

// ============================================================================
// Transaction Match Models
// ============================================================================

#[derive(Debug, Clone, FromRow)]
pub struct TransactionMatch {
    pub match_id: Uuid,
    pub bank_transaction_id: Uuid,
    pub ledger_entry_id: Uuid,
    pub match_method: String,
    pub confidence_score: Option<f64>,
    pub matched_by: Option<String>,
    pub matched_utc: DateTime<Utc>,
}

impl From<TransactionMatch> for proto::TransactionMatch {
    fn from(m: TransactionMatch) -> Self {
        Self {
            match_id: m.match_id.to_string(),
            bank_transaction_id: m.bank_transaction_id.to_string(),
            ledger_entry_id: m.ledger_entry_id.to_string(),
            match_method: m.match_method,
            confidence_score: m.confidence_score,
            matched_by: m.matched_by,
            matched_utc: Some(datetime_to_timestamp(m.matched_utc)),
        }
    }
}

// ============================================================================
// Reconciliation Models
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReconciliationStatus {
    InProgress,
    Completed,
    Abandoned,
}

impl ReconciliationStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::InProgress => "in_progress",
            Self::Completed => "completed",
            Self::Abandoned => "abandoned",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "in_progress" => Self::InProgress,
            "completed" => Self::Completed,
            "abandoned" => Self::Abandoned,
            _ => Self::InProgress,
        }
    }
}

impl From<ReconciliationStatus> for proto::ReconciliationStatus {
    fn from(s: ReconciliationStatus) -> Self {
        match s {
            ReconciliationStatus::InProgress => Self::InProgress,
            ReconciliationStatus::Completed => Self::Completed,
            ReconciliationStatus::Abandoned => Self::Abandoned,
        }
    }
}

#[derive(Debug, Clone, FromRow)]
pub struct Reconciliation {
    pub reconciliation_id: Uuid,
    pub bank_account_id: Uuid,
    pub tenant_id: Uuid,
    pub period_start: NaiveDate,
    pub period_end: NaiveDate,
    pub expected_balance: Decimal,
    pub actual_balance: Decimal,
    pub difference: Decimal,
    pub status: String,
    pub matched_count: i32,
    pub unmatched_count: i32,
    pub started_utc: DateTime<Utc>,
    pub completed_utc: Option<DateTime<Utc>>,
}

impl From<Reconciliation> for proto::Reconciliation {
    fn from(r: Reconciliation) -> Self {
        Self {
            reconciliation_id: r.reconciliation_id.to_string(),
            bank_account_id: r.bank_account_id.to_string(),
            tenant_id: r.tenant_id.to_string(),
            period_start: r.period_start.to_string(),
            period_end: r.period_end.to_string(),
            expected_balance: r.expected_balance.to_string(),
            actual_balance: r.actual_balance.to_string(),
            difference: r.difference.to_string(),
            status: proto::ReconciliationStatus::from(ReconciliationStatus::from_str(&r.status))
                .into(),
            matched_count: r.matched_count,
            unmatched_count: r.unmatched_count,
            started_utc: Some(datetime_to_timestamp(r.started_utc)),
            completed_utc: r.completed_utc.map(datetime_to_timestamp),
        }
    }
}

// ============================================================================
// Adjustment Models
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdjustmentType {
    BankFee,
    BankInterest,
    Correction,
    TimingDifference,
    Other,
}

impl AdjustmentType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::BankFee => "bank_fee",
            Self::BankInterest => "bank_interest",
            Self::Correction => "correction",
            Self::TimingDifference => "timing_difference",
            Self::Other => "other",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "bank_fee" => Self::BankFee,
            "bank_interest" => Self::BankInterest,
            "correction" => Self::Correction,
            "timing_difference" => Self::TimingDifference,
            "other" => Self::Other,
            _ => Self::Other,
        }
    }

    pub fn from_proto(p: proto::AdjustmentType) -> Self {
        match p {
            proto::AdjustmentType::BankFee => Self::BankFee,
            proto::AdjustmentType::BankInterest => Self::BankInterest,
            proto::AdjustmentType::Correction => Self::Correction,
            proto::AdjustmentType::TimingDifference => Self::TimingDifference,
            proto::AdjustmentType::Other => Self::Other,
            _ => Self::Other,
        }
    }
}

impl From<AdjustmentType> for proto::AdjustmentType {
    fn from(a: AdjustmentType) -> Self {
        match a {
            AdjustmentType::BankFee => Self::BankFee,
            AdjustmentType::BankInterest => Self::BankInterest,
            AdjustmentType::Correction => Self::Correction,
            AdjustmentType::TimingDifference => Self::TimingDifference,
            AdjustmentType::Other => Self::Other,
        }
    }
}

#[derive(Debug, Clone, FromRow)]
pub struct Adjustment {
    pub adjustment_id: Uuid,
    pub reconciliation_id: Uuid,
    pub tenant_id: Uuid,
    pub adjustment_type: String,
    pub description: String,
    pub amount: Decimal,
    pub ledger_entry_id: Option<Uuid>,
    pub created_utc: DateTime<Utc>,
}

impl From<Adjustment> for proto::Adjustment {
    fn from(a: Adjustment) -> Self {
        Self {
            adjustment_id: a.adjustment_id.to_string(),
            reconciliation_id: a.reconciliation_id.to_string(),
            tenant_id: a.tenant_id.to_string(),
            adjustment_type: proto::AdjustmentType::from(AdjustmentType::from_str(
                &a.adjustment_type,
            ))
            .into(),
            description: a.description,
            amount: a.amount.to_string(),
            ledger_entry_id: a.ledger_entry_id.map(|id| id.to_string()),
            created_utc: Some(datetime_to_timestamp(a.created_utc)),
        }
    }
}

// ============================================================================
// Utility Functions
// ============================================================================

fn datetime_to_timestamp(dt: DateTime<Utc>) -> Timestamp {
    Timestamp {
        seconds: dt.timestamp(),
        nanos: dt.timestamp_subsec_nanos() as i32,
    }
}
