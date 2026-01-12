use crate::dtos::ProcessingOptions;
use crate::models::{Document, ProcessingMetadata};
use crate::workers::executor::CommandExecutor;
use crate::workers::processor::Processor;
use async_trait::async_trait;
use service_core::error::AppError;
use std::path::Path;

#[derive(Default)]
pub struct ImageProcessor;

impl ImageProcessor {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Processor for ImageProcessor {
    fn supported_mime_types(&self) -> Vec<&str> {
        vec!["image/jpeg", "image/png", "image/gif", "image/bmp"]
    }

    async fn process(
        &self,
        _document: &Document,
        file_path: &Path,
        executor: &CommandExecutor,
        options: &ProcessingOptions,
    ) -> Result<ProcessingMetadata, AppError> {
        tracing::info!(file_path = ?file_path, "Processing image document");

        // Get image-specific options or use defaults
        let img_opts = options.image_options.as_ref();
        let format = img_opts.map_or("webp", |opts| opts.format.as_str());
        let quality = img_opts.map_or(85, |opts| opts.quality);

        let output_path = file_path.with_extension(format);

        // Convert image using ImageMagick
        let quality_str = quality.to_string();
        executor
            .execute(
                "convert",
                &[
                    file_path.to_str().unwrap(),
                    "-quality",
                    &quality_str,
                    "-define",
                    &format!("{}:method=6", format),
                    output_path.to_str().unwrap(),
                ],
                None,
            )
            .await?;

        let optimized_size = tokio::fs::metadata(&output_path)
            .await
            .map_err(|e| {
                AppError::InternalError(anyhow::anyhow!(
                    "Failed to read output file metadata: {}",
                    e
                ))
            })?
            .len() as i64;

        tracing::info!(
            optimized_size = optimized_size,
            output_path = ?output_path,
            format = format,
            quality = quality,
            "Image processing completed"
        );

        Ok(ProcessingMetadata {
            extracted_text: None,
            page_count: None,
            duration_seconds: None,
            optimized_size: Some(optimized_size),
            thumbnail_path: Some(output_path.to_string_lossy().to_string()),
            error_details: None,
        })
    }
}
