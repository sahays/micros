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
    ) -> Result<ProcessingMetadata, AppError> {
        tracing::info!(file_path = ?file_path, "Processing image document");

        let output_path = file_path.with_extension("webp");

        // Convert to WebP using ImageMagick
        executor
            .execute(
                "convert",
                &[
                    file_path.to_str().unwrap(),
                    "-quality",
                    "85",
                    "-define",
                    "webp:method=6",
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
