//! gRPC implementation of AuditService.

use crate::grpc::proto::auth::{
    audit_service_server::AuditService, AuditEvent as ProtoAuditEvent, ListAuditEventsRequest,
    ListAuditEventsResponse,
};
use crate::AppState;
use prost_types::Timestamp;
use service_core::grpc::IntoStatus;
use tonic::{Request, Response, Status};
use uuid::Uuid;

/// gRPC AuditService implementation.
pub struct AuditServiceImpl {
    state: AppState,
}

impl AuditServiceImpl {
    /// Create a new AuditServiceImpl.
    pub fn new(state: AppState) -> Self {
        Self { state }
    }
}

/// Convert chrono DateTime to protobuf Timestamp.
fn datetime_to_timestamp(dt: chrono::DateTime<chrono::Utc>) -> Timestamp {
    Timestamp {
        seconds: dt.timestamp(),
        nanos: dt.timestamp_subsec_nanos() as i32,
    }
}

#[tonic::async_trait]
impl AuditService for AuditServiceImpl {
    async fn list_audit_events(
        &self,
        request: Request<ListAuditEventsRequest>,
    ) -> Result<Response<ListAuditEventsResponse>, Status> {
        let req = request.into_inner();

        let tenant_id = Uuid::parse_str(&req.tenant_id)
            .map_err(|_| Status::invalid_argument("Invalid tenant_id"))?;

        let actor_user_id = req
            .actor_user_id
            .map(|id| Uuid::parse_str(&id))
            .transpose()
            .map_err(|_| Status::invalid_argument("Invalid actor_user_id"))?;

        let target_id = req
            .target_id
            .map(|id| Uuid::parse_str(&id))
            .transpose()
            .map_err(|_| Status::invalid_argument("Invalid target_id"))?;

        let start_utc = req
            .start_utc
            .map(|ts| {
                chrono::DateTime::from_timestamp(ts.seconds, ts.nanos as u32)
                    .ok_or_else(|| Status::invalid_argument("Invalid start_utc"))
            })
            .transpose()?;

        let end_utc = req
            .end_utc
            .map(|ts| {
                chrono::DateTime::from_timestamp(ts.seconds, ts.nanos as u32)
                    .ok_or_else(|| Status::invalid_argument("Invalid end_utc"))
            })
            .transpose()?;

        let limit = req.limit.clamp(1, 100) as i64;
        let offset = req.offset.max(0) as i64;

        let (events, total_count) = self
            .state
            .db
            .find_audit_events(
                tenant_id,
                actor_user_id,
                req.event_type.as_deref(),
                req.target_type.as_deref(),
                target_id,
                start_utc,
                end_utc,
                limit,
                offset,
            )
            .await
            .map_err(|e| e.into_status())?;

        let proto_events: Vec<ProtoAuditEvent> = events
            .into_iter()
            .map(|e| ProtoAuditEvent {
                event_id: e.event_id.to_string(),
                tenant_id: e.tenant_id.map(|id| id.to_string()).unwrap_or_default(),
                actor_user_id: e.actor_user_id.map(|id| id.to_string()).unwrap_or_default(),
                event_type: e.event_type_code,
                target_type: e.target_type.unwrap_or_default(),
                target_id: e.target_id.map(|id| id.to_string()).unwrap_or_default(),
                details_json: e
                    .event_data
                    .map(|v| serde_json::to_string(&v).unwrap_or_default())
                    .unwrap_or_default(),
                ip_address: e.ip_address,
                user_agent: e.user_agent,
                created_utc: Some(datetime_to_timestamp(e.created_utc)),
            })
            .collect();

        Ok(Response::new(ListAuditEventsResponse {
            events: proto_events,
            total_count,
        }))
    }
}
