# Story: Per-Request Model Selection

- [x] **Status: Complete**
- **Epic:** [epic-genai-service](../../epic-genai-service.md)

## Summary

Add an optional `model` field to `ProcessRequest` and `ProcessStreamRequest` so clients can override the auto-selected model on a per-request basis. When omitted, the existing behavior (auto-select based on `output_format`) is preserved. When set, the requested model is validated against configured models and used for the request.

## Tasks

- [x] Add `optional string model = 8` to ProcessRequest in genai.proto
- [x] Add `optional string model = 8` to ProcessStreamRequest in genai.proto
- [x] Add `GenaiConfig::is_valid_model(&self, model: &str) -> bool` to config
- [x] Add `pub model: Option<String>` to `GenerationParams` in providers/mod.rs
- [x] Update `GeminiTextProvider::api_url` to accept model parameter
- [x] Update `GeminiTextProvider::generate()` to resolve model from params
- [x] Update `GeminiTextProvider::generate_stream()` to resolve model from params
- [x] Update `process()` gRPC handler to validate and pass model override
- [x] Update `process_stream()` gRPC handler to validate and pass model override
- [x] Update `build_generation_params()` to include model in GenerationParams
- [x] Update `build_stream_generation_params()` to include model in GenerationParams
- [x] Add integration test: process rejects unknown model with INVALID_ARGUMENT

## gRPC Method Changes

### Process / ProcessStream

**New field on ProcessRequest and ProcessStreamRequest:**
```protobuf
// Optional model override. When set, overrides the auto-selected model
// based on output_format. Must be one of the configured models
// (text_model, audio_model, video_model). When omitted, auto-selects
// based on output_format as before.
optional string model = 8;
```

**Validation:**
- If `model` is set, validate it exists in configured models (`text_model`, `audio_model`, `video_model`)
- Return `INVALID_ARGUMENT` with message listing valid models if the requested model is unknown
- If `model` is empty string, treat as unset (use auto-selection)

**Business Logic:**
- When `model` is set and valid: use the specified model for the Gemini API call
- When `model` is unset: auto-select based on `output_format` (existing behavior)
- The response `model` field always reflects the model that was actually used

## Files Modified

| File | Change |
|------|--------|
| `proto/micros/genai/v1/genai.proto` | Add `optional string model = 8` to ProcessRequest and ProcessStreamRequest |
| `genai-service/src/config/mod.rs` | Add `is_valid_model()` and `valid_models()` methods |
| `genai-service/src/services/providers/mod.rs` | Add `model: Option<String>` to GenerationParams |
| `genai-service/src/services/providers/gemini.rs` | Update `api_url()`, `generate()`, `generate_stream()` to use model override |
| `genai-service/src/grpc/genai_service.rs` | Validate model, resolve override, pass to GenerationParams |
| `workflow-tests/tests/genai_tests/grpc.rs` | Add test for invalid model rejection |

## Acceptance Criteria

- [x] Process with valid model override uses the specified model
- [x] Process without model field auto-selects based on output_format (no regression)
- [x] Process with unknown model returns INVALID_ARGUMENT with valid model list
- [x] Process with empty string model auto-selects (same as unset)
- [x] ProcessStream with valid model override uses the specified model
- [x] ProcessStream without model field auto-selects (no regression)
- [x] ProcessStream with unknown model returns INVALID_ARGUMENT
- [x] Response model field reflects the model actually used
- [x] ListModels returns all valid models for client discovery

## Integration Tests

- [x] Process with unknown model returns INVALID_ARGUMENT
- [ ] Process with valid model override succeeds (API test, gated by SKIP_API_TESTS)
- [x] Process without model field succeeds (existing tests cover this)
