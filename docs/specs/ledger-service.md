# Ledger Service

**Generic double-entry accounting service for multi-tenant financial operations.**

## Problem

Every app handling money needs transaction tracking: apartment payments, school fees, marketplace commissions, wallet balances. Building custom ledgers per app leads to inconsistent accounting, balance errors, and audit failures.

## Solution

A reusable ledger microservice implementing double-entry accounting. Every transaction debits one account and credits another. Balances are always consistent. Immutable audit trail included.

## Core Principles

- **Double-entry:** Every transaction has equal debits and credits
- **Immutability:** Ledger entries never modified, only appended
- **Consistency:** Balances derived from entries, never stored separately
- **Multi-tenant:** Complete isolation between tenants
- **Idempotent:** Same request produces same result (safe retries)

## Data Model

### Accounts
- `account_id`: UUID
- `tenant_id`: UUID
- `account_type`: asset, liability, equity, revenue, expense
- `account_code`: tenant-defined identifier (e.g., "CASH", "TUITION_RECEIVABLE")
- `currency`: ISO 4217 code
- `metadata`: JSONB

### Ledger Entries
- `entry_id`: UUID
- `tenant_id`: UUID
- `journal_id`: groups related entries
- `account_id`: target account
- `amount`: decimal (positive)
- `direction`: debit or credit
- `effective_date`: when transaction occurred
- `posted_utc`: when recorded
- `idempotency_key`: prevents duplicates
- `metadata`: JSONB (reference_type, reference_id, description)

**Constraint:** Sum of debits = sum of credits per journal_id

## gRPC Service: LedgerService

| Method | Description |
|--------|-------------|
| `CreateAccount` | Create new account |
| `GetAccount` | Get account with current balance |
| `ListAccounts` | List accounts with filters |
| `PostTransaction` | Record double-entry transaction |
| `GetTransaction` | Get transaction by journal_id |
| `ListTransactions` | List transactions with filters |
| `GetBalance` | Get account balance at point in time |
| `GetBalances` | Get multiple account balances |
| `GetStatement` | Get account statement (date range) |

## Transaction Types

| Type | Debit | Credit | Example |
|------|-------|--------|---------|
| Payment received | Cash/Bank | Receivable | Rent payment |
| Fee charged | Receivable | Revenue | Tuition fee |
| Refund issued | Revenue | Cash/Bank | Overpayment refund |
| Transfer | Account A | Account B | Wallet to wallet |
| Adjustment | Expense | Receivable | Write-off bad debt |

## Use Cases

- **Apartments:** Rent receivables, maintenance fees, security deposits
- **Schools:** Tuition fees, installments, scholarships, penalties
- **Marketplaces:** Seller balances, commissions, payouts
- **Wallets:** User balances, top-ups, withdrawals, P2P transfers

## Key Features

- **Balance queries:** Real-time or point-in-time balances
- **Statements:** Date-range transaction history per account
- **Idempotency:** Safe retries with idempotency_key
- **Metadata:** Flexible JSONB for domain-specific data
- **Multi-currency:** Per-account currency, no auto-conversion
- **Audit trail:** Complete history, no deletions

## Integration Pattern

```
Domain Service (school-service, apartment-service)
    │
    │ PostTransaction(idempotency_key, entries[])
    ▼
Ledger Service
    │
    │ Validates double-entry, posts atomically
    ▼
PostgreSQL (entries table, accounts table)
```

Domain services own business logic. Ledger service owns accounting integrity.

## Edge Cases

- **Duplicate request:** Same idempotency_key returns original result
- **Unbalanced transaction:** Rejected (debits ≠ credits)
- **Negative balance:** Allowed or blocked per account configuration
- **Currency mismatch:** Rejected (all entries in transaction must match)
- **Account closure:** Soft-close, balance must be zero
- **Backdated entry:** Allowed with effective_date, posted_utc always now

## Non-Goals

- Payment processing (use payment-service)
- Currency conversion (use external rates)
- Invoice generation (domain service responsibility)
- Reporting/analytics (use read replicas + BI tools)

## References

- [Modern Treasury: Designing Ledgers API](https://www.moderntreasury.com/journal/designing-ledgers-with-optimistic-locking)
- [Modern Treasury: How to Scale a Ledger](https://www.moderntreasury.com/journal/how-to-scale-a-ledger-part-i)
- [Fragment: Ledger API](https://fragment.dev/)
- [FinLego: Real-Time Ledger System](https://finlego.com/tpost/c2pjjza3k1-designing-a-real-time-ledger-system-with)
