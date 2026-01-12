use crate::dtos::ProcessingOptions;
use crate::models::{ChunkInfo, Document, ProcessingMetadata};
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
        let output_resolution = video_opts
            .and_then(|opts| opts.resolution.as_ref())
            .map(|r| r.as_str())
            .unwrap_or("720p");

        // Get duration using ffprobe
        let duration = get_video_duration(file_path, executor).await?;

        // Downscale using ffmpeg
        let output_path = file_path.parent().unwrap().join(format!(
            "{}_{}.mp4",
            file_path.file_stem().unwrap().to_str().unwrap(),
            output_resolution
        ));

        let height = resolution_to_height(output_resolution);

        tracing::info!(
            output_path = ?output_path,
            resolution = output_resolution,
            height = height,
            "Downscaling video resolution"
        );

        executor
            .execute(
                "ffmpeg",
                &[
                    "-i",
                    file_path.to_str().unwrap(),
                    "-vf",
                    &format!("scale=-2:{}", height),
                    "-c:v",
                    "libx264",
                    "-crf",
                    "23",
                    "-c:a",
                    "aac",
                    output_path.to_str().unwrap(),
                ],
                None,
            )
            .await?;

        // Check size and chunk if needed
        let compressed_size = tokio::fs::metadata(&output_path)
            .await
            .map_err(|e| {
                AppError::InternalError(anyhow::anyhow!(
                    "Failed to read compressed file metadata: {}",
                    e
                ))
            })?
            .len();

        let chunk_threshold = 1_073_741_824; // 1GB

        if compressed_size > chunk_threshold {
            // Chunk the video
            tracing::info!(
                compressed_size = compressed_size,
                threshold = chunk_threshold,
                "Video exceeds threshold, chunking..."
            );

            let chunks = chunk_video(&output_path, chunk_threshold, executor, duration).await?;

            tracing::info!(
                duration_seconds = duration,
                resolution = output_resolution,
                chunk_count = chunks.len(),
                total_size = compressed_size,
                "Video processing completed (chunked)"
            );

            Ok(ProcessingMetadata {
                extracted_text: None,
                page_count: None,
                duration_seconds: Some(duration),
                optimized_size: None,
                thumbnail_path: None,
                error_details: None,
                resolution: Some(output_resolution.to_string()),
                chunks: Some(chunks.clone()),
                chunk_count: Some(chunks.len() as i32),
                total_size: Some(compressed_size as i64),
            })
        } else {
            // Single compressed file
            tracing::info!(
                duration_seconds = duration,
                resolution = output_resolution,
                optimized_size = compressed_size,
                "Video processing completed (single file)"
            );

            Ok(ProcessingMetadata {
                extracted_text: None,
                page_count: None,
                duration_seconds: Some(duration),
                optimized_size: Some(compressed_size as i64),
                thumbnail_path: Some(output_path.to_string_lossy().to_string()),
                error_details: None,
                resolution: Some(output_resolution.to_string()),
                chunks: None,
                chunk_count: None,
                total_size: None,
            })
        }
    }
}

async fn get_video_duration(file_path: &Path, executor: &CommandExecutor) -> Result<f64, AppError> {
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

    String::from_utf8_lossy(&probe_output.stdout)
        .trim()
        .parse::<f64>()
        .map_err(|e| {
            AppError::InternalError(anyhow::anyhow!("Failed to parse video duration: {}", e))
        })
}

async fn chunk_video(
    video_path: &Path,
    chunk_size: u64,
    executor: &CommandExecutor,
    total_duration: f64,
) -> Result<Vec<ChunkInfo>, AppError> {
    let mut chunks = Vec::new();
    let file_size = tokio::fs::metadata(video_path)
        .await
        .map_err(|e| {
            AppError::InternalError(anyhow::anyhow!("Failed to read video file metadata: {}", e))
        })?
        .len();

    // Calculate chunk duration based on size
    let chunk_duration = (total_duration * chunk_size as f64) / file_size as f64;
    let num_chunks = (total_duration / chunk_duration).ceil() as usize;

    tracing::info!(
        total_duration = total_duration,
        file_size = file_size,
        chunk_duration = chunk_duration,
        num_chunks = num_chunks,
        "Chunking video"
    );

    for i in 0..num_chunks {
        let start_time = i as f64 * chunk_duration;
        let chunk_path = video_path.parent().unwrap().join(format!(
            "{}_chunk_{}.mp4",
            video_path.file_stem().unwrap().to_str().unwrap(),
            i
        ));

        tracing::info!(
            chunk_index = i,
            start_time = start_time,
            duration = chunk_duration,
            output = ?chunk_path,
            "Extracting chunk"
        );

        // Extract chunk using ffmpeg
        executor
            .execute(
                "ffmpeg",
                &[
                    "-i",
                    video_path.to_str().unwrap(),
                    "-ss",
                    &start_time.to_string(),
                    "-t",
                    &chunk_duration.to_string(),
                    "-c",
                    "copy",
                    chunk_path.to_str().unwrap(),
                ],
                None,
            )
            .await?;

        let chunk_file_size = tokio::fs::metadata(&chunk_path)
            .await
            .map_err(|e| {
                AppError::InternalError(anyhow::anyhow!(
                    "Failed to read chunk {} metadata: {}",
                    i,
                    e
                ))
            })?
            .len();

        chunks.push(ChunkInfo {
            index: i,
            path: chunk_path.to_string_lossy().to_string(),
            size: chunk_file_size as i64,
        });
    }

    Ok(chunks)
}

fn resolution_to_height(resolution: &str) -> u32 {
    match resolution {
        "240p" => 240,
        "360p" => 360,
        "480p" => 480,
        "720p" => 720,
        "1080p" => 1080,
        _ => 720, // Default to 720p
    }
}
