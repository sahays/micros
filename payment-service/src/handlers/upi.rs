use crate::{
    dtos::{QrGenerateRequest, QrGenerateResponse},
    middleware::TenantContext,
    services::upi::UpiService,
    AppState,
};
use axum::{extract::State, http::StatusCode, Json};

/// Generate a UPI QR code for payment.
///
/// Accepts tenant context from the BFF (secure-frontend) via X-App-ID, X-Org-ID headers.
pub async fn generate_qr(
    State(state): State<AppState>,
    tenant: TenantContext,
    Json(payload): Json<QrGenerateRequest>,
) -> Result<Json<QrGenerateResponse>, StatusCode> {
    tracing::info!(
        app_id = %tenant.app_id,
        org_id = %tenant.org_id,
        user_id = ?tenant.user_id,
        amount = payload.amount,
        "Generating UPI QR code"
    );

    let service = UpiService::new(state.config.upi.clone());

    let tr_id = payload.transaction_id.map(|id| id.to_string());
    let upi_link = service.generate_upi_link(
        payload.amount,
        payload.description,
        tr_id,
        payload.vpa,
        payload.merchant_name,
    );

    let qr_image = service.generate_qr_base64(&upi_link).map_err(|e| {
        tracing::error!("Failed to generate QR code: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(QrGenerateResponse {
        upi_link,
        qr_image_base64: Some(qr_image),
    }))
}
