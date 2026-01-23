# Epic: GenAI Service

## Labels
`epic`, `high-priority`, `backend`, `genai`

## Overview

A Rust-based gRPC microservice providing generative AI capabilities. Simple transactional API: input a prompt with optional document context, get output in the requested format (text, structured JSON, audio, or video).

## Goals

### Primary
- Simple input/output transaction model
- Support documents as context (via document-service signed URLs)
- Flexible output formats: free text, schema-conforming JSON, audio, video
- Both streaming and non-streaming responses

### Secondary
- Provider abstraction (OpenAI, Anthropic, etc.)
- Usage tracking per tenant/user

## Scope

### In Scope
- Generic prompt processing with document context
- Multiple output formats (text, JSON, audio, video)
- Streaming for text/audio outputs
- Conversation sessions (optional context persistence)

### Out of Scope
- Specialized operations (summarize, extract, etc.) - the prompt handles this
- Model training/fine-tuning
- Image generation (future)

---

## API Operations

```protobuf
service GenAIService {
  // Process a prompt and return result (non-streaming)
  rpc Process(ProcessRequest) returns (ProcessResponse);

  // Process a prompt and stream the result
  rpc ProcessStream(ProcessRequest) returns (stream ProcessStreamResponse);

  // Manage conversation sessions
  rpc CreateSession(CreateSessionRequest) returns (CreateSessionResponse);
  rpc GetSession(GetSessionRequest) returns (GetSessionResponse);
  rpc DeleteSession(DeleteSessionRequest) returns (DeleteSessionResponse);

  // Usage tracking
  rpc GetUsage(GetUsageRequest) returns (GetUsageResponse);

  // List available models
  rpc ListModels(ListModelsRequest) returns (ListModelsResponse);
}
```

---

## Request/Response Specifications

### Process (Non-Streaming)

```protobuf
message ProcessRequest {
  // The prompt/instruction
  string prompt = 1;

  // Optional document context (from document-service)
  repeated DocumentContext documents = 2;

  // Desired output format
  OutputFormat output_format = 3;

  // JSON schema (required if output_format is STRUCTURED_JSON)
  optional string output_schema = 4;

  // Optional session ID for conversation context
  optional string session_id = 5;

  // Generation parameters
  optional GenerationParams params = 6;

  // Request metadata (tenant, user)
  RequestMetadata metadata = 7;
}

// Note: Model is auto-selected based on output_format:
// - TEXT/STRUCTURED_JSON → GENAI_TEXT_MODEL (e.g., gemini-2.0-flash)
// - AUDIO → GENAI_AUDIO_MODEL (e.g., gemini-2.0-flash with audio)
// - VIDEO → GENAI_VIDEO_MODEL (e.g., veo-2)
// Models configured via environment variables.

message DocumentContext {
  // Document ID from document-service
  string document_id = 1;

  // Signed URL for access
  string signed_url = 2;

  // MIME type (e.g., "application/pdf", "image/png")
  string mime_type = 3;

  // Optional: pre-extracted text (skip fetching if provided)
  optional string text_content = 4;
}

enum OutputFormat {
  OUTPUT_FORMAT_UNSPECIFIED = 0;
  OUTPUT_FORMAT_TEXT = 1;           // Free-form text
  OUTPUT_FORMAT_STRUCTURED_JSON = 2; // JSON conforming to output_schema
  OUTPUT_FORMAT_AUDIO = 3;          // Audio bytes (e.g., TTS)
  OUTPUT_FORMAT_VIDEO = 4;          // Video bytes
}

message GenerationParams {
  optional float temperature = 1;
  optional float top_p = 2;
  repeated string stop_sequences = 3;

  // Content size threshold in bytes (default: 1MB = 1048576)
  // If total document size exceeds this, service uses max token output
  optional int64 content_size_threshold_bytes = 4;

  // Audio-specific
  optional string voice = 5;        // For TTS output
  optional string audio_format = 6; // "mp3", "wav", etc.

  // Video-specific
  optional string video_format = 7; // "mp4", etc.
  optional int32 duration_seconds = 8;
}

// Note: max_tokens is determined automatically:
// - If total document size > content_size_threshold_bytes: use model's max output tokens
// - Otherwise: use reasonable default based on output_format

message RequestMetadata {
  string tenant_id = 1;
  string user_id = 2;
  map<string, string> tags = 3;
}
```

