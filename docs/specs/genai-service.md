# GenAI Service

**Multi-tenant generative AI service with document context, streaming, and usage tracking.**

## Problem

Every application needs AI capabilities: text generation, structured data extraction, document analysis, and media processing. Building custom AI integrations per app leads to:
- Inconsistent provider abstractions
- No centralized usage tracking or cost attribution
- Repeated document processing logic
- No multi-tenant isolation

## Solution

A reusable AI microservice providing:
- Multi-tenant prompt processing with document context
- Streaming and non-streaming responses
- Multiple output formats (text, JSON, audio, video)
- Session management for conversational AI
- Per-tenant usage tracking and billing data
- Provider abstraction (Google Gemini, Veo, etc.)

## Core Principles

- **Multi-tenant:** Complete isolation via tenant_id/user_id metadata
- **Document-aware:** Integrates with document-service for contextual AI
- **Provider abstraction:** Pluggable AI providers with unified interface
- **Usage tracking:** Per-request token metering for cost attribution
- **BFF trust model:** Trusts upstream services for authorization

## Data Model

### Sessions
- `id`: UUID
- `tenant_id`: Application/tenant identifier
- `user_id`: User who created session
- `title`: Optional session title
- `system_prompt`: Optional system instruction
- `documents`: Attached document references
- `messages`: Conversation history
- `total_input_tokens`: Cumulative input token count
- `total_output_tokens`: Cumulative output token count
- `created_at`: Timestamp
- `updated_at`: Timestamp

### Usage Records
- `id`: UUID
- `tenant_id`: Application/tenant identifier
- `user_id`: User who made request
- `model`: AI model used
- `input_tokens`: Tokens consumed by input
- `output_tokens`: Tokens generated
- `request_id`: Correlation ID
- `tags`: Optional categorization
- `created_at`: Timestamp

## gRPC Service: GenAIService

| Method | Type | Description |
|--------|------|-------------|
| `Process` | Unary | Process prompt with optional documents |
| `ProcessStream` | Server streaming | Stream response tokens in real-time |
| `CreateSession` | Unary | Create conversation session |
| `GetSession` | Unary | Retrieve session with optional messages |
| `DeleteSession` | Unary | Delete session and data |
| `GetUsage` | Unary | Query usage statistics |
| `ListModels` | Unary | List available AI models |

## Output Formats

| Format | Description | Requirements |
|--------|-------------|--------------|
| `TEXT` | Free-form text generation | Default |
| `STRUCTURED_JSON` | JSON conforming to schema | `output_schema` required |
| `AUDIO` | Audio bytes (TTS) | Audio model |
| `VIDEO` | Video bytes | Video model |

## Document Integration

GenAI-service integrates with document-service for contextual AI:

1. Client provides `DocumentContext` with document_id and signed_url
2. If `text_content` not provided, service fetches from document-service
3. Document content injected into prompt context
4. Supports images via signed URLs (multimodal models)

```
Client Request
    │
    │ prompt + DocumentContext[]
    ▼
GenAI Service
    │
    │ Fetch document text (if not provided)
    ▼
Document Service ←── GetDocument RPC
    │
    ▼
AI Provider (Gemini/Veo)
    │
    ▼
Response with token usage
```

## Authentication Model

### Request Metadata
All requests include `RequestMetadata`:
- `tenant_id`: Required tenant identifier
- `user_id`: Required user identifier
- `tags`: Optional categorization map

### Trust Model
GenAI-service uses a **BFF trust model**:
- Trusts upstream services to validate authorization
- Does NOT validate JWT tokens directly
- Does NOT enforce session ownership checks
- Multi-tenant isolation via database queries

## Capabilities

Capabilities control access to genai-service operations.

**Format:** `{domain}.{resource}:{action}`

| Capability | gRPC Methods | Description |
|------------|--------------|-------------|
| `genai:process` | Process, ProcessStream | Execute AI prompts |
| `genai.session:create` | CreateSession | Create conversation sessions |
| `genai.session:read` | GetSession | View session details |
| `genai.session:delete` | DeleteSession | Delete sessions |
| `genai.usage:read` | GetUsage | Query usage statistics |
| `genai.models:read` | ListModels | List available models |

### Capability Enforcement Modes

**1. BFF Trust Model (Default)**
- When `AUTH_SERVICE_ENDPOINT` is not configured
- Trusts upstream services for authorization
- Capability enforcement handled by secure-frontend

**2. Direct Capability Enforcement (Optional)**
- When `AUTH_SERVICE_ENDPOINT` is configured
- Validates JWT tokens via auth-service
- Checks capabilities for each gRPC method

## Integration Pattern

```
Client App
    │
    │ Authorization: Bearer <access_token>
    ▼
BFF (secure-frontend)
    │
    │ 1. Validate JWT token
    │ 2. Check genai:* capabilities
    │ 3. Add RequestMetadata (tenant_id, user_id)
    ▼
GenAI Service
    │
    │ Trust caller, process request
    ▼
AI Provider (Gemini/Veo) + MongoDB
```

