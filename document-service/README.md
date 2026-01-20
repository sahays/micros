# Document Service

Document storage and processing service with streaming upload/download support.

## Architecture

**gRPC-only** internal service. External clients access via BFF.

- **gRPC**: Port 50052 (all business logic)
- **HTTP**: Port 8080 (health checks only)

## gRPC Service

### DocumentService

```protobuf
// Streaming upload
rpc UploadDocument(stream UploadDocumentRequest) returns (UploadDocumentResponse)

// Streaming download
rpc DownloadDocument(DownloadDocumentRequest) returns (stream DownloadDocumentResponse)

// Metadata operations
rpc GetDocument(GetDocumentRequest) returns (GetDocumentResponse)
rpc ListDocuments(ListDocumentsRequest) returns (ListDocumentsResponse)
rpc DeleteDocument(DeleteDocumentRequest) returns (DeleteDocumentResponse)

// Processing
rpc ProcessDocument(ProcessDocumentRequest) returns (ProcessDocumentResponse)
rpc GetProcessingStatus(GetProcessingStatusRequest) returns (GetProcessingStatusResponse)

// Signed URLs
rpc GenerateSignedUrl(GenerateSignedUrlRequest) returns (GenerateSignedUrlResponse)

// Video streaming
rpc DownloadVideoChunk(DownloadVideoChunkRequest) returns (stream DownloadVideoChunkResponse)
```

## Tenant Context

All RPCs require gRPC metadata headers:
- `x-app-id`: Application ID
- `x-org-id`: Organization ID
- `x-user-id`: User ID

## Usage (grpcurl)

```bash
# List services
grpcurl -plaintext localhost:50052 list

# Get document metadata
grpcurl -plaintext \
  -H "x-app-id: app-123" \
  -H "x-org-id: org-456" \
  -H "x-user-id: user-789" \
  -d '{"document_id": "doc-abc"}' \
  localhost:50052 micros.document.v1.DocumentService/GetDocument

# List documents
grpcurl -plaintext \
  -H "x-app-id: app-123" \
  -H "x-org-id: org-456" \
  -H "x-user-id: user-789" \
  -d '{"page": 1, "page_size": 10}' \
  localhost:50052 micros.document.v1.DocumentService/ListDocuments

# Generate signed URL
grpcurl -plaintext \
  -H "x-app-id: app-123" \
  -H "x-org-id: org-456" \
  -H "x-user-id: user-789" \
  -d '{"document_id": "doc-abc", "expires_in_seconds": 3600}' \
  localhost:50052 micros.document.v1.DocumentService/GenerateSignedUrl
```

## Configuration

| Variable | Description |
|----------|-------------|
| `MONGODB_URI` | MongoDB connection |
| `STORAGE_PATH` | Local storage path |
| `GRPC_PORT` | gRPC port (default: 50052) |
| `HTTP_PORT` | Health check port (default: 8080) |

## Health Checks

```bash
# HTTP
curl http://localhost:8080/health

# gRPC
grpcurl -plaintext localhost:50052 grpc.health.v1.Health/Check
```

## Proto Definitions

See `proto/micros/document/v1/document.proto`
