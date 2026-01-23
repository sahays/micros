# Epic: Metering for Invoicing Service

Status: planning
Created: 2026-01-23

## Overview

Add usage metering capabilities to invoicing-service to support consumption-based billing. Enables recording usage events, aggregating usage per billing period, and generating invoices with usage-based line items.

## Core Principles

- Idempotent ingestion: Usage events can be safely retried
- Flexible metering: Support per-unit, tiered, and volume pricing models
- Real-time aggregation: Usage summaries available immediately
- Multi-tenant: Complete isolation via tenant_id
- Audit trail: Full history of usage events

## Use Cases

1. **API Usage Billing**: Charge per API call, storage GB, compute hours
2. **Overage Billing**: Include base units in plan, charge for excess
3. **Tiered Pricing**: Different rates at different usage levels
4. **Volume Discounts**: Lower unit price at higher volumes

## Stories

### Phase 1: Foundation

- [ ] [010-usage-events](../stories/010-usage-events.md) - RecordUsage, GetUsageEvent, usage event storage
- [ ] [011-meters](../stories/011-meters.md) - CreateMeter, GetMeter, ListMeters, meter configuration

### Future

- [ ] [012-usage-aggregation](../stories/012-usage-aggregation.md) - GetUsageSummary, aggregation by meter/period
- [ ] [013-pricing-models](../stories/013-pricing-models.md) - Per-unit, tiered, volume pricing calculations
- [ ] [014-usage-invoicing](../stories/014-usage-invoicing.md) - CreateUsageInvoice, convert usage to line items

## Data Model

### Meter

Defines what is being measured and how to price it.

```
meters
├── meter_id (UUID, PK)
├── tenant_id (UUID, FK)
├── name (VARCHAR) - e.g., "api_calls", "storage_gb"
├── display_name (VARCHAR) - e.g., "API Calls", "Storage (GB)"
├── unit_name (VARCHAR) - e.g., "call", "GB"
├── aggregation_type (ENUM) - sum, max, last, unique_count
├── pricing_model (ENUM) - per_unit, tiered, volume
├── pricing_tiers (JSONB) - tier definitions
├── created_utc (TIMESTAMP)
└── updated_utc (TIMESTAMP)
```

### Usage Event

Individual usage record.

```
usage_events
├── event_id (UUID, PK)
├── tenant_id (UUID, FK)
├── meter_id (UUID, FK)
├── customer_id (UUID)
├── idempotency_key (VARCHAR, UNIQUE per tenant)
├── quantity (DECIMAL)
├── timestamp_utc (TIMESTAMP) - when usage occurred
├── properties (JSONB) - additional context
├── created_utc (TIMESTAMP)
```

### Usage Summary (Materialized/Computed)

Aggregated usage per customer/meter/period.

```
usage_summaries
├── summary_id (UUID, PK)
├── tenant_id (UUID, FK)
├── customer_id (UUID)
├── meter_id (UUID, FK)
├── period_start (DATE)
├── period_end (DATE)
├── total_quantity (DECIMAL)
├── event_count (INTEGER)
├── computed_amount (DECIMAL) - based on pricing model
├── last_updated_utc (TIMESTAMP)
```

## gRPC Methods

| Method | Story | Description |
|--------|-------|-------------|
| CreateMeter | 011 | Define a new meter |
| GetMeter | 011 | Retrieve meter by ID |
| ListMeters | 011 | List meters for tenant |
| UpdateMeter | 011 | Update meter configuration |
| RecordUsage | 010 | Record a usage event |
| RecordUsageBatch | 010 | Record multiple usage events |
| GetUsageEvent | 010 | Retrieve usage event by ID |
| ListUsageEvents | 010 | List raw usage events with filters |

### Future Methods

| Method | Story | Description |
|--------|-------|-------------|
| GetUsageSummary | 012 | Get aggregated usage for period |
| CalculateUsageCharges | 013 | Preview charges for usage |
| CreateUsageInvoice | 014 | Generate invoice from usage |

## Pricing Models

### Per-Unit
Simple multiplication: `quantity × unit_price`

```json
{
  "model": "per_unit",
  "unit_price": "0.001"
}
```

### Tiered
Different rates at different levels (each tier priced separately):

```json
{
  "model": "tiered",
  "tiers": [
    { "up_to": 1000, "unit_price": "0.01" },
    { "up_to": 10000, "unit_price": "0.008" },
    { "up_to": null, "unit_price": "0.005" }
  ]
}
```

Example: 15,000 units = (1000 × $0.01) + (9000 × $0.008) + (5000 × $0.005) = $107

### Volume
All units at the tier rate reached:

```json
{
  "model": "volume",
  "tiers": [
    { "up_to": 1000, "unit_price": "0.01" },
    { "up_to": 10000, "unit_price": "0.008" },
    { "up_to": null, "unit_price": "0.005" }
  ]
}
```

Example: 15,000 units = 15,000 × $0.005 = $75

## Aggregation Types

- **sum**: Total quantity (API calls, bytes transferred)
- **max**: Maximum value in period (concurrent users, peak storage)
- **last**: Last recorded value (current storage level)
- **unique_count**: Count of unique property values (active users)

## Integration with Invoicing

Usage-based invoicing flow:

1. **Record Usage**: Services call `RecordUsage` throughout billing period
2. **Period End**: At billing cycle end, call `CreateUsageInvoice`
3. **Aggregate**: Service aggregates usage per meter for the period
4. **Calculate**: Apply pricing model to determine charges
5. **Create Invoice**: Generate invoice with line items per meter
6. **Ledger**: Post AR and revenue entries (existing flow)

## Acceptance Criteria

- [ ] Meters can be created and configured per tenant
- [ ] Usage events recorded with idempotency
- [ ] Usage events can be listed with filters
- [ ] Multi-tenant isolation verified
- [ ] Prometheus metrics for usage ingestion rate
- [ ] OpenTelemetry tracing on all operations

## Future Considerations

- Usage aggregation by meter/customer/period
- Pricing model calculations (per-unit, tiered, volume)
- Usage-to-invoice conversion
- Real-time usage alerts and thresholds
- Prepaid usage packages (credits)
- Usage-based discounts and commitments
- Retroactive usage adjustments
- Usage export for customer self-service
