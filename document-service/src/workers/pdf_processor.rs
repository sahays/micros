use crate::models::{Document, ProcessingMetadata};
use crate::workers::executor::CommandExecutor;
use crate::workers::processor::Processor;
use async_trait::async_trait;
use service_core::error::AppError;
use std::path::Path;

#[derive(Default)]
pub struct PdfProcessor;

impl PdfProcessor {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Processor for PdfProcessor {
    fn supported_mime_types(&self) -> Vec<&str> {
        vec!["application/pdf"]
    }

    async fn process(
        &self,
        _document: &Document,
        file_path: &Path,
        executor: &CommandExecutor,
    ) -> Result<ProcessingMetadata, AppError> {
        tracing::info!(file_path = ?file_path, "Processing PDF document");

        // Extract text using pdftotext
        let output = executor
            .execute("pdftotext", &[file_path.to_str().unwrap(), "-"], None)
            .await?;

        let extracted_text = String::from_utf8_lossy(&output.stdout).to_string();

        // Get page count using pdfinfo
        let info_output = executor
            .execute("pdfinfo", &[file_path.to_str().unwrap()], None)
            .await?;

        let page_count = parse_page_count(&info_output.stdout)?;

        tracing::info!(
            page_count = page_count,
            text_length = extracted_text.len(),
            "PDF processing completed"
        );

        Ok(ProcessingMetadata {
            extracted_text: Some(extracted_text),
            page_count: Some(page_count),
            duration_seconds: None,
            optimized_size: None,
            thumbnail_path: None,
            error_details: None,
        })
    }
}

fn parse_page_count(output: &[u8]) -> Result<i32, AppError> {
    let output_str = String::from_utf8_lossy(output);

    for line in output_str.lines() {
        if line.starts_with("Pages:") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                return parts[1].parse::<i32>().map_err(|e| {
                    AppError::InternalError(anyhow::anyhow!("Failed to parse page count: {}", e))
                });
            }
        }
    }

    Err(AppError::InternalError(anyhow::anyhow!(
        "Page count not found in pdfinfo output"
    )))
}
