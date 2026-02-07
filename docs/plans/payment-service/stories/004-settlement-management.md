# Story: Settlement Management

- [ ] **Status: Planning**
- **Epic:** [001-saas-payment-platform](../epics/001-saas-payment-platform.md)

## Summary

Implement HoldTransferSettlement, ReleaseTransferSettlement, RequestOnDemandSettlement, GetSettlement, and ListSettlements gRPC methods for managing linked account settlement lifecycle.

## Tasks

- [ ] Define proto messages: Settlement, SettlementStatus enum, SettlementType enum
- [ ] Define proto messages: HoldTransferSettlementRequest/Response
- [ ] Define proto messages: ReleaseTransferSettlementRequest/Response
- [ ] Define proto messages: RequestOnDemandSettlementRequest/Response
- [ ] Define proto messages: GetSettlementRequest/Response
- [ ] Define proto messages: ListSettlementsRequest/Response
- [ ] Add Settlement MongoDB collection and indexes
- [ ] Implement Razorpay settlement API client
- [ ] Implement HoldTransferSettlement handler
- [ ] Implement ReleaseTransferSettlement handler
- [ ] Implement RequestOnDemandSettlement handler
- [ ] Implement GetSettlement handler
- [ ] Implement ListSettlements handler with filters and pagination
- [ ] Add capability checks to all methods
- [ ] Add metering for settlement operations

## gRPC Methods

### HoldTransferSettlement
**Input:** tenant_id, transfer_id, on_hold_until (optional — if omitted, indefinite hold)
**Output:** transfer with updated hold status

**Validation:**
- Transfer exists and is in PROCESSED status
- Transfer is not already on hold

**Business Logic:**
- Updates transfer via Razorpay Route API to set on_hold = true
- If on_hold_until provided, sets time-bound hold (auto-release)
- If on_hold_until omitted, sets indefinite hold (manual release required)
- Updates local transfer record

**Capability:** `payment.transfer:hold`

### ReleaseTransferSettlement
**Input:** tenant_id, transfer_id
**Output:** transfer with updated hold status

**Validation:**
- Transfer exists and is currently on hold

**Business Logic:**
- Updates transfer via Razorpay Route API to set on_hold = false
- Settlement proceeds according to linked account's schedule
- Updates local transfer record

**Capability:** `payment.transfer:hold`

### RequestOnDemandSettlement
**Input:** tenant_id, linked_account_id, amount (optional — if omitted, settle full balance), settle_full_balance
**Output:** settlement

**Validation:**
- Linked account exists and is ACTIVATED
- Amount does not exceed available balance (if specified)

**Business Logic:**
- Requests on-demand or instant settlement via Razorpay API
- Creates settlement record with type ON_DEMAND or INSTANT
- Settlement processed asynchronously (status updated via webhook)

**Capability:** `payment.settlement:create`

### GetSettlement
**Input:** tenant_id, settlement_id
**Output:** settlement with full details including UTR

**Capability:** `payment.settlement:read`

### ListSettlements
**Input:** tenant_id, linked_account_id (optional), status (optional), type (optional), page_size, page_token
**Output:** settlements[], next_page_token

**Capability:** `payment.settlement:read`

## Settlement Types

| Type | Description |
|------|-------------|
| `NORMAL` | Automatic per schedule (T+2 default) |
| `INSTANT` | Immediate settlement |
| `ON_DEMAND` | Requested, next settlement cycle |

## Settlement Status

| Status | Description |
|--------|-------------|
| `CREATED` | Settlement initiated |
| `PROCESSED` | Funds transferred to bank |
| `FAILED` | Settlement failed |

## Metering

Record on each operation:
```rust
record_settlement(&tenant_id, &settlement_type, &status);
record_settlement_amount(&tenant_id, &currency, amount);
```

## Acceptance Criteria

- [ ] HoldTransferSettlement places time-bound hold on transfer
- [ ] HoldTransferSettlement places indefinite hold when on_hold_until omitted
- [ ] HoldTransferSettlement returns FAILED_PRECONDITION for non-PROCESSED transfer
- [ ] HoldTransferSettlement returns FAILED_PRECONDITION for already-held transfer
- [ ] ReleaseTransferSettlement releases hold on transfer
- [ ] ReleaseTransferSettlement returns FAILED_PRECONDITION for non-held transfer
- [ ] RequestOnDemandSettlement requests settlement for linked account
- [ ] RequestOnDemandSettlement validates linked account is ACTIVATED
- [ ] RequestOnDemandSettlement supports amount or full balance
- [ ] GetSettlement returns settlement with UTR
- [ ] GetSettlement returns NOT_FOUND for missing settlement
- [ ] ListSettlements filters by linked account, status, type
- [ ] ListSettlements pagination works correctly
- [ ] All methods enforce tenant isolation
- [ ] All methods check capabilities

## Integration Tests

- [ ] Hold transfer settlement sets on_hold to true
- [ ] Hold transfer settlement with on_hold_until sets time-bound hold
- [ ] Hold transfer settlement without on_hold_until sets indefinite hold
- [ ] Hold transfer settlement on non-processed transfer returns FAILED_PRECONDITION
- [ ] Hold transfer settlement on already-held transfer returns FAILED_PRECONDITION
- [ ] Release transfer settlement clears hold
- [ ] Release transfer settlement on non-held transfer returns FAILED_PRECONDITION
- [ ] Request on-demand settlement creates settlement record
- [ ] Request on-demand settlement with non-activated account returns FAILED_PRECONDITION
- [ ] Request on-demand settlement with amount creates partial settlement
- [ ] Get settlement returns complete settlement
- [ ] Get settlement returns NOT_FOUND for missing settlement
- [ ] List settlements filters by linked account
- [ ] List settlements filters by status and type
- [ ] List settlements pagination works
- [ ] Operations without capability return PERMISSION_DENIED

## User Journey Integration Tests

- [ ] `journey_settlement_hold_and_release` — Verifies [journey 004](../../../user-journeys/payment-service/004-settlement-payout.md) steps 6-7: platform holds transfer settlement, then releases it
- [ ] `journey_on_demand_instant_settlement` — Verifies [journey 004](../../../user-journeys/payment-service/004-settlement-payout.md) step 5: tenant requests on-demand settlement, receives funds