```protobuf
message ProcessResponse {
  // The result (interpretation depends on output_format)
  oneof result {
    // Free-form text result
    string text = 1;

    // Structured JSON (as string, conforms to output_schema)
    string json = 2;

    // Audio bytes
    bytes audio = 3;

    // Video bytes
    bytes video = 4;
  }

  // Output format used
  OutputFormat output_format = 5;

  // Model that was auto-selected and used
  string model = 6;

  // Token usage (input + output tokens consumed)
  TokenUsage usage = 7;

  // Finish reason
  FinishReason finish_reason = 8;

  // Request ID for correlation
  string request_id = 9;

  // Session ID (if session was used/created)
  optional string session_id = 10;
}

// Token usage returned with every response
message TokenUsage {
  int32 input_tokens = 1;   // Tokens consumed by prompt + documents
  int32 output_tokens = 2;  // Tokens generated in response
  int32 total_tokens = 3;   // input_tokens + output_tokens
}

enum FinishReason {
  FINISH_REASON_UNSPECIFIED = 0;
  FINISH_REASON_COMPLETE = 1;
  FINISH_REASON_LENGTH = 2;
  FINISH_REASON_CONTENT_FILTER = 3;
  FINISH_REASON_ERROR = 4;
}
```

### ProcessStream (Streaming)

**Request:** Same `ProcessRequest`

**Response (streamed):**
```protobuf
message ProcessStreamResponse {
  oneof data {
    // Text chunk (for TEXT or STRUCTURED_JSON)
    string text_chunk = 1;

    // Audio chunk (for AUDIO)
    bytes audio_chunk = 2;

    // Video chunk (for VIDEO)
    bytes video_chunk = 3;

    // Final completion message
    StreamComplete complete = 4;
  }
}

message StreamComplete {
  OutputFormat output_format = 1;
  string model = 2;
  TokenUsage usage = 3;
  FinishReason finish_reason = 4;
  string request_id = 5;
  optional string session_id = 6;
}
```

### Session Management

```protobuf
message CreateSessionRequest {
  // Optional title
  optional string title = 1;

  // System prompt for this session
  optional string system_prompt = 2;

  // Default model for session
  optional string default_model = 3;

  // Persistent document context for all requests in session
  repeated DocumentContext documents = 4;

  RequestMetadata metadata = 5;
}

message CreateSessionResponse {
  Session session = 1;
}

message Session {
  string id = 1;
  optional string title = 2;
  optional string system_prompt = 3;
  optional string default_model = 4;
  repeated DocumentContext documents = 5;
  int32 message_count = 6;
  TokenUsage total_usage = 7;
  google.protobuf.Timestamp created_at = 8;
  google.protobuf.Timestamp updated_at = 9;
}

message GetSessionRequest {
  string session_id = 1;
  // Include full message history?
  bool include_messages = 2;
}

message GetSessionResponse {
  Session session = 1;
  // Messages if requested
  repeated SessionMessage messages = 2;
}

message SessionMessage {
  string role = 1;  // "user" or "assistant"
  string content = 2;
  OutputFormat output_format = 3;
  google.protobuf.Timestamp timestamp = 4;
}

message DeleteSessionRequest {
  string session_id = 1;
}

message DeleteSessionResponse {
  bool success = 1;
}
```

### Usage & Models

```protobuf
message GetUsageRequest {
  google.protobuf.Timestamp start_time = 1;
  google.protobuf.Timestamp end_time = 2;
  optional string tenant_id = 3;
  optional string user_id = 4;
}

message GetUsageResponse {
  int64 total_prompt_tokens = 1;
  int64 total_completion_tokens = 2;
  int64 total_tokens = 3;
  int32 total_requests = 4;
  repeated ModelUsage by_model = 5;
}

message ModelUsage {
  string model = 1;
  int64 tokens = 2;
  int32 requests = 3;
}

message ListModelsRequest {}

message ListModelsResponse {
  repeated ModelInfo models = 1;
}

message ModelInfo {
  string id = 1;
  string name = 2;
  string provider = 3;
  bool supports_vision = 4;
  bool supports_audio_output = 5;
  bool supports_video_output = 6;
  bool supports_streaming = 7;
  int32 context_window = 8;
}
```

