-- Reconciliation Service Initial Schema
-- Creates tables for bank statement reconciliation with AI-powered matching

-- bank_accounts: Bank accounts registered for reconciliation
CREATE TABLE IF NOT EXISTS bank_accounts (
    bank_account_id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL,
    ledger_account_id UUID NOT NULL,
    bank_name VARCHAR(100) NOT NULL,
    account_number_masked VARCHAR(20) NOT NULL,
    currency VARCHAR(3) NOT NULL,
    last_reconciled_date DATE,
    last_reconciled_balance DECIMAL(19,4),
    created_utc TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_utc TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_bank_accounts_tenant ON bank_accounts(tenant_id);
CREATE INDEX IF NOT EXISTS idx_bank_accounts_ledger ON bank_accounts(tenant_id, ledger_account_id);

-- bank_statements: Imported statement files
CREATE TABLE IF NOT EXISTS bank_statements (
    statement_id UUID PRIMARY KEY,
    bank_account_id UUID NOT NULL REFERENCES bank_accounts(bank_account_id) ON DELETE CASCADE,
    tenant_id UUID NOT NULL,
    document_id UUID,
    period_start DATE NOT NULL,
    period_end DATE NOT NULL,
    opening_balance DECIMAL(19,4) NOT NULL,
    closing_balance DECIMAL(19,4) NOT NULL,
    status VARCHAR(20) NOT NULL DEFAULT 'uploaded' CHECK (status IN ('uploaded', 'extracting', 'staged', 'committed', 'reconciling', 'reconciled', 'failed', 'abandoned')),
    error_message TEXT,
    extraction_confidence DOUBLE PRECISION,
    created_utc TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_utc TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_bank_statements_tenant ON bank_statements(tenant_id);
CREATE INDEX IF NOT EXISTS idx_bank_statements_account ON bank_statements(bank_account_id);
CREATE INDEX IF NOT EXISTS idx_bank_statements_status ON bank_statements(tenant_id, status);
CREATE INDEX IF NOT EXISTS idx_bank_statements_period ON bank_statements(bank_account_id, period_start, period_end);

-- bank_transactions: Individual transactions from statements
CREATE TABLE IF NOT EXISTS bank_transactions (
    transaction_id UUID PRIMARY KEY,
    statement_id UUID NOT NULL REFERENCES bank_statements(statement_id) ON DELETE CASCADE,
    tenant_id UUID NOT NULL,
    transaction_date DATE NOT NULL,
    description TEXT NOT NULL,
    reference VARCHAR(100),
    amount DECIMAL(19,4) NOT NULL,
    running_balance DECIMAL(19,4),
    status VARCHAR(20) NOT NULL DEFAULT 'staged' CHECK (status IN ('staged', 'unmatched', 'matched', 'manually_matched', 'excluded')),
    extraction_confidence DOUBLE PRECISION,
    is_modified BOOLEAN NOT NULL DEFAULT FALSE,
    created_utc TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_bank_transactions_tenant ON bank_transactions(tenant_id);
CREATE INDEX IF NOT EXISTS idx_bank_transactions_statement ON bank_transactions(statement_id);
CREATE INDEX IF NOT EXISTS idx_bank_transactions_status ON bank_transactions(statement_id, status);
CREATE INDEX IF NOT EXISTS idx_bank_transactions_date ON bank_transactions(tenant_id, transaction_date);

-- matching_rules: User-defined rules for automatic matching
CREATE TABLE IF NOT EXISTS matching_rules (
    rule_id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL,
    name VARCHAR(100) NOT NULL,
    description_pattern VARCHAR(255) NOT NULL,
    match_type VARCHAR(20) NOT NULL CHECK (match_type IN ('contains', 'exact', 'regex', 'starts_with', 'ends_with')),
    target_account_id UUID,
    priority INTEGER NOT NULL DEFAULT 0,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_utc TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_matching_rules_tenant ON matching_rules(tenant_id);
CREATE INDEX IF NOT EXISTS idx_matching_rules_active ON matching_rules(tenant_id, is_active, priority);

-- transaction_matches: Links between bank transactions and ledger entries
CREATE TABLE IF NOT EXISTS transaction_matches (
    match_id UUID PRIMARY KEY,
    bank_transaction_id UUID NOT NULL REFERENCES bank_transactions(transaction_id) ON DELETE CASCADE,
    ledger_entry_id UUID NOT NULL,
    match_method VARCHAR(20) NOT NULL CHECK (match_method IN ('auto', 'manual', 'ai_confirmed')),
    confidence_score DOUBLE PRECISION,
    matched_by VARCHAR(100),
    matched_utc TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_transaction_matches_bank_txn ON transaction_matches(bank_transaction_id);
CREATE INDEX IF NOT EXISTS idx_transaction_matches_ledger ON transaction_matches(ledger_entry_id);

-- ai_suggestions: AI-generated match suggestions awaiting confirmation
CREATE TABLE IF NOT EXISTS ai_suggestions (
    suggestion_id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL,
    bank_transaction_id UUID NOT NULL REFERENCES bank_transactions(transaction_id) ON DELETE CASCADE,
    ledger_entry_id UUID NOT NULL,
    confidence_score DOUBLE PRECISION NOT NULL,
    explanation TEXT,
    status VARCHAR(20) NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'confirmed', 'rejected')),
    created_utc TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    resolved_utc TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_ai_suggestions_tenant ON ai_suggestions(tenant_id);
CREATE INDEX IF NOT EXISTS idx_ai_suggestions_bank_txn ON ai_suggestions(bank_transaction_id);
CREATE INDEX IF NOT EXISTS idx_ai_suggestions_pending ON ai_suggestions(tenant_id, status) WHERE status = 'pending';

-- reconciliations: Reconciliation sessions
CREATE TABLE IF NOT EXISTS reconciliations (
    reconciliation_id UUID PRIMARY KEY,
    bank_account_id UUID NOT NULL REFERENCES bank_accounts(bank_account_id) ON DELETE CASCADE,
    tenant_id UUID NOT NULL,
    period_start DATE NOT NULL,
    period_end DATE NOT NULL,
    expected_balance DECIMAL(19,4) NOT NULL,
    actual_balance DECIMAL(19,4) NOT NULL,
    difference DECIMAL(19,4) NOT NULL,
    status VARCHAR(20) NOT NULL DEFAULT 'in_progress' CHECK (status IN ('in_progress', 'completed', 'abandoned')),
    matched_count INTEGER NOT NULL DEFAULT 0,
    unmatched_count INTEGER NOT NULL DEFAULT 0,
    started_utc TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_utc TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_reconciliations_tenant ON reconciliations(tenant_id);
CREATE INDEX IF NOT EXISTS idx_reconciliations_account ON reconciliations(bank_account_id);
CREATE INDEX IF NOT EXISTS idx_reconciliations_status ON reconciliations(tenant_id, status);
CREATE INDEX IF NOT EXISTS idx_reconciliations_period ON reconciliations(bank_account_id, period_start, period_end);

-- adjustments: Adjustment entries for discrepancies
CREATE TABLE IF NOT EXISTS adjustments (
    adjustment_id UUID PRIMARY KEY,
    reconciliation_id UUID NOT NULL REFERENCES reconciliations(reconciliation_id) ON DELETE CASCADE,
    tenant_id UUID NOT NULL,
    adjustment_type VARCHAR(20) NOT NULL CHECK (adjustment_type IN ('bank_fee', 'bank_interest', 'correction', 'timing_difference', 'other')),
    description TEXT NOT NULL,
    amount DECIMAL(19,4) NOT NULL,
    ledger_entry_id UUID,
    created_utc TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_adjustments_tenant ON adjustments(tenant_id);
CREATE INDEX IF NOT EXISTS idx_adjustments_reconciliation ON adjustments(reconciliation_id);

-- extraction_feedback: Learning from user corrections
CREATE TABLE IF NOT EXISTS extraction_feedback (
    feedback_id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL,
    statement_id UUID NOT NULL REFERENCES bank_statements(statement_id) ON DELETE CASCADE,
    transaction_id UUID REFERENCES bank_transactions(transaction_id) ON DELETE CASCADE,
    field_name VARCHAR(50) NOT NULL,
    original_value TEXT,
    corrected_value TEXT NOT NULL,
    created_utc TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_extraction_feedback_tenant ON extraction_feedback(tenant_id);
CREATE INDEX IF NOT EXISTS idx_extraction_feedback_statement ON extraction_feedback(statement_id);

-- Trigger to update updated_utc on bank_accounts
CREATE OR REPLACE FUNCTION update_bank_accounts_updated_utc()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_utc = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_bank_accounts_updated ON bank_accounts;
CREATE TRIGGER trg_bank_accounts_updated
    BEFORE UPDATE ON bank_accounts
    FOR EACH ROW
    EXECUTE FUNCTION update_bank_accounts_updated_utc();

-- Trigger to update updated_utc on bank_statements
CREATE OR REPLACE FUNCTION update_bank_statements_updated_utc()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_utc = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_bank_statements_updated ON bank_statements;
CREATE TRIGGER trg_bank_statements_updated
    BEFORE UPDATE ON bank_statements
    FOR EACH ROW
    EXECUTE FUNCTION update_bank_statements_updated_utc();
