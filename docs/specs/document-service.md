# Document Service

**Multi-tenant document storage and processing service with streaming support.**

## Problem

Every app needs file handling: uploads, downloads, thumbnails, video transcoding, and PDF text extraction. Building custom document handling per app leads to inconsistent security, storage sprawl, and repeated processing logic.

## Solution

A reusable document microservice providing:
- Multi-tenant document storage with tenant isolation
- Streaming upload/download via gRPC
- Async document processing (images, videos, PDFs)
- Signed URLs for temporary public access
- Chunked video delivery for large files

## Core Principles

- **Multi-tenant:** Complete isolation between tenants via app_id/org_id
- **Streaming:** Client and server streaming for efficient large file handling
- **Async processing:** Background workers for compute-intensive operations
- **BFF trust model:** Trusts upstream services for authorization (see Architecture Decisions)
- **Storage abstraction:** Pluggable storage backends (local filesystem, S3, etc.)

## Data Model

### Documents
- `id`: UUID
- `app_id`: tenant application ID
- `org_id`: organization within tenant
- `owner_id`: user who uploaded
- `original_name`: original filename
- `mime_type`: MIME type
- `size`: file size in bytes
- `storage_key`: path in storage backend
- `status`: uploading, processing, ready, failed
- `error_message`: optional error details
- `processing_metadata`: optional processing results
- `created_at`: timestamp
- `updated_at`: timestamp

### Processing Metadata
- `extracted_text`: text from PDFs
- `page_count`: PDF page count
- `duration_seconds`: video duration
- `optimized_size`: processed file size
- `thumbnail_path`: thumbnail storage path
- `resolution`: video resolution
- `chunk_count`: number of video chunks
- `total_size`: total chunked size
- `chunks`: video chunk info array

## gRPC Service: DocumentService

| Method | Type | Description |
|--------|------|-------------|
| `UploadDocument` | Client streaming | Upload document in chunks |
| `DownloadDocument` | Server streaming | Download document in chunks |
| `GetDocument` | Unary | Get document metadata |
| `ListDocuments` | Unary | List documents with pagination |
| `DeleteDocument` | Unary | Delete document and files |
| `ProcessDocument` | Unary | Trigger async processing |
| `GetProcessingStatus` | Unary | Check processing progress |
| `GenerateSignedUrl` | Unary | Create temporary access URL |
| `DownloadVideoChunk` | Server streaming | Download specific video chunk |

## Document Lifecycle

```
Upload → Ready → (Optional) Process → Processing → Ready/Failed
                                         ↑
                                     Retry (3x)
```

1. **Upload:** Client streams file data, server saves to storage
2. **Ready:** Document available for download
3. **Process:** Optional processing triggered (images, videos, PDFs)
4. **Processing:** Background worker processes document
5. **Ready/Failed:** Processing completes or fails after retries

## Processing Capabilities

### Images
- Format conversion (JPEG, PNG, WebP)
- Quality optimization (1-100)
- Thumbnail generation

### Videos
- Format conversion (MP4, HLS)
- Resolution adjustment
- Chunked delivery for streaming

### PDFs
- Text extraction
- Image extraction
- Page count detection

## Authentication Model

### Tenant Context Headers
All requests require tenant context via gRPC metadata:
- `x-app-id`: Application/tenant identifier
- `x-org-id`: Organization within tenant
- `x-user-id`: User identifier

### Trust Model
Document-service uses a **BFF trust model**:
- Trusts upstream services (secure-frontend) to validate authorization
- Does NOT validate JWT tokens directly
- Does NOT enforce document ownership checks
- Multi-tenant isolation via database queries (filter by app_id/org_id)

### Signed URLs
For temporary public access without authentication:
- HMAC-SHA256 signature
- Configurable expiration (1 min to 24 hours)
- Bypass tenant headers when valid signature provided

## Capabilities

Capabilities control access to document-service operations. Each capability maps to specific gRPC methods.

**Format:** `{domain}.{resource}:{action}`

| Capability | gRPC Methods | Description |
|------------|--------------|-------------|
| `document:upload` | UploadDocument | Upload new documents |
| `document:download` | DownloadDocument, DownloadVideoChunk | Download document content |
| `document:read` | GetDocument, ListDocuments, GetProcessingStatus | View document metadata |
| `document:delete` | DeleteDocument | Delete documents |
| `document:process` | ProcessDocument | Trigger document processing |
| `document.signed_url:generate` | GenerateSignedUrl | Create temporary access URLs |

### Capability Enforcement Modes

Document-service supports two authorization models:

**1. BFF Trust Model (Default)**
- When `AUTH_SERVICE_ENDPOINT` is not configured
- Trusts upstream services (BFF) to validate authorization
- Capability enforcement handled by secure-frontend
- Document-service only enforces tenant isolation

