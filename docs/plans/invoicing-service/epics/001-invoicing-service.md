# Epic: Invoicing Service

Status: planning
Created: 2026-01-23

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
- PDF generation (printpdf or typst)
- Prometheus metrics, OpenTelemetry tracing

## Stories

### Phase 1: Foundation

- [ ] [001-project-setup](../stories/001-project-setup.md) - Project scaffolding, PostgreSQL, migrations
- [ ] [002-invoice-management](../stories/002-invoice-management.md) - CreateInvoice, GetInvoice, UpdateInvoice, ListInvoices
- [ ] [003-line-items](../stories/003-line-items.md) - AddLineItem, UpdateLineItem, RemoveLineItem

### Phase 2: Lifecycle

- [ ] [004-invoice-lifecycle](../stories/004-invoice-lifecycle.md) - IssueInvoice, VoidInvoice with ledger integration
- [ ] [005-tax-rates](../stories/005-tax-rates.md) - CreateTaxRate, ListTaxRates, tax calculations

### Phase 3: Payments

- [ ] [006-payments-receipts](../stories/006-payments-receipts.md) - RecordPayment, GetReceipt, ListReceipts
- [ ] [007-statements](../stories/007-statements.md) - GenerateStatement, GetStatement

### Phase 4: Documents

- [ ] [008-pdf-generation](../stories/008-pdf-generation.md) - Invoice, receipt, statement PDFs
- [ ] [009-observability](../stories/009-observability.md) - Metrics, tracing, structured logging

## gRPC Service

| Method | Story |
|--------|-------|
| CreateInvoice | 002 |
| GetInvoice | 002 |
| UpdateInvoice | 002 |
| ListInvoices | 002 |
| AddLineItem | 003 |
| UpdateLineItem | 003 |
| RemoveLineItem | 003 |
| IssueInvoice | 004 |
| VoidInvoice | 004 |
| CreateTaxRate | 005 |
| ListTaxRates | 005 |
| RecordPayment | 006 |
| GetReceipt | 006 |
| ListReceipts | 006 |
| GenerateStatement | 007 |
| GetStatement | 007 |
| GenerateInvoicePdf | 008 |
| GenerateReceiptPdf | 008 |
| GenerateStatementPdf | 008 |

## Acceptance Criteria

- [ ] All gRPC methods implemented and tested
- [ ] Invoice lifecycle enforced (draft → issued → paid/void)
- [ ] Tax calculations correct for inclusive/exclusive rates
- [ ] Ledger entries created on issue and payment
- [ ] PDF generation produces valid documents
- [ ] Multi-tenant isolation verified
- [ ] Prometheus metrics exposed
- [ ] Health and readiness endpoints working
