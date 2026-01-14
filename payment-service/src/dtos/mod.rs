use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Deserialize)]
pub struct QrGenerateRequest {
    pub amount: f64,
    pub description: Option<String>,
    pub transaction_id: Option<Uuid>,
    pub vpa: Option<String>,
    pub merchant_name: Option<String>,
}

#[derive(Serialize)]
pub struct QrGenerateResponse {
    pub upi_link: String,
    pub qr_image_base64: Option<String>,
}