## Use Cases

- **Text generation:** Content creation, summarization, translation
- **Document analysis:** Extract data from PDFs, images
- **Structured extraction:** Parse documents to JSON schema
- **Conversational AI:** Multi-turn conversations with context
- **Usage analytics:** Track AI costs per tenant/user

## Key Features

- **Streaming:** Real-time token generation via gRPC streaming
- **Model selection:** Auto-select model based on output format
- **Session persistence:** MongoDB-backed conversation history
- **Token tracking:** Per-request usage recording
- **Health endpoints:** HTTP /health, /ready, /metrics
- **Provider abstraction:** Easy to add new AI providers

## Edge Cases

- **Empty prompt:** Returns InvalidArgument
- **Missing schema:** Returns InvalidArgument for STRUCTURED_JSON
- **Invalid schema:** Returns InvalidArgument
- **Rate limited:** Returns ResourceExhausted
- **Content filtered:** Returns InvalidArgument with safety message
- **Provider error:** Returns Internal with details
- **Network error:** Returns Unavailable

## Non-Goals

- Direct user authentication (use auth-service)
- Session ownership enforcement (BFF responsibility)
- File storage (use document-service)
- Billing/invoicing (use ledger-service)

## Observability

### HTTP Endpoints

| Endpoint | Description |
|----------|-------------|
| `GET /health` | Liveness probe (checks MongoDB) |
| `GET /ready` | Readiness probe (checks MongoDB) |
| `GET /metrics` | Prometheus metrics |

### Logging
- Structured JSON to stdout (PLG-compatible)
- Request/response logging with trace correlation
- Token usage logging per request
- Provider error classification

### Metrics

**gRPC Metrics:**
- `grpc_requests_total` - Requests by method, status
- `grpc_request_duration_seconds` - Duration histogram by method
- `grpc_requests_in_flight` - Current request count by method
- `grpc_metering_total` - Requests by tenant_id (via interceptor)

**AI Metrics (Per-Tenant Billing):**
- `genai_tokens_total{tenant_id, model, type}` - Tokens by tenant, model, type (input/output)
- `genai_requests_total{tenant_id, output_format, model, finish_reason}` - Requests by tenant
- `genai_provider_latency_seconds` - Provider API latency by provider, model
- `genai_provider_errors_total` - Provider errors by type

**Database Metrics:**
- `db_operation_duration_seconds` - Operation latency by operation, collection
- `db_errors_total` - Database errors by operation, collection

**Document Metrics:**
- `document_fetch_duration_seconds` - Fetch latency by operation
- `document_fetch_errors_total` - Fetch errors by error_type

### Billing and Metering

Per-tenant usage is tracked through:
1. **Prometheus metrics:** `genai_tokens_total` and `genai_requests_total` include `tenant_id` label
2. **MongoDB usage collection:** UsageRecord documents with tenant_id, user_id, model, tokens
3. **gRPC metering interceptor:** `grpc_metering_total` counter from service-core

Query tenant billing:
```promql
# Total tokens by tenant
sum(genai_tokens_total{tenant_id="acme"}) by (model, type)

# Requests by tenant
sum(genai_requests_total{tenant_id="acme"}) by (output_format, model)
```

### Tracing
- OpenTelemetry spans for all operations
- Trace ID propagation to providers
- Tenant/user context in spans
- Exports to Tempo via OTLP/gRPC

## Configuration

| Variable | Description | Default |
|----------|-------------|---------|
| `MONGODB_URI` | MongoDB connection string | (required in prod) |
| `MONGODB_DATABASE` | Database name | `genai_db` |
| `GOOGLE_API_KEY` | Google AI API key | (required in prod) |
| `GENAI_TEXT_MODEL` | Text/JSON model | `gemini-2.0-flash` |
| `GENAI_AUDIO_MODEL` | Audio model | `gemini-2.0-flash` |
| `GENAI_VIDEO_MODEL` | Video model | `veo-2` |
| `GENAI_DEFAULT_CONTENT_THRESHOLD_BYTES` | Max doc size for full output | `1048576` (1MB) |
| `DOCUMENT_SERVICE_GRPC_URL` | Document-service endpoint | `http://document-service:8081` |
| `AUTH_SERVICE_ENDPOINT` | Auth-service endpoint (enables capability enforcement) | (unset) |
| `OTLP_ENDPOINT` | OpenTelemetry collector | `http://tempo:4317` |
| `APP__PORT` | HTTP port (gRPC = port + 1) | `3000` |

## AI Providers

| Provider | Models | Status | Capabilities |
|----------|--------|--------|--------------|
| **Gemini** | gemini-2.0-flash | Implemented | Text, JSON, vision |
| **Veo** | veo-2 | Stub | Video generation |
| **Mock** | mock-* | Testing | All formats |

## References

- Proto Definition: `proto/micros/genai/v1/genai.proto`
- Google Gemini: https://ai.google.dev/gemini-api
- Document Service: `docs/specs/document-service.md`
