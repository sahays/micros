use crate::models::{Document, ProcessingMetadata};
use crate::workers::executor::CommandExecutor;
use crate::workers::processor::Processor;
use async_trait::async_trait;
use service_core::error::AppError;
use std::path::Path;

#[derive(Default)]
pub struct VideoProcessor;

impl VideoProcessor {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Processor for VideoProcessor {
    fn supported_mime_types(&self) -> Vec<&str> {
        vec!["video/mp4", "video/quicktime", "video/x-msvideo"]
    }

    async fn process(
        &self,
        _document: &Document,
        file_path: &Path,
        executor: &CommandExecutor,
    ) -> Result<ProcessingMetadata, AppError> {
        tracing::info!(file_path = ?file_path, "Processing video document");

        let output_dir = file_path
            .parent()
            .unwrap()
            .join(file_path.file_stem().unwrap())
            .join("hls");

        tokio::fs::create_dir_all(&output_dir).await.map_err(|e| {
            AppError::InternalError(anyhow::anyhow!("Failed to create HLS directory: {}", e))
        })?;

        let playlist_path = output_dir.join("playlist.m3u8");

        // Transcode to HLS using ffmpeg
        executor
            .execute(
                "ffmpeg",
                &[
                    "-i",
                    file_path.to_str().unwrap(),
                    "-codec:",
                    "copy",
                    "-start_number",
                    "0",
                    "-hls_time",
                    "10",
                    "-hls_list_size",
                    "0",
                    "-f",
                    "hls",
                    playlist_path.to_str().unwrap(),
                ],
                None,
            )
            .await?;

        // Get duration using ffprobe
        let probe_output = executor
            .execute(
                "ffprobe",
                &[
                    "-v",
                    "error",
                    "-show_entries",
                    "format=duration",
                    "-of",
                    "default=noprint_wrappers=1:nokey=1",
                    file_path.to_str().unwrap(),
                ],
                None,
            )
            .await?;

        let duration = String::from_utf8_lossy(&probe_output.stdout)
            .trim()
            .parse::<f64>()
            .map_err(|e| {
                AppError::InternalError(anyhow::anyhow!("Failed to parse video duration: {}", e))
            })?;

        tracing::info!(
            duration_seconds = duration,
            playlist_path = ?playlist_path,
            "Video processing completed"
        );

        Ok(ProcessingMetadata {
            extracted_text: None,
            page_count: None,
            duration_seconds: Some(duration),
            optimized_size: None,
            thumbnail_path: Some(playlist_path.to_string_lossy().to_string()),
            error_details: None,
        })
    }
}