---

## Example Usage

### 1. Simple text generation
```
prompt: "Write a haiku about rust programming"
output_format: TEXT
→ returns: text
```

### 2. Document summarization
```
prompt: "Summarize this document in 3 bullet points"
documents: [{document_id: "abc", signed_url: "..."}]
output_format: TEXT
→ returns: text
```

### 3. Structured extraction
```
prompt: "Extract all person names and their roles from this document"
documents: [{document_id: "abc", signed_url: "..."}]
output_format: STRUCTURED_JSON
output_schema: '{"type":"array","items":{"type":"object","properties":{"name":{"type":"string"},"role":{"type":"string"}}}}'
→ returns: json '[{"name":"John","role":"CEO"},...]'
```

### 4. Text-to-speech
```
prompt: "Read this text aloud: Hello world"
output_format: AUDIO
params: {voice: "alloy", audio_format: "mp3"}
→ returns: audio bytes
```

### 5. Multi-document Q&A with session
```
# First request - creates session
prompt: "What are the key differences between these two contracts?"
documents: [{doc1}, {doc2}]
output_format: TEXT
→ returns: text, session_id: "sess_123"

# Follow-up (session maintains context)
prompt: "Which one has better termination clauses?"
session_id: "sess_123"
output_format: TEXT
→ returns: text (knows about the contracts from session)
```

---

## Project Structure

```
genai-service/
├── Cargo.toml
├── Dockerfile
├── src/
│   ├── main.rs
│   ├── lib.rs
│   ├── startup.rs
│   ├── config/
│   │   └── mod.rs
│   ├── grpc/
│   │   ├── mod.rs
│   │   └── genai_service.rs
│   ├── handlers/
│   │   └── health.rs
│   ├── models/
│   │   ├── session.rs
│   │   └── usage.rs
│   └── services/
│       ├── mod.rs
│       ├── database.rs
│       ├── document_fetcher.rs
│       └── providers/
│           ├── mod.rs          # Provider trait
│           ├── gemini.rs       # Gemini (text, audio)
│           ├── veo.rs          # Veo (video)
│           └── mock.rs         # Mock for testing
└── tests/
```

---

## Stories

- [x] Story 1: Project Setup & Health Endpoint
- [x] Story 2: Gemini Provider & Process (text output)
- [x] Story 3: ProcessStream (streaming text)
- [x] Story 4: Document Fetching & Context Integration
- [x] Story 5: Structured JSON Output with Schema Validation
- [ ] ~~Story 6: Audio Output via Gemini TTS~~ (not required)
- [ ] ~~Story 7: Video Output via Veo~~ (not required)
- [x] Story 8: Session Management (MongoDB)
- [x] Story 9: Usage Tracking & Token Reporting
- [x] Story 10: Docker & Observability

---

## Configuration

```bash
# Service
GENAI_SERVICE_PORT=8080
GENAI_GRPC_PORT=8081

# MongoDB (session & usage storage)
GENAI_MONGODB_URI=mongodb://host.docker.internal:27017
GENAI_MONGODB_DATABASE=genai_db

# Model Selection (auto-selected based on output_format)
GENAI_TEXT_MODEL=gemini-2.0-flash          # For TEXT and STRUCTURED_JSON output
GENAI_AUDIO_MODEL=gemini-2.0-flash         # For AUDIO output (TTS)
GENAI_VIDEO_MODEL=veo-2                    # For VIDEO output

# Provider API Keys
GOOGLE_API_KEY=...                         # For Gemini/Veo models

# Token Limits
GENAI_DEFAULT_CONTENT_THRESHOLD_BYTES=1048576  # 1MB - use max tokens if exceeded

# Document Service
DOCUMENT_SERVICE_GRPC_URL=http://document-service:8081

# Observability
OTLP_ENDPOINT=http://tempo:4317
RUST_LOG=info
```

---

## Dependencies

- **document-service**: Fetch documents via gRPC (signed URLs)
- **MongoDB**: Session and usage storage
- **Google AI APIs**: Gemini (text/audio), Veo (video)
