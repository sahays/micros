-- Invoicing Service Schema
-- Invoice, receipt, and statement management with multi-tenant support

-- Tax rates table
CREATE TABLE tax_rates (
    tax_rate_id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL,
    name VARCHAR(100) NOT NULL,
    rate DECIMAL(10, 6) NOT NULL CHECK (rate >= 0 AND rate <= 1),
    calculation VARCHAR(20) NOT NULL CHECK (calculation IN ('exclusive', 'inclusive')),
    effective_from DATE NOT NULL,
    effective_to DATE,
    active BOOLEAN NOT NULL DEFAULT TRUE,
    created_utc TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(tenant_id, name, effective_from)
);

CREATE INDEX idx_tax_rates_tenant ON tax_rates(tenant_id);
CREATE INDEX idx_tax_rates_active ON tax_rates(tenant_id, active) WHERE active = TRUE;

-- Invoices table
CREATE TABLE invoices (
    invoice_id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL,
    invoice_number VARCHAR(50),
    invoice_type VARCHAR(20) NOT NULL CHECK (invoice_type IN ('standard', 'credit_note', 'proforma')),
    status VARCHAR(20) NOT NULL CHECK (status IN ('draft', 'issued', 'paid', 'void', 'overdue')),
    customer_id UUID NOT NULL,
    customer_name VARCHAR(255) NOT NULL,
    billing_line1 VARCHAR(255),
    billing_line2 VARCHAR(255),
    billing_city VARCHAR(100),
    billing_state VARCHAR(100),
    billing_postal_code VARCHAR(20),
    billing_country VARCHAR(100),
    currency VARCHAR(3) NOT NULL,
    issue_date DATE,
    due_date DATE,
    subtotal DECIMAL(19, 4) NOT NULL DEFAULT 0,
    tax_total DECIMAL(19, 4) NOT NULL DEFAULT 0,
    total DECIMAL(19, 4) NOT NULL DEFAULT 0,
    amount_paid DECIMAL(19, 4) NOT NULL DEFAULT 0,
    amount_due DECIMAL(19, 4) NOT NULL DEFAULT 0,
    notes TEXT,
    reference_invoice_id UUID REFERENCES invoices(invoice_id),
    journal_id UUID,
    metadata JSONB,
    created_utc TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    issued_utc TIMESTAMPTZ,
    voided_utc TIMESTAMPTZ,
    UNIQUE(tenant_id, invoice_number)
);

CREATE INDEX idx_invoices_tenant ON invoices(tenant_id);
CREATE INDEX idx_invoices_customer ON invoices(tenant_id, customer_id);
CREATE INDEX idx_invoices_status ON invoices(tenant_id, status);
CREATE INDEX idx_invoices_issue_date ON invoices(tenant_id, issue_date);

-- Sequence for invoice numbers per tenant per month
-- Format: INV-YYYYMM-NNNN
CREATE TABLE invoice_sequences (
    tenant_id UUID NOT NULL,
    year_month VARCHAR(6) NOT NULL,
    last_number INT NOT NULL DEFAULT 0,
    PRIMARY KEY (tenant_id, year_month)
);

