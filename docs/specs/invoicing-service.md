# Invoicing Service

## Purpose

Generate and manage invoices, receipts, and customer statements. Handles document creation, tax calculations, and payment tracking.

## Domain

### Invoice
A request for payment sent to a customer.

- Identified by tenant-unique invoice number (e.g., INV-202601-0042)
- Contains line items with descriptions, quantities, unit prices
- Supports multiple tax rates per line item
- Tracks payment status: draft → issued → paid/void/overdue
- Links to customer and their billing address
- Supports standard invoices, credit notes, and proforma invoices

### Receipt
Proof of payment received against an invoice.

- Generated when payment is recorded
- Contains payment method, amount, reference number
- One invoice can have multiple receipts (partial payments)

### Statement
Summary of customer account activity over a period.

- Shows opening balance, invoices issued, payments received, closing balance
- Generated on-demand for any date range
- Used for customer account reconciliation

### Tax Rate
Configurable tax rules per tenant.

- Named rates (e.g., "GST 18%", "VAT 20%")
- Inclusive or exclusive calculation
- Effective date ranges for rate changes

## Key Operations

**Invoice Management**
- Create draft invoice with line items
- Add/update/remove line items on draft
- Issue invoice (finalizes, assigns number, creates ledger entry)
- Void invoice (creates reversing ledger entry)
- List invoices with filters (status, customer, date range)

**Payment Processing**
- Record payment against invoice (full or partial)
- Generate receipt for payment
- Auto-update invoice status when fully paid

**Statement Generation**
- Generate statement for customer and date range
- Calculate opening/closing balances from invoice and payment history

**PDF Generation**
- Generate PDF for invoice, receipt, or statement
- Customizable templates per tenant (future)

## Ledger Integration

**On Invoice Issue:**
- Debit: Accounts Receivable (customer)
- Credit: Revenue (per line item account)
- Credit: Tax Payable (if applicable)

**On Payment Receipt:**
- Debit: Cash/Bank
- Credit: Accounts Receivable (customer)

**On Invoice Void:**
- Reverse the original journal entry

## Business Rules

1. Invoice numbers are auto-generated, sequential per tenant per month
2. Draft invoices can be modified; issued invoices are immutable
3. Only draft invoices can be deleted; issued invoices must be voided
4. Credit notes reference the original invoice and create negative entries
5. Overdue status is computed from due_date vs current date
6. All monetary amounts use 4 decimal places for precision
7. Currency is set at invoice level; all line items use same currency

## Dependencies

- **ledger-service**: Create journal entries for AR, revenue, payments
- **document-service**: Store generated PDFs (optional)
- **notification-service**: Email invoices to customers (optional)
