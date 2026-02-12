//! gRPC handlers for session and usage operations.

use crate::grpc::capability_check::capabilities;
use crate::grpc::helpers::{session_message_to_proto, session_to_proto};
use crate::grpc::proto::{
    CreateSessionRequest, CreateSessionResponse, DeleteSessionRequest, DeleteSessionResponse,
    GetSessionRequest, GetSessionResponse, GetUsageRequest, GetUsageResponse, ListModelsRequest,
    ListModelsResponse, ModelInfo,
};
use crate::models::{Session, SessionDocument};
use crate::services::metrics::{dec_grpc_in_flight, inc_grpc_in_flight, record_grpc_request};
use crate::startup::AppState;
use std::time::Instant;
use tonic::{Request, Response, Status};

#[tracing::instrument(skip(state, request), fields(tenant_id, user_id, session_id))]
pub async fn create_session(
    state: &AppState,
    request: Request<CreateSessionRequest>,
) -> Result<Response<CreateSessionResponse>, Status> {
    let start = Instant::now();
    let method = "CreateSession";
    inc_grpc_in_flight(method);

    let auth = state
        .capability_checker
        .require_capability(&request, capabilities::GENAI_SESSION_CREATE)
        .await?;

    let req = request.into_inner();

    let span = tracing::Span::current();
    span.record("tenant_id", &auth.tenant_id);
    span.record("user_id", &auth.user_id);

    let documents: Vec<SessionDocument> = req
        .documents
        .iter()
        .map(|d| {
            SessionDocument::new(
                d.document_id.clone(),
                d.signed_url.clone(),
                d.mime_type.clone(),
                d.text_content.clone(),
            )
        })
        .collect();

    let session = Session::new(
        auth.tenant_id.clone(),
        auth.user_id.clone(),
        req.title,
        req.system_prompt,
        documents,
    );

    span.record("session_id", &session.session_id);

    tracing::info!("Creating session");

    if let Err(e) = state.db.insert_session(&session).await {
        dec_grpc_in_flight(method);
        record_grpc_request(method, "INTERNAL", start.elapsed().as_secs_f64());
        tracing::error!(error = %e, "Failed to create session");
        return Err(Status::internal(format!("Failed to create session: {}", e)));
    }

    let duration = start.elapsed();
    dec_grpc_in_flight(method);
    record_grpc_request(method, "OK", duration.as_secs_f64());

    tracing::info!(
        duration_ms = duration.as_millis(),
        "Session created successfully"
    );

    Ok(Response::new(CreateSessionResponse {
        session: Some(session_to_proto(&session)),
    }))
}

#[tracing::instrument(skip(state, request), fields(session_id))]
pub async fn get_session(
    state: &AppState,
    request: Request<GetSessionRequest>,
) -> Result<Response<GetSessionResponse>, Status> {
    let start = Instant::now();
    let method = "GetSession";
    inc_grpc_in_flight(method);

    let auth = state
        .capability_checker
        .require_capability(&request, capabilities::GENAI_SESSION_READ)
        .await?;

    let req = request.into_inner();

    if req.session_id.is_empty() {
        dec_grpc_in_flight(method);
        record_grpc_request(method, "INVALID_ARGUMENT", start.elapsed().as_secs_f64());
        tracing::warn!("GetSession called with empty session_id");
        return Err(Status::invalid_argument("session_id is required"));
    }

    let span = tracing::Span::current();
    span.record("session_id", &req.session_id);

    tracing::info!(include_messages = req.include_messages, "Getting session");

    let session = match state
        .db
        .find_session(&auth.tenant_id, &req.session_id)
        .await
    {
        Ok(Some(s)) => s,
        Ok(None) => {
            dec_grpc_in_flight(method);
            record_grpc_request(method, "NOT_FOUND", start.elapsed().as_secs_f64());
            tracing::warn!("Session not found");
            return Err(Status::not_found(format!(
                "Session not found: {}",
                req.session_id
            )));
        }
        Err(e) => {
            dec_grpc_in_flight(method);
            record_grpc_request(method, "INTERNAL", start.elapsed().as_secs_f64());
            tracing::error!(error = %e, "Failed to get session");
            return Err(Status::internal(format!("Failed to get session: {}", e)));
        }
    };

    let messages = if req.include_messages {
        session
            .messages
            .iter()
            .map(session_message_to_proto)
            .collect()
    } else {
        vec![]
    };

    let duration = start.elapsed();
    dec_grpc_in_flight(method);
    record_grpc_request(method, "OK", duration.as_secs_f64());

    tracing::info!(
        duration_ms = duration.as_millis(),
        message_count = messages.len(),
        "Session retrieved"
    );

    Ok(Response::new(GetSessionResponse {
        session: Some(session_to_proto(&session)),
        messages,
    }))
}

