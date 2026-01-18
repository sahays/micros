use crate::config::UpiConfig;
use anyhow::Result;
use base64::{engine::general_purpose, Engine as _};
use image::{DynamicImage, Luma};
use qrcode::QrCode;
use std::io::Cursor;

pub struct UpiService {
    config: UpiConfig,
}

impl UpiService {
    pub fn new(config: UpiConfig) -> Self {
        Self { config }
    }

    pub fn generate_upi_link(
        &self,
        amount: f64,
        description: Option<String>,
        tr_id: Option<String>,
        vpa: Option<String>,
        merchant_name: Option<String>,
    ) -> String {
        let description = description.unwrap_or_else(|| "Payment".to_string());
        let vpa = vpa.unwrap_or_else(|| self.config.vpa.clone());
        let merchant_name = merchant_name.unwrap_or_else(|| self.config.merchant_name.clone());

        // Basic UPI intent format: upi://pay?pa=...&pn=...&am=...&cu=INR&tn=...
        // tr is transaction reference ID
        let mut link = format!(
            "upi://pay?pa={}&pn={}&am={:.2}&cu=INR&tn={}",
            vpa,
            urlencoding::encode(&merchant_name),
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
