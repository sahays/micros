---
name: service-observability
description:
  Set up production observability using SigNoz with OpenTelemetry. Use when implementing centralized
  logging, tracing, and metrics infrastructure. Focuses on Docker configuration and OpenTelemetry integration.
---

# Service Observability with SigNoz

## OpenTelemetry Architecture

- Deploy OpenTelemetry Collector as sidecar or centralized service
- Applications instrument with OpenTelemetry SDK and export OTLP protocol
- OTel Collector receives traces, logs, and metrics on port 4317 (gRPC) and 4318 (HTTP)
- SigNoz backend stores telemetry data in ClickHouse (logs, traces) and query service
- SigNoz frontend UI connects to query service on port 8080
- Services use shared Docker network, remain decoupled from observability infrastructure

## Docker Compose Structure

- SigNoz ClickHouse: Data storage for logs and traces, expose 9000, 8123, named volume `signoz-clickhouse-data`
- SigNoz Query Service: Aggregates and queries data, environment `ClickHouseUrl=tcp://clickhouse:9000`, expose 8080
- SigNoz OTel Collector: Receives OTLP telemetry, expose 4317 (gRPC), 4318 (HTTP), mount otel-collector-config.yaml
- SigNoz Frontend: UI dashboard, environment `FRONTEND_API_ENDPOINT=http://query-service:8080`, expose 3301
- Application service: Environment `OTEL_EXPORTER_OTLP_ENDPOINT=http://otel-collector:4317`, `SERVICE_NAME`, `OTEL_RESOURCE_ATTRIBUTES=service.name=<name>`
- Health checks: ClickHouse `wget --spider -q localhost:8123/ping`, query service curl `:8080/api/v1/health`
- Depends_on with health conditions: Query service waits for ClickHouse, frontend and collector wait for query service
- Named volumes: ClickHouse data, OTel Collector data
- Network: Bridge driver for all services

## OpenTelemetry Collector Configuration

- Receivers: `otlp` protocol on gRPC `:4317` and HTTP `:4318`
- Receivers: `hostmetrics` for system metrics (cpu, memory, disk, network)
- Processors: `batch` with timeout 1s and batch size 1024 for efficient export
- Processors: `memory_limiter` with check interval 1s, limit 512MB, spike limit 128MB
- Processors: `resource` to add deployment environment attributes
- Processors: `attributes` to filter or add custom attributes
- Exporters: `otlp` to SigNoz backend `query-service:4317` with compression gzip
- Exporters: `logging` for debugging (disable in production)
- Service pipelines: traces, metrics, logs all flow through batch processor to otlp exporter
- Extensions: `health_check` on :13133, `pprof` on :1777, `zpages` on :55679

## Application Instrumentation Requirements

- Integrate OpenTelemetry SDK for your language (JS, Python, Java, Go, Rust, etc.)
- Configure OTLP exporter with endpoint `http://otel-collector:4317` (gRPC) or `:4318` (HTTP)
- Required resource attributes: `service.name`, `service.version`, `deployment.environment`
- Logs: Use OpenTelemetry Logs API with `timestamp`, `severity_text`, `severity_number`, `body`, `trace_id`, `span_id`
- Traces: Auto-instrument HTTP, database, RPC calls or manually create spans for business operations
- Metrics: Export runtime metrics (CPU, memory, GC) and custom business metrics
- Context propagation: Use W3C Trace Context headers for distributed tracing across services
- Log correlation: Ensure logs include `trace_id` and `span_id` from active span context

## SigNoz Data Retention Configuration

- ClickHouse TTL settings for automatic data cleanup
- Traces table: TTL after 15 days (configurable via `RETENTION_PERIOD` in query service)
- Logs table: TTL after 7 days for high volume, 30 days for errors/warnings
- Metrics table: Raw metrics 7 days, aggregated metrics 90 days
- Configure via ClickHouse SQL: `ALTER TABLE signoz_traces.signoz_index_v2 MODIFY TTL toDateTime(timestamp) + INTERVAL 15 DAY`
- Set retention policies in SigNoz UI: Settings → Retention → Configure per signal type
- Use sampling for high-volume traces: Configure in OTel Collector with `probabilistic_sampler` processor

## Verification

- SigNoz UI: Access at `http://localhost:3301`, verify services appear in Services tab
- Query service health: GET `http://localhost:8080/api/v1/health` returns healthy status
- OTel Collector metrics: Access zpages at `http://localhost:55679/debug/tracez` to see received spans
- View traces: SigNoz UI → Traces tab shows distributed traces with service topology
- View logs: SigNoz UI → Logs tab shows structured logs with trace correlation
- Check metrics: SigNoz UI → Metrics tab shows service metrics and custom metrics
- Verify correlation: Click trace in UI, see linked logs with matching `trace_id`

## Multi-Service Scaling

- Each service integrates OpenTelemetry SDK with unique `service.name` resource attribute
- Single centralized OTel Collector can handle multiple services (recommended for simplicity)
- Alternative: Deploy sidecar OTel Collector per service for isolation and network efficiency
- All services export to same OTel Collector endpoint (centralized) or local sidecar
- SigNoz automatically groups telemetry by `service.name` attribute
- Use service map in SigNoz UI to visualize cross-service dependencies
- Configure sampling per service using tail-based sampling in OTel Collector

## Production Configuration

- ClickHouse memory: Allocate 4-8GB for small deployments, 16-32GB for production scale
- Use persistent volumes for ClickHouse data and OTel Collector buffer
- Set application instrumentation to sample traces: 100% for low traffic, 10-20% for high traffic
- Configure OTel Collector `memory_limiter` processor to prevent OOM conditions
- Enable TLS for OTLP endpoints if network-exposed: Configure certificates in OTel Collector
- Use separate SigNoz deployments for production and non-production environments
- Configure alerting rules in SigNoz UI for error rates, latency spikes, service downtime
- Enable authentication in SigNoz: Set `SIGNOZ_ADMIN_PASSWORD` environment variable
- Resource limits: OTel Collector 512MB-2GB memory, ClickHouse 8-32GB memory, Query service 2-4GB memory

## Troubleshooting

- No data in SigNoz: Check OTel Collector logs for export errors, verify app OTLP endpoint configuration
- Traces missing: Ensure OpenTelemetry SDK initialized with tracer provider, check sampling configuration
- Logs not correlated: Verify `trace_id` and `span_id` included in log records, use OTel Logs Bridge API
- High cardinality errors: Reduce unique attribute combinations, use aggregation in OTel Collector processors
- OTel Collector crash: Check `memory_limiter` settings, verify ClickHouse connectivity, review collector logs
- Missing spans: Check for instrumentation gaps, verify context propagation across async boundaries
- ClickHouse disk full: Verify TTL policies active, manually drop old partitions, increase disk or reduce retention
