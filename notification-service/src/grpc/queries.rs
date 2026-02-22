use crate::grpc::notification_service::{notification_to_proto, proto_to_channel, proto_to_status};
use crate::grpc::proto::{
    GetNotificationRequest, GetNotificationResponse, ListNotificationsRequest,
    ListNotificationsResponse, Notification as ProtoNotification,
};
use crate::startup::AppState;
use tonic::{Request, Response, Status};

#[tracing::instrument(skip(state, request))]
pub async fn get_notification(
    state: &AppState,
    request: Request<GetNotificationRequest>,
) -> Result<Response<GetNotificationResponse>, Status> {
    // Extract tenant context from request metadata
    let ctx = service_core::grpc::extract_tenant_context(&request)?;

    let req = request.into_inner();

    if req.notification_id.is_empty() {
        return Err(Status::invalid_argument("Notification ID is required"));
    }

    let notification = state
        .db
        .find_by_id(&ctx.tenant_id, &req.notification_id)
        .await
        .map_err(|e| Status::internal(format!("Database error: {}", e)))?;

    match notification {
        Some(n) => Ok(Response::new(GetNotificationResponse {
            notification: Some(notification_to_proto(&n)),
        })),
        None => Err(Status::not_found(format!(
            "Notification not found: {}",
            req.notification_id
        ))),
    }
}

#[tracing::instrument(skip(state, request))]
pub async fn list_notifications(
    state: &AppState,
    request: Request<ListNotificationsRequest>,
) -> Result<Response<ListNotificationsResponse>, Status> {
    // Extract tenant context from request metadata
    let ctx = service_core::grpc::extract_tenant_context(&request)?;

    let req = request.into_inner();

    // Parse channel filter
    let channel = req.channel.and_then(proto_to_channel);

    // Parse status filter
    let status = req.status.and_then(proto_to_status);

    // Clamp page size
    let limit = (req.page_size as i64).clamp(1, 100);
    let offset = req
        .page_token
        .as_ref()
        .and_then(|t| t.parse::<u64>().ok())
        .unwrap_or(0);

    let notifications = state
        .db
        .list(&ctx.tenant_id, channel, status, limit, offset)
        .await
        .map_err(|e| Status::internal(format!("Database error: {}", e)))?;

    let proto_notifications: Vec<ProtoNotification> =
        notifications.iter().map(notification_to_proto).collect();

    let next_offset = offset + proto_notifications.len() as u64;
    let next_page_token = if proto_notifications.len() == limit as usize {
        Some(next_offset.to_string())
    } else {
        None
    };

    Ok(Response::new(ListNotificationsResponse {
        notifications: proto_notifications,
        next_page_token,
        total_count: None, // Could add count query if needed
    }))
}
