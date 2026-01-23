# Epic: Invoicing Service

Status: in_progress
Created: 2026-01-23
Updated: 2026-01-23

## Overview

Invoice generation and management service for multi-tenant operations. Handles invoices, receipts, customer statements, tax calculations, and PDF generation. Integrates with ledger-service for accounting entries.

## Core Principles

- Immutable issued invoices: Only drafts can be modified
- Tax compliance: Configurable tax rates with effective dates
- Audit trail: Full history of invoice lifecycle
- Multi-tenant: Complete isolation via tenant_id
- Ledger integration: AR and revenue entries on issue/payment

## Tech Stack

- Rust + Tonic (gRPC) + Axum (HTTP health/metrics)
- PostgreSQL + sqlx
- PDF generation (printpdf or typst) - deferred
- Prometheus metrics, OpenTelemetry tracing

## Stories

### Phase 1: Foundation

- [x] [001-project-setup](../stories/001-project-setup.md) - Project scaffolding, PostgreSQL, migrations
- [x] [002-invoice-management](../stories/002-invoice-management.md) - CreateInvoice, GetInvoice, UpdateInvoice, ListInvoices
- [x] [003-line-items](../stories/003-line-items.md) - AddLineItem, UpdateLineItem, RemoveLineItem

### Phase 2: Lifecycle

- [x] [004-invoice-lifecycle](../stories/004-invoice-lifecycle.md) - IssueInvoice, VoidInvoice with ledger integration
- [x] [005-tax-rates](../stories/005-tax-rates.md) - CreateTaxRate, GetTaxRate, ListTaxRates, UpdateTaxRate

### Phase 3: Payments

- [x] [006-payments-receipts](../stories/006-payments-receipts.md) - RecordPayment, GetReceipt, ListReceipts
- [x] [007-statements](../stories/007-statements.md) - GenerateStatement

### Phase 4: Documents

- [ ] [008-pdf-generation](../stories/008-pdf-generation.md) - Invoice, receipt, statement PDFs (deferred)
- [x] [009-observability](../stories/009-observability.md) - Metrics, tracing, structured logging

## gRPC Service

| Method | Story | Status |
|--------|-------|--------|
| CreateInvoice | 002 | ✅ Implemented |
| GetInvoice | 002 | ✅ Implemented |
| UpdateInvoice | 002 | ✅ Implemented |
| ListInvoices | 002 | ✅ Implemented |
| DeleteInvoice | 002 | ✅ Implemented |
| AddLineItem | 003 | ✅ Implemented |
| UpdateLineItem | 003 | ✅ Implemented |
| RemoveLineItem | 003 | ✅ Implemented |
| IssueInvoice | 004 | ✅ Implemented (with ledger) |
| VoidInvoice | 004 | ✅ Implemented (with ledger) |
| CreateTaxRate | 005 | ✅ Implemented |
| GetTaxRate | 005 | ✅ Implemented |
| ListTaxRates | 005 | ✅ Implemented |
| UpdateTaxRate | 005 | ✅ Implemented |
| RecordPayment | 006 | ✅ Implemented (with ledger) |
| GetReceipt | 006 | ✅ Implemented |
| ListReceipts | 006 | ✅ Implemented |
| GenerateStatement | 007 | ✅ Implemented |
| GenerateInvoicePdf | 008 | ⏸️ Deferred |
| GenerateReceiptPdf | 008 | ⏸️ Deferred |
| GenerateStatementPdf | 008 | ⏸️ Deferred |

## Completed Work

### Must Have (Done)
- [x] **UpdateInvoice** - Implemented handler (draft invoices only)
- [x] **GenerateStatement** - Statement generation with opening/closing balance
- [x] **Ledger Integration** - LedgerClient in service-core with retry support
  - IssueInvoice: Debit A/R, Credit Revenue
  - VoidInvoice: Reversing entry (Credit A/R, Debit Revenue)
  - RecordPayment: Debit Cash, Credit A/R
- [x] **Overdue Status** - Computed on read (checks due_date vs current date)

### Deferred

- [ ] **PDF Generation** - All three PDF endpoints (story 008)
  - Requires: PDF library (typst/printpdf), template system
  - Note: Spec mentions "customizable templates per tenant" as future
- [ ] **document-service integration** - Store generated PDFs
- [ ] **notification-service integration** - Email invoices to customers

## Acceptance Criteria

- [x] Project scaffolding complete
- [x] PostgreSQL schema and migrations working
- [x] Invoice CRUD operations (CreateInvoice, GetInvoice, UpdateInvoice, ListInvoices, DeleteInvoice)
- [x] Line item management (AddLineItem, UpdateLineItem, RemoveLineItem)
- [x] Tax rate CRUD (CreateTaxRate, GetTaxRate, ListTaxRates, UpdateTaxRate)
- [x] Payment recording and receipts (RecordPayment, GetReceipt, ListReceipts)
- [x] Invoice lifecycle with ledger integration (IssueInvoice, VoidInvoice)
- [x] Statement generation (GenerateStatement)
- [x] Overdue status computation
- [ ] PDF generation (deferred)
- [x] Multi-tenant isolation verified
- [x] Prometheus metrics exposed
- [x] Health and readiness endpoints working
- [x] OpenTelemetry tracing configured

## Architecture Notes

### Ledger Integration

The invoicing-service integrates with ledger-service via gRPC:

1. **service-core/src/grpc/ledger_client.rs** - Reusable LedgerClient with retry support
2. **service-core/src/grpc/retry.rs** - Retry utilities with exponential backoff
3. Connection is optional - service degrades gracefully if ledger-service unavailable

### Account Conventions

- A/R Account: `AR-{currency}` (e.g., `AR-USD`)
- Revenue Account: Line item's `ledger_account_id` or `REVENUE-{currency}`
- Cash Account: `CASH-{payment_method}-{currency}` (e.g., `CASH-CARD-USD`)

### Overdue Computation

Overdue status is computed at read time in `invoice_to_proto()`:
- If status is "issued" AND due_date < today AND amount_due > 0 → OVERDUE
- No background job required
