---
name: service-observability
description:
  Set up production observability using PLG stack (Prometheus, Loki, Grafana).
  Focuses on self-hosted deployment on dedicated servers using Docker containers.
---

- Architecture
  - Logs: Promtail (agent) → Loki (storage/query) → Grafana (visualization)
  - Metrics: Application /metrics endpoint → Prometheus (scrape/storage) → Grafana (visualization)
  - Dashboards: Grafana unified interface for logs and metrics
  - Container count: 4 (Prometheus, Loki, Grafana, Promtail)

- Prometheus (Metrics)
  - Pull-based scraper: polls /metrics endpoints at configured intervals
  - Storage: local time-series database (TSDB), configurable retention
  - Config: /etc/prometheus/prometheus.yml
  - Scrape targets: static_configs or service discovery (docker, consul, kubernetes)
  - Data model: metrics with labels, e.g., http_requests_total{method="GET", status="200"}
  - Query language: PromQL for aggregation, filtering, rate calculations
  - Default port: 9090

- Loki (Logs)
  - Push-based ingestion: Promtail ships logs to Loki HTTP API
  - Storage: local filesystem or object storage (S3, MinIO)
  - Config: /etc/loki/loki.yaml
  - Index: labels only (not full-text), query by label filters then grep JSON
  - Query language: LogQL for label filtering and log stream processing
  - Data model: log streams identified by label set {service="api", env="prod"}
  - Default port: 3100

- Grafana (Visualization)
  - Unified dashboard for logs (Loki) and metrics (Prometheus)
  - Config: /etc/grafana/grafana.ini
  - Datasources: add Prometheus (http://prometheus:9090) and Loki (http://loki:3100)
  - Dashboards: JSON definitions, import community dashboards or build custom
  - Alerting: alert rules based on PromQL or LogQL queries, notification channels
  - Default port: 3000
  - Auth: admin user, SSO/LDAP for multi-user setups

- Promtail (Log Shipper)
  - Tails log files or scrapes systemd journal
  - Config: /etc/promtail/promtail.yaml
  - Pipeline stages: parse JSON, extract labels, filter, timestamp parsing
  - Push to Loki: batched HTTP requests
  - Label extraction: static labels (job, host) + dynamic from JSON
  - Default port: 9080 (metrics endpoint)

- Docker Deployment (docker-compose.yml)
  - prometheus: prom/prometheus:latest, volume /etc/prometheus, port 9090
  - loki: grafana/loki:latest, volume /etc/loki + /data/loki, port 3100
  - grafana: grafana/grafana:latest, volume /var/lib/grafana, port 3000
  - promtail: grafana/promtail:latest, volume /var/log + /etc/promtail, port 9080
  - Network: bridge or host network mode for simplicity
  - Restart policy: unless-stopped for production reliability

- Prometheus Configuration
  - Global: scrape_interval (15s), evaluation_interval (15s)
  - Scrape configs: job_name, static_configs with targets, metrics_path (/metrics)
  - Example target: application container exposing /metrics on port 8080
  - Relabeling: add/drop labels, modify target addresses
  - Storage: --storage.tsdb.path=/data, --storage.tsdb.retention.time=15d

- Loki Configuration
  - auth_enabled: false (single-tenant mode for self-hosted)
  - server: http_listen_port 3100
  - ingester: chunk_idle_period, chunk_retain_period, max_chunk_age
  - schema_config: index prefix, object storage config, period
  - storage_config: filesystem path or S3/MinIO bucket
  - limits_config: ingestion_rate_mb, ingestion_burst_size_mb per tenant

- Promtail Configuration
  - server: http_listen_port 9080
  - clients: Loki URL (http://loki:3100/loki/api/v1/push)
  - positions: file to track read offsets (/var/log/positions.yaml)
  - scrape_configs: job_name, static_configs paths, pipeline_stages
  - Pipeline: json stage (extract fields), labels stage (promote to index), timestamp, output
  - Example: parse JSON logs from /var/log/app/*.log, extract service/level labels

- Application Instrumentation
  - Logs: output JSON to stdout/stderr (Docker logs) or file (/var/log/app/)
  - Logs: follow logging-design skill (level, msg, ts, trace_id, static labels)
  - Metrics: expose /metrics endpoint in Prometheus format (use client library)
  - Metrics: counter (total events), gauge (current value), histogram (distribution), summary
  - Metrics libraries: prometheus_client (Python), prom-client (Node.js), prometheus crate (Rust)
  - Label cardinality: keep metric labels low (< 10 values), no user_id or request_id

- Verification
  - Prometheus: http://localhost:9090/targets (check scrape targets UP)
  - Prometheus: http://localhost:9090/graph (query metrics, e.g., up, rate(http_requests_total[5m]))
  - Loki: check Promtail logs (docker logs promtail) for successful pushes
  - Loki: use Grafana Explore with Loki datasource, query {service="api"}
  - Grafana: http://localhost:3000, login admin/admin, add datasources, create dashboard
  - Promtail: http://localhost:9080/metrics (check promtail_sent_entries_total)
  - Container health: docker ps, check all 4 containers running

- Querying
  - PromQL: rate(http_requests_total[5m]), histogram_quantile(0.99, http_duration_bucket)
  - LogQL: {service="api", level="error"} | json | line_format "{{.msg}}"
  - LogQL: {service="api"} | json | trace_id="abc123" (filter by JSON field)
  - LogQL aggregation: rate({service="api"} | json | level="error" [5m])
  - Grafana variables: $service, $env for dynamic dashboard filtering

- Best Practices
  - Prometheus retention: balance disk space vs historical data (7-30 days typical)
  - Loki log retention: configure compactor for object storage, or filesystem cleanup
  - Promtail batching: adjust batch_size and batch_wait for throughput vs latency
  - Use Grafana folders to organize dashboards by team or service
  - Export dashboards as JSON to version control
  - Set up alerts in Grafana: error rate thresholds, disk space, scrape failures
  - Use Loki label matchers efficiently: query by indexed labels first, then grep
  - Secure Grafana with reverse proxy (nginx) + TLS in production
  - Back up Grafana database (/var/lib/grafana/grafana.db) and Prometheus data
  - Monitor the monitoring stack: Prometheus self-scrape, Loki metrics, Grafana health