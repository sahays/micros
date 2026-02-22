//! Shared conversion helpers for genai-service gRPC handlers.

use crate::config::OutputFormat;
use crate::grpc::proto::{
    FinishReason, OutputFormat as ProtoOutputFormat, ProcessRequest, ProcessStreamRequest,
    Session as ProtoSession, SessionMessage as ProtoSessionMessage, TokenUsage,
};
use crate::models::{Session, SessionMessage};
use crate::services::providers::{
    DocumentContext, FinishReason as ProviderFinishReason, GenerationParams, ProviderError,
};
use chrono::Utc;
use prost_types::Timestamp;
use tonic::Status;

/// Convert proto output format to internal enum.
pub fn proto_to_output_format(format: i32) -> OutputFormat {
    match format {
        2 => OutputFormat::StructuredJson,
        _ => OutputFormat::Text,
    }
}

/// Convert internal output format to proto enum.
pub fn output_format_to_proto(format: OutputFormat) -> i32 {
    match format {
        OutputFormat::Text => ProtoOutputFormat::Text as i32,
        OutputFormat::StructuredJson => ProtoOutputFormat::StructuredJson as i32,
    }
}

/// Convert output format to string for metrics.
pub fn output_format_str(format: OutputFormat) -> &'static str {
    match format {
        OutputFormat::Text => "text",
        OutputFormat::StructuredJson => "json",
    }
}

/// Convert provider finish reason to proto enum.
pub fn finish_reason_to_proto(reason: ProviderFinishReason) -> i32 {
    match reason {
        ProviderFinishReason::Complete => FinishReason::Complete as i32,
        ProviderFinishReason::Length => FinishReason::Length as i32,
        ProviderFinishReason::ContentFilter => FinishReason::ContentFilter as i32,
        ProviderFinishReason::Error => FinishReason::Error as i32,
    }
}

/// Convert provider error to gRPC status with error classification.
pub fn provider_error_to_status(error: ProviderError) -> Status {
    let (code, message, error_type) = match &error {
        ProviderError::NotConfigured(msg) => (
            tonic::Code::FailedPrecondition,
            msg.clone(),
            "not_configured",
        ),
        ProviderError::ApiError(msg) => (
            tonic::Code::Internal,
            format!("Provider API error: {}", msg),
            "api_error",
        ),
        ProviderError::InvalidRequest(msg) => {
            (tonic::Code::InvalidArgument, msg.clone(), "invalid_request")
        }
        ProviderError::RateLimited => (
            tonic::Code::ResourceExhausted,
            "Rate limited by AI provider".to_string(),
            "rate_limited",
        ),
        ProviderError::ContentFiltered => (
            tonic::Code::InvalidArgument,
            "Content was filtered by AI provider safety settings".to_string(),
            "content_filtered",
        ),
        ProviderError::NetworkError(msg) => (
            tonic::Code::Unavailable,
            format!("Network error: {}", msg),
            "network_error",
        ),
        ProviderError::Timeout(secs) => (
            tonic::Code::DeadlineExceeded,
            format!("AI provider request timed out after {}s", secs),
            "timeout",
        ),
    };

    tracing::error!(
        error_type = error_type,
        grpc_code = ?code,
        error = %error,
        "Provider error converted to gRPC status"
    );

    Status::new(code, message)
}

/// Convert proto document context to provider document context.
pub fn proto_to_document_context(doc: &crate::grpc::proto::DocumentContext) -> DocumentContext {
    DocumentContext {
        document_id: doc.document_id.clone(),
        mime_type: doc.mime_type.clone(),
        text_content: doc.text_content.clone(),
    }
}

/// Build generation params from request.
pub fn build_generation_params(
    req: &ProcessRequest,
    output_format: OutputFormat,
    model: Option<String>,
) -> GenerationParams {
    let params = req.params.as_ref();

    GenerationParams {
        temperature: params.and_then(|p| p.temperature),
        top_p: params.and_then(|p| p.top_p),
        max_tokens: None,
        stop_sequences: params.map(|p| p.stop_sequences.clone()).unwrap_or_default(),
        output_schema: if output_format == OutputFormat::StructuredJson {
            req.output_schema.clone()
        } else {
            None
        },
        model,
        tenant_context: None,
    }
}

/// Build generation params from stream request.
pub fn build_stream_generation_params(
    req: &ProcessStreamRequest,
    output_format: OutputFormat,
    model: Option<String>,
) -> GenerationParams {
    let params = req.params.as_ref();

    GenerationParams {
        temperature: params.and_then(|p| p.temperature),
        top_p: params.and_then(|p| p.top_p),
        max_tokens: None,
        stop_sequences: params.map(|p| p.stop_sequences.clone()).unwrap_or_default(),
        output_schema: if output_format == OutputFormat::StructuredJson {
            req.output_schema.clone()
        } else {
            None
        },
        model,
        tenant_context: None,
    }
}

/// Convert chrono DateTime to protobuf Timestamp.
pub fn datetime_to_timestamp(dt: chrono::DateTime<Utc>) -> Option<Timestamp> {
    Some(Timestamp {
        seconds: dt.timestamp(),
        nanos: dt.timestamp_subsec_nanos() as i32,
    })
}

/// Convert Session model to proto Session.
pub fn session_to_proto(session: &Session) -> ProtoSession {
    ProtoSession {
        id: session.session_id.clone(),
        title: session.title.clone(),
        system_prompt: session.system_prompt.clone(),
        documents: session
            .documents
            .iter()
            .map(|d| crate::grpc::proto::DocumentContext {
                document_id: d.document_id.clone(),
                mime_type: d.mime_type.clone(),
                text_content: d.text_content.clone(),
            })
            .collect(),
        message_count: session.message_count,
        total_usage: Some(TokenUsage {
            input_tokens: session.total_input_tokens,
            output_tokens: session.total_output_tokens,
            total_tokens: session.total_input_tokens + session.total_output_tokens,
        }),
        created_at: datetime_to_timestamp(session.created_at),
        updated_at: datetime_to_timestamp(session.updated_at),
    }
}

/// Convert SessionMessage model to proto SessionMessage.
pub fn session_message_to_proto(msg: &SessionMessage) -> ProtoSessionMessage {
    ProtoSessionMessage {
        role: msg.role.clone(),
        content: msg.content.clone(),
        output_format: msg
            .output_format
            .as_ref()
            .map(|f| match f.as_str() {
                "text" => ProtoOutputFormat::Text as i32,
                "json" => ProtoOutputFormat::StructuredJson as i32,
                _ => ProtoOutputFormat::Unspecified as i32,
            })
            .unwrap_or(ProtoOutputFormat::Unspecified as i32),
        timestamp: datetime_to_timestamp(msg.timestamp),
    }
}
