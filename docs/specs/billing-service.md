# Billing Service

## Purpose

Manage recurring billing schedules, execute billing runs, and process charges. Automates subscription and usage-based billing cycles.

## Domain

### Subscription
A customer's agreement to pay on a recurring basis.

- Links customer to one or more billing plans
- Has start date, optional end date, and billing anchor date
- Status: active, paused, cancelled, expired
- Supports trial periods with separate trial end date
- Can be upgraded/downgraded mid-cycle with proration

### Billing Plan
A template defining what to charge and how often.

- Named plan with description (e.g., "Pro Monthly", "Enterprise Annual")
- Billing interval: daily, weekly, monthly, quarterly, annually
- Base price and currency
- Optional usage-based components with unit pricing
- Tax rate references

### Billing Cycle
A single billing period for a subscription.

- Defined by period start and end dates
- Status: pending, invoiced, paid, failed
- Contains calculated charges for the period
- Links to generated invoice

### Charge
An individual billable item within a cycle.

- Type: recurring (from plan), usage (metered), one-time (ad-hoc)
- Description, quantity, unit price, amount
- Can be prorated for partial periods

### Usage Record
Metered usage reported for billing.

- Links to subscription and usage component
- Quantity consumed in a time window
- Aggregated per billing cycle for invoicing

### Billing Run
Batch execution of billing for due subscriptions.

- Scheduled or manually triggered
- Processes all subscriptions due for billing
- Generates invoices via invoicing-service
- Tracks success/failure per subscription

## Key Operations

**Subscription Management**
- Create subscription linking customer to plan
- Activate, pause, resume, cancel subscription
- Change plan (upgrade/downgrade with proration)
- Set trial period

**Plan Management**
- Create/update billing plans
- Define recurring and usage-based pricing
- Archive plans (no new subscriptions, existing continue)

**Usage Tracking**
- Record usage events for metered billing
- Query usage totals for a subscription/period
- Support idempotent usage reporting

**Billing Execution**
- Run billing for all due subscriptions (batch)
- Run billing for single subscription (on-demand)
- Calculate charges including proration
- Generate invoices through invoicing-service
- Handle billing failures with retry logic

**Proration**
- Calculate prorated charges for mid-cycle changes
- Support proration modes: immediate, next_cycle, none

## Ledger Integration

Billing service does not post directly to ledger. It creates invoices through invoicing-service, which handles ledger entries.

**Flow:**
1. Billing run identifies due subscriptions
2. Charges calculated for each subscription
3. Invoice created via invoicing-service
4. Invoicing-service posts to ledger

## Business Rules

1. Billing anchor date determines when cycles start (e.g., 1st of month, signup date)
2. Usage is aggregated at cycle end before invoicing
3. Failed billing attempts are retried with exponential backoff
4. Cancelled subscriptions bill through current period end
5. Paused subscriptions skip billing runs until resumed
6. Plan changes take effect immediately or at next cycle based on configuration
7. Proration calculated as (days_used / days_in_period) * price
8. Trials convert to paid automatically unless cancelled
9. Usage records are immutable once invoiced

## Dependencies

- **invoicing-service**: Create invoices for billing cycles
- **ledger-service**: Indirect, via invoicing-service
- **notification-service**: Send billing reminders, payment failures (optional)
