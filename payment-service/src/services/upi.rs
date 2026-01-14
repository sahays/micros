use anyhow::Result;
use qrcode::QrCode;
use image::{Luma, DynamicImage};
use base64::{Engine as _, engine::general_purpose};
use std::io::Cursor;
use crate::config::UpiConfig;

pub struct UpiService {
    config: UpiConfig,
}

impl UpiService {
    pub fn new(config: UpiConfig) -> Self {
        Self { config }
    }

    pub fn generate_upi_link(&self, amount: f64, description: Option<String>, tr_id: Option<String>) -> String {
        let description = description.unwrap_or_else(|| "Payment".to_string());
        // Basic UPI intent format: upi://pay?pa=...&pn=...&am=...&cu=INR&tn=...
        // tr is transaction reference ID
        let mut link = format!(
            "upi://pay?pa={}&pn={}&am={:.2}&cu=INR&tn={}",
            self.config.vpa,
            urlencoding::encode(&self.config.merchant_name),
            amount,
            urlencoding::encode(&description)
        );

        if let Some(id) = tr_id {
            link.push_str(&format!("&tr={}", id));
        }

        link
    }

    pub fn generate_qr_base64(&self, upi_link: &str) -> Result<String> {
        let code = QrCode::new(upi_link)?;
        let image = code.render::<Luma<u8>>().build();
        
        let dynamic_image = DynamicImage::ImageLuma8(image);
        let mut buffer = Cursor::new(Vec::new());
        dynamic_image.write_to(&mut buffer, image::ImageOutputFormat::Png)?;
        
        Ok(general_purpose::STANDARD.encode(buffer.get_ref()))
    }
}
