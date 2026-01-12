use crate::dtos::ProcessingOptions;
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
        options: &ProcessingOptions,
    ) -> Result<ProcessingMetadata, AppError> {
        tracing::info!(file_path = ?file_path, "Processing video document");

        // Get video-specific options or use defaults
        let video_opts = options.video_options.as_ref();
        let format = video_opts.map_or("hls", |opts| opts.format.as_str());

        let output_dir = file_path
            .parent()
            .unwrap()
            .join(file_path.file_stem().unwrap())
            .join(format);

        tokio::fs::create_dir_all(&output_dir).await.map_err(|e| {
            AppError::InternalError(anyhow::anyhow!("Failed to create output directory: {}", e))
        })?;

        let playlist_path = output_dir.join("playlist.m3u8");
        let mp4_output_path = output_dir.join("output.mp4");

        // Build ffmpeg arguments based on format
        let mut args = vec!["-i", file_path.to_str().unwrap()];

        // Add resolution if specified
        if let Some(resolution) = video_opts.and_then(|opts| opts.resolution.as_ref()) {
            args.extend_from_slice(&["-s", resolution.as_str()]);
        }

        // Add format-specific arguments
        if format == "hls" {
            args.extend_from_slice(&[
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
            ]);
        } else {
            // Default MP4 output
            args.extend_from_slice(&["-codec:", "copy", mp4_output_path.to_str().unwrap()]);
        }

        // Transcode video using ffmpeg
        executor.execute("ffmpeg", &args, None).await?;

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
            format = format,
            resolution = ?video_opts.and_then(|opts| opts.resolution.as_ref()),
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
