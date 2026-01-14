use axum::{Json, extract::State, http::StatusCode};
use crate::{AppState, dtos::{QrGenerateRequest, QrGenerateResponse}, services::upi::UpiService};

pub async fn generate_qr(
    State(state): State<AppState>,
    Json(payload): Json<QrGenerateRequest>,
) -> Result<Json<QrGenerateResponse>, StatusCode> {
    let service = UpiService::new(state.config.upi.clone());
    
    let tr_id = payload.transaction_id.map(|id| id.to_string());
    let upi_link = service.generate_upi_link(payload.amount, payload.description, tr_id);
    
    let qr_image = service.generate_qr_base64(&upi_link).map_err(|e| {
        tracing::error!("Failed to generate QR code: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(QrGenerateResponse {
        upi_link,
        qr_image_base64: Some(qr_image),
    }))
}
