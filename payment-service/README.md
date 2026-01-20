# Payment Service

Payment processing service with Razorpay and UPI support.

## Architecture

**gRPC-only** internal service. External clients access via BFF.

- **gRPC**: Port 50054 (all business logic)
- **HTTP**: Port 8082 (health checks only)

## gRPC Service

### PaymentService

```protobuf
// Transaction management
rpc CreateTransaction(CreateTransactionRequest) returns (CreateTransactionResponse)
rpc GetTransaction(GetTransactionRequest) returns (GetTransactionResponse)
rpc UpdateTransactionStatus(UpdateTransactionStatusRequest) returns (UpdateTransactionStatusResponse)
rpc ListTransactions(ListTransactionsRequest) returns (ListTransactionsResponse)

// Razorpay
rpc CreateRazorpayOrder(CreateRazorpayOrderRequest) returns (CreateRazorpayOrderResponse)
rpc VerifyRazorpayPayment(VerifyRazorpayPaymentRequest) returns (VerifyRazorpayPaymentResponse)

// UPI
rpc GenerateUpiQr(GenerateUpiQrRequest) returns (GenerateUpiQrResponse)

// Webhooks (proxied from BFF)
rpc HandleRazorpayWebhook(HandleRazorpayWebhookRequest) returns (HandleRazorpayWebhookResponse)
```

## Tenant Context

All RPCs require gRPC metadata headers:
- `x-app-id`: Application ID
- `x-org-id`: Organization ID
- `x-user-id`: User ID (optional)

## Usage (grpcurl)

```bash
# List services
grpcurl -plaintext localhost:50054 list

# Create Razorpay order
grpcurl -plaintext \
  -H "x-app-id: app-123" \
  -H "x-org-id: org-456" \
  -H "x-user-id: user-789" \
  -d '{
    "amount": 10000,
    "currency": "INR",
    "receipt": "order-123"
  }' localhost:50054 micros.payment.v1.PaymentService/CreateRazorpayOrder

# Verify payment
grpcurl -plaintext \
  -H "x-app-id: app-123" \
  -H "x-org-id: org-456" \
  -H "x-user-id: user-789" \
  -d '{
    "transaction_id": "txn-abc",
    "razorpay_order_id": "order_xyz",
    "razorpay_payment_id": "pay_xyz",
    "razorpay_signature": "sig..."
  }' localhost:50054 micros.payment.v1.PaymentService/VerifyRazorpayPayment

# Generate UPI QR
grpcurl -plaintext \
  -H "x-app-id: app-123" \
  -H "x-org-id: org-456" \
  -d '{
    "amount": 100.50,
    "description": "Payment for order"
  }' localhost:50054 micros.payment.v1.PaymentService/GenerateUpiQr

# List transactions
grpcurl -plaintext \
  -H "x-app-id: app-123" \
  -H "x-org-id: org-456" \
  -d '{
    "status": "TRANSACTION_STATUS_COMPLETED",
    "limit": 20
  }' localhost:50054 micros.payment.v1.PaymentService/ListTransactions
```

## Configuration

| Variable | Description |
|----------|-------------|
| `MONGODB_URI` | MongoDB connection |
| `RAZORPAY_KEY_ID` | Razorpay API key |
| `RAZORPAY_KEY_SECRET` | Razorpay secret |
| `RAZORPAY_WEBHOOK_SECRET` | Webhook verification |
| `UPI_VPA` | UPI Virtual Payment Address |
| `GRPC_PORT` | gRPC port (default: 50054) |
| `HTTP_PORT` | Health check port (default: 8082) |

## Health Checks

```bash
# HTTP
curl http://localhost:8082/health

# gRPC
grpcurl -plaintext localhost:50054 grpc.health.v1.Health/Check
```

## Proto Definitions

See `proto/micros/payment/v1/`:
- `payment.proto` - Payment service
- `transaction.proto` - Transaction messages
