# Story: Payment Service Client

- [ ] **Status: Planning**
- **Epic:** [002-payment-service-integration](../epics/002-payment-service-integration.md)

## Summary

Add PaymentServiceClient to billing-service for gRPC communication with payment-service. Configure connection, retry logic, and trace context propagation.

## Tasks

- [ ] Add payment-service proto dependency to billing-service build.rs
- [ ] Create PaymentServiceClient wrapper in service-core or billing-service
- [ ] Configure gRPC channel with retry interceptor from service-core
- [ ] Add PAYMENT_SERVICE_ENDPOINT configuration variable
- [ ] Initialize PaymentServiceClient in billing-service startup
- [ ] Add trace context propagation to payment-service calls
- [ ] Add health check for payment-service connectivity in /ready endpoint
- [ ] Add connection failure handling (graceful degradation)

## Configuration

| Variable | Description | Default |
|----------|-------------|---------|
| `PAYMENT_SERVICE_ENDPOINT` | Payment-service gRPC endpoint | `http://payment-service:50054` |

## Client Methods

The PaymentServiceClient wraps the following payment-service gRPC methods used by billing-service:

| Method | Usage |
|--------|-------|
| `CreateRazorpayPlan` | Create Razorpay plan matching billing plan |
| `CreateRazorpaySubscription` | Create subscription for customer enrollment |
| `GetRazorpaySubscription` | Query subscription status |
| `PauseRazorpaySubscription` | Pause subscription on billing pause |
| `ResumeRazorpaySubscription` | Resume subscription on billing resume |
| `CancelRazorpaySubscription` | Cancel subscription on billing cancellation |

## Connection Handling

- Retry on transient failures (UNAVAILABLE, DEADLINE_EXCEEDED) via service-core retry interceptor
- Log connection failures at error level
- /ready endpoint reports payment-service connectivity status
- Billing operations that require payment-service fail with clear error if unavailable

## Acceptance Criteria

- [ ] PaymentServiceClient connects to payment-service
- [ ] PaymentServiceClient wraps required gRPC methods
- [ ] Retry logic handles transient failures
- [ ] Trace context propagated to payment-service
- [ ] /ready endpoint checks payment-service connectivity
- [ ] Connection failure logged at error level
- [ ] Configuration via PAYMENT_SERVICE_ENDPOINT variable

## Integration Tests

- [ ] PaymentServiceClient connects to payment-service
- [ ] PaymentServiceClient call succeeds with valid request
- [ ] PaymentServiceClient returns error when payment-service unavailable
- [ ] /ready endpoint reflects payment-service status
