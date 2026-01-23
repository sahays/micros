-- Billing Service Initial Schema
-- Creates tables for subscription billing and usage tracking

-- billing_plans: Templates defining pricing and billing intervals
CREATE TABLE IF NOT EXISTS billing_plans (
    plan_id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL,
    name VARCHAR(100) NOT NULL,
    description TEXT,
    billing_interval VARCHAR(20) NOT NULL CHECK (billing_interval IN ('daily', 'weekly', 'monthly', 'quarterly', 'annually')),
    interval_count INTEGER NOT NULL DEFAULT 1,
    base_price DECIMAL(19,4) NOT NULL,
    currency VARCHAR(3) NOT NULL,
    tax_rate_id UUID,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    is_archived BOOLEAN NOT NULL DEFAULT FALSE,
    metadata JSONB,
    created_utc TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_utc TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_billing_plans_tenant ON billing_plans(tenant_id);
CREATE INDEX IF NOT EXISTS idx_billing_plans_tenant_active ON billing_plans(tenant_id, is_active) WHERE is_active = TRUE AND is_archived = FALSE;

-- usage_components: Metered usage components within a plan
CREATE TABLE IF NOT EXISTS usage_components (
    component_id UUID PRIMARY KEY,
    plan_id UUID NOT NULL REFERENCES billing_plans(plan_id) ON DELETE CASCADE,
    name VARCHAR(100) NOT NULL,
    unit_name VARCHAR(50) NOT NULL,
    unit_price DECIMAL(19,4) NOT NULL,
    included_units INTEGER NOT NULL DEFAULT 0,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_utc TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_usage_components_plan ON usage_components(plan_id);

-- subscriptions: Customer agreements for recurring billing
CREATE TABLE IF NOT EXISTS subscriptions (
    subscription_id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL,
    customer_id UUID NOT NULL,
    plan_id UUID NOT NULL REFERENCES billing_plans(plan_id),
    status VARCHAR(20) NOT NULL DEFAULT 'active' CHECK (status IN ('trial', 'active', 'paused', 'cancelled', 'expired')),
    billing_anchor_day INTEGER NOT NULL CHECK (billing_anchor_day BETWEEN 1 AND 31),
    start_date DATE NOT NULL,
    end_date DATE,
    trial_end_date DATE,
    current_period_start DATE NOT NULL,
    current_period_end DATE NOT NULL,
    proration_mode VARCHAR(20) NOT NULL DEFAULT 'immediate' CHECK (proration_mode IN ('immediate', 'next_cycle', 'none')),
    pending_plan_id UUID REFERENCES billing_plans(plan_id),
    metadata JSONB,
    created_utc TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_utc TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_subscriptions_tenant ON subscriptions(tenant_id);
CREATE INDEX IF NOT EXISTS idx_subscriptions_customer ON subscriptions(tenant_id, customer_id);
CREATE INDEX IF NOT EXISTS idx_subscriptions_plan ON subscriptions(plan_id);
CREATE INDEX IF NOT EXISTS idx_subscriptions_status ON subscriptions(tenant_id, status);
CREATE INDEX IF NOT EXISTS idx_subscriptions_period_end ON subscriptions(current_period_end) WHERE status IN ('trial', 'active');

-- billing_cycles: Individual billing periods for a subscription
CREATE TABLE IF NOT EXISTS billing_cycles (
    cycle_id UUID PRIMARY KEY,
    subscription_id UUID NOT NULL REFERENCES subscriptions(subscription_id) ON DELETE CASCADE,
    period_start DATE NOT NULL,
    period_end DATE NOT NULL,
    status VARCHAR(20) NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'invoiced', 'paid', 'failed')),
    invoice_id UUID,
    created_utc TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_utc TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_billing_cycles_subscription ON billing_cycles(subscription_id);
CREATE INDEX IF NOT EXISTS idx_billing_cycles_status ON billing_cycles(status);
CREATE INDEX IF NOT EXISTS idx_billing_cycles_period ON billing_cycles(subscription_id, period_start, period_end);

-- charges: Individual billable items within a cycle
CREATE TABLE IF NOT EXISTS charges (
    charge_id UUID PRIMARY KEY,
    cycle_id UUID NOT NULL REFERENCES billing_cycles(cycle_id) ON DELETE CASCADE,
    charge_type VARCHAR(20) NOT NULL CHECK (charge_type IN ('recurring', 'usage', 'one_time', 'proration')),
    description TEXT NOT NULL,
    quantity DECIMAL(19,4) NOT NULL,
    unit_price DECIMAL(19,4) NOT NULL,
    amount DECIMAL(19,4) NOT NULL,
    is_prorated BOOLEAN NOT NULL DEFAULT FALSE,
    proration_factor DECIMAL(10,6),
    component_id UUID REFERENCES usage_components(component_id),
    metadata JSONB,
    created_utc TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_charges_cycle ON charges(cycle_id);
CREATE INDEX IF NOT EXISTS idx_charges_type ON charges(cycle_id, charge_type);

-- usage_records: Metered usage events for billing
CREATE TABLE IF NOT EXISTS usage_records (
    record_id UUID PRIMARY KEY,
    subscription_id UUID NOT NULL REFERENCES subscriptions(subscription_id) ON DELETE CASCADE,
    component_id UUID NOT NULL REFERENCES usage_components(component_id),
    idempotency_key VARCHAR(255) NOT NULL,
    quantity DECIMAL(19,4) NOT NULL,
    timestamp TIMESTAMPTZ NOT NULL,
    cycle_id UUID REFERENCES billing_cycles(cycle_id),
    is_invoiced BOOLEAN NOT NULL DEFAULT FALSE,
    metadata JSONB,
    created_utc TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT uq_usage_idempotency UNIQUE (idempotency_key)
);

CREATE INDEX IF NOT EXISTS idx_usage_records_subscription ON usage_records(subscription_id);
CREATE INDEX IF NOT EXISTS idx_usage_records_component ON usage_records(subscription_id, component_id);
CREATE INDEX IF NOT EXISTS idx_usage_records_cycle ON usage_records(cycle_id);
CREATE INDEX IF NOT EXISTS idx_usage_records_invoiced ON usage_records(subscription_id, is_invoiced) WHERE is_invoiced = FALSE;

-- billing_runs: Batch execution of billing for subscriptions
CREATE TABLE IF NOT EXISTS billing_runs (
    run_id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL,
    run_type VARCHAR(20) NOT NULL CHECK (run_type IN ('scheduled', 'manual', 'single')),
    status VARCHAR(20) NOT NULL DEFAULT 'running' CHECK (status IN ('running', 'completed', 'failed')),
    started_utc TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_utc TIMESTAMPTZ,
    subscriptions_processed INTEGER NOT NULL DEFAULT 0,
    subscriptions_succeeded INTEGER NOT NULL DEFAULT 0,
    subscriptions_failed INTEGER NOT NULL DEFAULT 0,
    error_message TEXT
);

CREATE INDEX IF NOT EXISTS idx_billing_runs_tenant ON billing_runs(tenant_id);
CREATE INDEX IF NOT EXISTS idx_billing_runs_status ON billing_runs(tenant_id, status);

-- billing_run_results: Per-subscription results from a billing run
CREATE TABLE IF NOT EXISTS billing_run_results (
    result_id UUID PRIMARY KEY,
    run_id UUID NOT NULL REFERENCES billing_runs(run_id) ON DELETE CASCADE,
    subscription_id UUID NOT NULL REFERENCES subscriptions(subscription_id),
    status VARCHAR(20) NOT NULL CHECK (status IN ('success', 'failed')),
    invoice_id UUID,
    error_message TEXT,
    created_utc TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_billing_run_results_run ON billing_run_results(run_id);
CREATE INDEX IF NOT EXISTS idx_billing_run_results_subscription ON billing_run_results(subscription_id);

-- Trigger to update updated_utc on billing_plans
CREATE OR REPLACE FUNCTION update_billing_plans_updated_utc()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_utc = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_billing_plans_updated ON billing_plans;
CREATE TRIGGER trg_billing_plans_updated
    BEFORE UPDATE ON billing_plans
    FOR EACH ROW
    EXECUTE FUNCTION update_billing_plans_updated_utc();

-- Trigger to update updated_utc on subscriptions
CREATE OR REPLACE FUNCTION update_subscriptions_updated_utc()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_utc = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_subscriptions_updated ON subscriptions;
CREATE TRIGGER trg_subscriptions_updated
    BEFORE UPDATE ON subscriptions
    FOR EACH ROW
    EXECUTE FUNCTION update_subscriptions_updated_utc();

-- Trigger to update updated_utc on billing_cycles
CREATE OR REPLACE FUNCTION update_billing_cycles_updated_utc()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_utc = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_billing_cycles_updated ON billing_cycles;
CREATE TRIGGER trg_billing_cycles_updated
    BEFORE UPDATE ON billing_cycles
    FOR EACH ROW
    EXECUTE FUNCTION update_billing_cycles_updated_utc();
