# Epic: Metering for All Services

Status: complete
Created: 2026-01-23
Updated: 2026-01-23

## Overview

API call metering via Prometheus metrics with tenant_id labels. Implemented in `service-core` so all services get metering automatically.

## Implementation

Metering is handled by existing observability infrastructure with tenant_id added as a label:

### HTTP Services

**File:** `service-core/src/middleware/metrics.rs`

- Extracts `x-tenant-id` header from requests
- Records `http_requests_total{method, path, status, tenant_id}`
- Records `http_request_duration_seconds{method, path, status, tenant_id}`

### gRPC Services

**File:** `service-core/src/grpc/interceptors.rs`

- Extracts `x-tenant-id` from gRPC metadata
- Records `grpc_metering_total{tenant_id}`
- Provides `metrics_interceptor` for services to use
- Note: Service/method not available in interceptors; use service-specific metrics for method-level detail

## Usage

### Sending Requests with Tenant ID

**HTTP:**
```bash
curl -H "x-tenant-id: tenant-123" http://service/endpoint
```

**gRPC (from code):**
```rust
use service_core::grpc::inject_tenant_id;

let mut request = Request::new(my_message);
inject_tenant_id(&mut request, "tenant-123");
client.some_rpc(request).await?;
```

### Querying Metrics (Prometheus)

```promql
# Total gRPC calls per tenant (metering)
sum by (tenant_id) (grpc_metering_total)

# HTTP requests per tenant
sum by (tenant_id) (http_requests_total)

# HTTP requests per tenant per path
sum by (tenant_id, path) (http_requests_total)

# Requests without tenant_id (unknown)
grpc_metering_total{tenant_id="unknown"}
http_requests_total{tenant_id="unknown"}
```

### Billing Period Queries

```promql
# gRPC calls in last 30 days per tenant
sum by (tenant_id) (increase(grpc_metering_total[30d]))

# HTTP calls in last 30 days per tenant
sum by (tenant_id) (increase(http_requests_total[30d]))

# Daily API calls per tenant
sum by (tenant_id) (increase(grpc_metering_total[1d]))
```

## Services Using Metering

All services using `service-core` automatically get metering:
- invoicing-service (gRPC)
- ledger-service (gRPC)
- auth-service (gRPC + HTTP)
- document-service (gRPC)
- genai-service (gRPC)
- notification-service (gRPC)

## Enabling the gRPC Interceptor

Services must add the metrics interceptor to their gRPC server:

```rust
use service_core::grpc::metrics_interceptor;

Server::builder()
    .layer(tonic::service::interceptor(metrics_interceptor))
    .add_service(my_service)
    .serve(addr)
    .await?;
```

## Future Considerations

- Usage aggregation for billing periods
- Pricing model calculations
- Usage-to-invoice conversion
- Real-time usage alerts and thresholds
- Prepaid usage packages (credits)