**2. Direct Capability Enforcement (Optional)**
- When `AUTH_SERVICE_ENDPOINT` is configured
- Document-service validates JWT tokens via auth-service
- Checks capabilities for each gRPC method
- Provides defense-in-depth for direct access scenarios

## Integration Pattern

```
Client App
    │
    │ Authorization: Bearer <access_token>
    ▼
BFF (secure-frontend)
    │
    │ 1. Validate JWT token
    │ 2. Check document:* capabilities
    │ 3. Verify document ownership (if required)
    │ 4. Add tenant context headers (x-app-id, x-org-id, x-user-id)
    ▼
Document Service
    │
    │ Trust caller, process request
    ▼
Storage (Local/S3) + MongoDB
```

## Use Cases

- **User uploads:** Profile photos, documents, attachments
- **Media processing:** Image optimization, video transcoding
- **Document management:** PDF storage with text search
- **Temporary sharing:** Signed URLs for limited-time access

## Key Features

- **Streaming:** 64KB chunks for efficient transfer
- **Size limits:** 20MB default (configurable)
- **Worker pool:** Configurable concurrent processing (default 4 workers)
- **Retry logic:** 3 attempts for failed processing
- **Health endpoints:** HTTP /health, /ready, /metrics for orchestration
- **Optional auth:** Direct capability enforcement via auth-service integration
- **Metering:** Per-tenant metrics for usage tracking and billing

## Edge Cases

- **Large files:** Chunked streaming, no memory bloat
- **Processing failure:** Retries with exponential backoff
- **Concurrent processing:** Rejected if already processing
- **Storage error:** Returns error, no partial uploads
- **Signed URL expired:** Returns 403 Forbidden
- **Missing tenant headers:** Returns Unauthenticated

## Non-Goals

- Direct user authentication (use auth-service)
- Document ownership enforcement (BFF responsibility)
- Full-text search (use search service)
- CDN integration (deploy behind CDN)

## Observability

### HTTP Endpoints

| Endpoint | Description |
|----------|-------------|
| `GET /health` | Liveness probe (checks MongoDB connection) |
| `GET /ready` | Readiness probe (checks MongoDB connection) |
| `GET /metrics` | Prometheus metrics in text format |

### Logging
- Structured JSON logging to stdout (PLG-compatible)
- Request/response logging with trace correlation
- Processing job status logging
- Fields: `trace_id`, `span_id`, `request_id`, `tenant_id`

### Metrics

**HTTP/gRPC Metrics (via interceptors):**
- `grpc_metering_total` - gRPC requests by tenant
- `http_requests_total` - HTTP requests by method, path, status

**Document Metrics:**
- `document_uploads_total` - Uploads by tenant_id, mime_type
- `document_upload_bytes` - Upload size histogram by tenant_id
- `document_downloads_total` - Downloads by tenant_id
- `document_download_bytes` - Download size histogram by tenant_id
- `document_deletes_total` - Deletes by tenant_id
- `document_processing_requests_total` - Processing requests by tenant_id, mime_type

**Worker Metrics:**
- `document_processing_total` - Processing jobs by mime_type
- `document_processing_success` - Successful processing by mime_type
- `document_processing_failed` - Failed processing by mime_type
- `document_processing_duration` - Processing duration histogram by mime_type

### Tracing
- OpenTelemetry spans for all operations
- Trace ID propagation through workers
- Storage and database span attribution
- Exports to Tempo via OTLP/gRPC

## Configuration

| Variable | Description | Default |
|----------|-------------|---------|
| `MONGODB_URI` | MongoDB connection string | `mongodb://localhost:27017` |
| `MONGODB_DATABASE` | Database name | `document_db` |
| `STORAGE_LOCAL_PATH` | Local storage directory | `./storage` |
| `SIGNING_SECRET` | HMAC secret for signed URLs | (required in prod) |
| `REQUIRE_SIGNATURES` | Require signed requests | `false` |
| `WORKER_ENABLED` | Enable worker pool | `true` |
| `WORKER_COUNT` | Number of processing workers | `4` |
| `QUEUE_SIZE` | Worker job queue size | `100` |
| `COMMAND_TIMEOUT_SECONDS` | Processing command timeout | `300` |
| `TEMP_DIR` | Temp directory for processing | `/tmp/document-processing` |
| `AUTH_SERVICE_ENDPOINT` | Auth-service gRPC endpoint (enables capability enforcement) | (unset) |
| `OTLP_ENDPOINT` | OpenTelemetry collector | `http://tempo:4317` |
| `APP__PORT` | HTTP port (gRPC = port + 1) | `8080` |

## References

- Architecture Decisions: `document-service/docs/architecture-decisions.md`
- Proto Definition: `proto/micros/document/v1/document.proto`
- BFF Pattern: https://samnewman.io/patterns/architectural/bff/
