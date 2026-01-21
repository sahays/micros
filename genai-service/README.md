# GenAI Service

Generative AI service with Gemini integration, supporting text generation with document context.

## Architecture

**gRPC-only** internal service. External clients access via BFF.

- **gRPC**: Port 50055 (all business logic)
- **HTTP**: Port 8080 (health checks and metrics)

## gRPC Service

### GenAIService

```protobuf
// Text generation
rpc Process(ProcessRequest) returns (ProcessResponse)
rpc ProcessStream(ProcessStreamRequest) returns (stream ProcessStreamResponse)

// Session management
rpc CreateSession(CreateSessionRequest) returns (CreateSessionResponse)
rpc GetSession(GetSessionRequest) returns (GetSessionResponse)
rpc DeleteSession(DeleteSessionRequest) returns (DeleteSessionResponse)

// Usage and models
rpc GetUsage(GetUsageRequest) returns (GetUsageResponse)
rpc ListModels(ListModelsRequest) returns (ListModelsResponse)
```

## Output Formats

| Format | Description |
|--------|-------------|
| `TEXT` | Free-form text response |
| `STRUCTURED_JSON` | JSON conforming to provided schema |

## Usage (grpcurl)

```bash
# List services
grpcurl -plaintext localhost:50055 list

# Simple text generation
grpcurl -plaintext -d '{
  "prompt": "Explain microservices in one sentence",
  "output_format": 1,
  "metadata": {"tenant_id": "t-123", "user_id": "u-456"}
}' localhost:50055 micros.genai.v1.GenAIService/Process

# Structured JSON output
grpcurl -plaintext -d '{
  "prompt": "List 3 programming languages with their use cases",
  "output_format": 2,
  "output_schema": "{\"type\":\"array\",\"items\":{\"type\":\"object\",\"properties\":{\"name\":{\"type\":\"string\"},\"use_case\":{\"type\":\"string\"}}}}",
  "metadata": {"tenant_id": "t-123", "user_id": "u-456"}
}' localhost:50055 micros.genai.v1.GenAIService/Process

# With document context
grpcurl -plaintext -d '{
  "prompt": "Summarize this document",
  "documents": [{"document_id": "doc-abc", "signed_url": "https://...", "mime_type": "application/pdf"}],
  "output_format": 1,
  "metadata": {"tenant_id": "t-123", "user_id": "u-456"}
}' localhost:50055 micros.genai.v1.GenAIService/Process

# Streaming response
grpcurl -plaintext -d '{
  "prompt": "Write a short story about a robot",
  "output_format": 1,
  "metadata": {"tenant_id": "t-123", "user_id": "u-456"}
}' localhost:50055 micros.genai.v1.GenAIService/ProcessStream

# Create session
grpcurl -plaintext -d '{
  "title": "Code Review Session",
  "system_prompt": "You are a code reviewer",
  "metadata": {"tenant_id": "t-123", "user_id": "u-456"}
}' localhost:50055 micros.genai.v1.GenAIService/CreateSession

# List models
grpcurl -plaintext localhost:50055 micros.genai.v1.GenAIService/ListModels
```

## Configuration

| Variable | Description |
|----------|-------------|
| `MONGODB_URI` | MongoDB connection |
| `MONGODB_DATABASE` | Database name |
| `GOOGLE_API_KEY` | Gemini API key |
| `TEXT_MODEL` | Text model (default: gemini-1.5-flash) |
| `DOCUMENT_SERVICE_GRPC_URL` | Document service endpoint |
| `HTTP_PORT` | Health/metrics port (default: 8080) |
| `GRPC_PORT` | gRPC port (default: 50055) |

## Health Checks

```bash
# HTTP
curl http://localhost:8080/health
curl http://localhost:8080/ready

# Metrics
curl http://localhost:8080/metrics

# gRPC
grpcurl -plaintext localhost:50055 grpc.health.v1.Health/Check
```

## Proto Definitions

See `proto/micros/genai/v1/genai.proto`
