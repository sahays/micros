// Utils module

use anyhow::Result;
use base64::{engine::general_purpose, Engine as _};
use image::{DynamicImage, Luma};
use qrcode::QrCode;
use std::io::Cursor;

/// Generate a QR code as base64-encoded PNG image.
pub fn generate_qr_base64(data: &str) -> Result<String> {
    let code = QrCode::new(data)?;
    let image = code.render::<Luma<u8>>().build();

    let dynamic_image = DynamicImage::ImageLuma8(image);
    let mut buffer = Cursor::new(Vec::new());
    dynamic_image.write_to(&mut buffer, image::ImageOutputFormat::Png)?;

    Ok(general_purpose::STANDARD.encode(buffer.get_ref()))
}
