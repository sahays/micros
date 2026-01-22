-- Ledger Service Schema
-- Double-entry accounting with multi-tenant support

-- Accounts table
CREATE TABLE accounts (
    account_id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL,
    account_type VARCHAR(20) NOT NULL CHECK (account_type IN ('asset', 'liability', 'equity', 'revenue', 'expense')),
    account_code VARCHAR(100) NOT NULL,
    currency VARCHAR(3) NOT NULL,
    allow_negative BOOLEAN NOT NULL DEFAULT FALSE,
    metadata JSONB,
    created_utc TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    closed_utc TIMESTAMPTZ,
    UNIQUE(tenant_id, account_code)
);

CREATE INDEX idx_accounts_tenant ON accounts(tenant_id);
CREATE INDEX idx_accounts_type ON accounts(tenant_id, account_type);

-- Ledger entries table
CREATE TABLE ledger_entries (
    entry_id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL,
    journal_id UUID NOT NULL,
    account_id UUID NOT NULL REFERENCES accounts(account_id),
    amount DECIMAL(19, 4) NOT NULL CHECK (amount > 0),
    direction VARCHAR(6) NOT NULL CHECK (direction IN ('debit', 'credit')),
    effective_date DATE NOT NULL,
    posted_utc TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    idempotency_key VARCHAR(255),
    metadata JSONB
);

CREATE INDEX idx_entries_journal ON ledger_entries(journal_id);
CREATE INDEX idx_entries_account_date ON ledger_entries(account_id, effective_date);
CREATE INDEX idx_entries_tenant_date ON ledger_entries(tenant_id, effective_date DESC);
CREATE UNIQUE INDEX idx_entries_idempotency ON ledger_entries(idempotency_key) WHERE idempotency_key IS NOT NULL;

-- Function to validate double-entry constraint
-- Sum of debits must equal sum of credits for each journal_id
CREATE OR REPLACE FUNCTION check_double_entry()
RETURNS TRIGGER AS $$
DECLARE
    debit_sum DECIMAL(19, 4);
    credit_sum DECIMAL(19, 4);
BEGIN
    SELECT
        COALESCE(SUM(CASE WHEN direction = 'debit' THEN amount ELSE 0 END), 0),
        COALESCE(SUM(CASE WHEN direction = 'credit' THEN amount ELSE 0 END), 0)
    INTO debit_sum, credit_sum
    FROM ledger_entries
    WHERE journal_id = NEW.journal_id;

    IF debit_sum != credit_sum THEN
        RAISE EXCEPTION 'Double-entry violation: debits (%) != credits (%) for journal %',
            debit_sum, credit_sum, NEW.journal_id;
    END IF;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Note: Trigger is applied per-statement after all entries are inserted
-- This allows inserting multiple entries in a single transaction
CREATE CONSTRAINT TRIGGER enforce_double_entry
    AFTER INSERT ON ledger_entries
    DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW
    EXECUTE FUNCTION check_double_entry();