#[tracing::instrument(skip(state, request), fields(session_id))]
pub async fn delete_session(
    state: &AppState,
    request: Request<DeleteSessionRequest>,
) -> Result<Response<DeleteSessionResponse>, Status> {
    let start = Instant::now();
    let method = "DeleteSession";
    inc_grpc_in_flight(method);

    let auth = state
        .capability_checker
        .require_capability(&request, capabilities::GENAI_SESSION_DELETE)
        .await?;

    let req = request.into_inner();

    if req.session_id.is_empty() {
        dec_grpc_in_flight(method);
        record_grpc_request(method, "INVALID_ARGUMENT", start.elapsed().as_secs_f64());
        tracing::warn!("DeleteSession called with empty session_id");
        return Err(Status::invalid_argument("session_id is required"));
    }

    let span = tracing::Span::current();
    span.record("session_id", &req.session_id);

    tracing::info!("Deleting session");

    let success = match state
        .db
        .delete_session(&auth.tenant_id, &req.session_id)
        .await
    {
        Ok(deleted) => deleted,
        Err(e) => {
            dec_grpc_in_flight(method);
            record_grpc_request(method, "INTERNAL", start.elapsed().as_secs_f64());
            tracing::error!(error = %e, "Failed to delete session");
            return Err(Status::internal(format!("Failed to delete session: {}", e)));
        }
    };

    if !success {
        dec_grpc_in_flight(method);
        record_grpc_request(method, "NOT_FOUND", start.elapsed().as_secs_f64());
        tracing::warn!("Session not found for deletion");
        return Err(Status::not_found(format!(
            "Session not found: {}",
            req.session_id
        )));
    }

    let duration = start.elapsed();
    dec_grpc_in_flight(method);
    record_grpc_request(method, "OK", duration.as_secs_f64());

    tracing::info!(duration_ms = duration.as_millis(), "Session deleted");

    Ok(Response::new(DeleteSessionResponse { success: true }))
}

#[tracing::instrument(skip(state, request), fields(tenant_id, user_id))]
pub async fn get_usage(
    state: &AppState,
    request: Request<GetUsageRequest>,
) -> Result<Response<GetUsageResponse>, Status> {
    let start = Instant::now();
    let method = "GetUsage";
    inc_grpc_in_flight(method);

    let auth = state
        .capability_checker
        .require_capability(&request, capabilities::GENAI_USAGE_READ)
        .await?;

    let req = request.into_inner();

    let span = tracing::Span::current();
    span.record("tenant_id", &auth.tenant_id);
    if let Some(ref uid) = req.user_id {
        span.record("user_id", uid);
    }

    let start_time = req
        .start_time
        .as_ref()
        .and_then(|t| chrono::DateTime::from_timestamp(t.seconds, t.nanos as u32))
        .ok_or_else(|| {
            dec_grpc_in_flight(method);
            record_grpc_request(method, "INVALID_ARGUMENT", start.elapsed().as_secs_f64());
            tracing::warn!("Invalid or missing start_time");
            Status::invalid_argument("start_time is required")
        })?;

    let end_time = req
        .end_time
        .as_ref()
        .and_then(|t| chrono::DateTime::from_timestamp(t.seconds, t.nanos as u32))
        .ok_or_else(|| {
            dec_grpc_in_flight(method);
            record_grpc_request(method, "INVALID_ARGUMENT", start.elapsed().as_secs_f64());
            tracing::warn!("Invalid or missing end_time");
            Status::invalid_argument("end_time is required")
        })?;

    tracing::info!(
        start_time = %start_time,
        end_time = %end_time,
        "Getting usage statistics"
    );

    let records = match state
        .db
        .get_usage(
            &auth.tenant_id,
            req.user_id.as_deref(),
            start_time,
            end_time,
        )
        .await
    {
        Ok(r) => r,
        Err(e) => {
            dec_grpc_in_flight(method);
            record_grpc_request(method, "INTERNAL", start.elapsed().as_secs_f64());
            tracing::error!(error = %e, "Failed to get usage");
            return Err(Status::internal(format!("Failed to get usage: {}", e)));
        }
    };

    let stats = crate::models::UsageStats::from_records(&records);

    let by_model: Vec<crate::grpc::proto::ModelUsage> = stats
        .by_model
        .values()
        .map(|m| crate::grpc::proto::ModelUsage {
            model: m.model.clone(),
            tokens: m.tokens,
            requests: m.requests,
        })
        .collect();

    let duration = start.elapsed();
    dec_grpc_in_flight(method);
    record_grpc_request(method, "OK", duration.as_secs_f64());

    tracing::info!(
        duration_ms = duration.as_millis(),
        record_count = records.len(),
        total_requests = stats.total_requests,
        "Usage statistics retrieved"
    );

    Ok(Response::new(GetUsageResponse {
        total_input_tokens: stats.total_input_tokens,
        total_output_tokens: stats.total_output_tokens,
        total_tokens: stats.total_tokens,
        total_requests: stats.total_requests,
        by_model,
    }))
}

#[tracing::instrument(skip(state, request))]
pub async fn list_models(
    state: &AppState,
    request: Request<ListModelsRequest>,
) -> Result<Response<ListModelsResponse>, Status> {
    let start = Instant::now();
    let method = "ListModels";
    inc_grpc_in_flight(method);

    let _auth = state
        .capability_checker
        .require_capability(&request, capabilities::GENAI_MODELS_READ)
        .await?;

    tracing::debug!("Listing available models");

    let models = vec![
        ModelInfo {
            id: state.config.models.text_model.clone(),
            name: "Text Model".to_string(),
            provider: "google".to_string(),
            supports_vision: true,
            supports_audio_output: false,
            supports_video_output: false,
            supports_streaming: true,
            context_window: 1_000_000,
        },
        ModelInfo {
            id: state.config.models.audio_model.clone(),
            name: "Audio Model".to_string(),
            provider: "google".to_string(),
            supports_vision: true,
            supports_audio_output: true,
            supports_video_output: false,
            supports_streaming: true,
            context_window: 1_000_000,
        },
        ModelInfo {
            id: state.config.models.video_model.clone(),
            name: "Video Model".to_string(),
            provider: "google".to_string(),
            supports_vision: false,
            supports_audio_output: false,
            supports_video_output: true,
            supports_streaming: false,
            context_window: 0,
        },
    ];

    let duration = start.elapsed();
    dec_grpc_in_flight(method);
    record_grpc_request(method, "OK", duration.as_secs_f64());

    tracing::debug!(
        duration_ms = duration.as_millis(),
        model_count = models.len(),
        "Models listed"
    );

    Ok(Response::new(ListModelsResponse { models }))
}