-- Line items table
CREATE TABLE line_items (
    line_item_id UUID PRIMARY KEY,
    invoice_id UUID NOT NULL REFERENCES invoices(invoice_id) ON DELETE CASCADE,
    tenant_id UUID NOT NULL,
    description VARCHAR(500) NOT NULL,
    quantity DECIMAL(19, 4) NOT NULL CHECK (quantity > 0),
    unit_price DECIMAL(19, 4) NOT NULL,
    tax_rate_id UUID REFERENCES tax_rates(tax_rate_id),
    tax_amount DECIMAL(19, 4) NOT NULL DEFAULT 0,
    subtotal DECIMAL(19, 4) NOT NULL,
    total DECIMAL(19, 4) NOT NULL,
    ledger_account_id UUID,
    sort_order INT NOT NULL DEFAULT 0,
    created_utc TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_line_items_invoice ON line_items(invoice_id);
CREATE INDEX idx_line_items_tenant ON line_items(tenant_id);

-- Receipts table
CREATE TABLE receipts (
    receipt_id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL,
    receipt_number VARCHAR(50) NOT NULL,
    invoice_id UUID NOT NULL REFERENCES invoices(invoice_id),
    customer_id UUID NOT NULL,
    amount DECIMAL(19, 4) NOT NULL CHECK (amount > 0),
    currency VARCHAR(3) NOT NULL,
    payment_method VARCHAR(50) NOT NULL,
    payment_reference VARCHAR(255),
    payment_date DATE NOT NULL,
    journal_id UUID,
    notes TEXT,
    created_utc TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(tenant_id, receipt_number)
);

CREATE INDEX idx_receipts_tenant ON receipts(tenant_id);
CREATE INDEX idx_receipts_invoice ON receipts(invoice_id);
CREATE INDEX idx_receipts_customer ON receipts(tenant_id, customer_id);
CREATE INDEX idx_receipts_date ON receipts(tenant_id, payment_date);

-- Sequence for receipt numbers per tenant per month
CREATE TABLE receipt_sequences (
    tenant_id UUID NOT NULL,
    year_month VARCHAR(6) NOT NULL,
    last_number INT NOT NULL DEFAULT 0,
    PRIMARY KEY (tenant_id, year_month)
);

-- Function to generate next invoice number
CREATE OR REPLACE FUNCTION next_invoice_number(p_tenant_id UUID, p_prefix VARCHAR DEFAULT 'INV')
RETURNS VARCHAR AS $$
DECLARE
    v_year_month VARCHAR(6);
    v_next_number INT;
BEGIN
    v_year_month := TO_CHAR(CURRENT_DATE, 'YYYYMM');

    INSERT INTO invoice_sequences (tenant_id, year_month, last_number)
    VALUES (p_tenant_id, v_year_month, 1)
    ON CONFLICT (tenant_id, year_month)
    DO UPDATE SET last_number = invoice_sequences.last_number + 1
    RETURNING last_number INTO v_next_number;

    RETURN p_prefix || '-' || v_year_month || '-' || LPAD(v_next_number::TEXT, 4, '0');
END;
$$ LANGUAGE plpgsql;

-- Function to generate next receipt number
CREATE OR REPLACE FUNCTION next_receipt_number(p_tenant_id UUID, p_prefix VARCHAR DEFAULT 'RCP')
RETURNS VARCHAR AS $$
DECLARE
    v_year_month VARCHAR(6);
    v_next_number INT;
BEGIN
    v_year_month := TO_CHAR(CURRENT_DATE, 'YYYYMM');

    INSERT INTO receipt_sequences (tenant_id, year_month, last_number)
    VALUES (p_tenant_id, v_year_month, 1)
    ON CONFLICT (tenant_id, year_month)
    DO UPDATE SET last_number = receipt_sequences.last_number + 1
    RETURNING last_number INTO v_next_number;

    RETURN p_prefix || '-' || v_year_month || '-' || LPAD(v_next_number::TEXT, 4, '0');
END;
$$ LANGUAGE plpgsql;

-- Trigger to recalculate invoice totals when line items change
CREATE OR REPLACE FUNCTION recalculate_invoice_totals()
RETURNS TRIGGER AS $$
DECLARE
    v_invoice_id UUID;
BEGIN
    IF TG_OP = 'DELETE' THEN
        v_invoice_id := OLD.invoice_id;
    ELSE
        v_invoice_id := NEW.invoice_id;
    END IF;

    UPDATE invoices
    SET subtotal = COALESCE((SELECT SUM(subtotal) FROM line_items WHERE invoice_id = v_invoice_id), 0),
        tax_total = COALESCE((SELECT SUM(tax_amount) FROM line_items WHERE invoice_id = v_invoice_id), 0),
        total = COALESCE((SELECT SUM(total) FROM line_items WHERE invoice_id = v_invoice_id), 0),
        amount_due = COALESCE((SELECT SUM(total) FROM line_items WHERE invoice_id = v_invoice_id), 0) - amount_paid
    WHERE invoice_id = v_invoice_id;

    RETURN NULL;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_recalculate_invoice_totals
    AFTER INSERT OR UPDATE OR DELETE ON line_items
    FOR EACH ROW
    EXECUTE FUNCTION recalculate_invoice_totals();

-- Trigger to update invoice amount_paid and status when receipt is added
CREATE OR REPLACE FUNCTION update_invoice_on_receipt()
RETURNS TRIGGER AS $$
BEGIN
    IF TG_OP = 'INSERT' THEN
        UPDATE invoices
        SET amount_paid = amount_paid + NEW.amount,
            amount_due = amount_due - NEW.amount,
            status = CASE
                WHEN amount_due - NEW.amount <= 0 AND status = 'issued' THEN 'paid'
                ELSE status
            END
        WHERE invoice_id = NEW.invoice_id;
    END IF;

    RETURN NULL;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_update_invoice_on_receipt
    AFTER INSERT ON receipts
    FOR EACH ROW
    EXECUTE FUNCTION update_invoice_on_receipt();
