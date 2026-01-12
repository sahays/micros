use crate::models::{Document, DocumentStatus, ProcessingMetadata};
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ProcessorType {
    Pdf,
    Image,
    Video,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, Validate)]
pub struct ProcessingOptions {
    /// List of processors to run. If None, all applicable processors will run.
    pub processors: Option<Vec<ProcessorType>>,
    pub pdf_options: Option<PdfOptions>,
    pub image_options: Option<ImageOptions>,
    pub video_options: Option<VideoOptions>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PdfOptions {
    #[serde(default = "default_true")]
    pub extract_text: bool,
    #[serde(default)]
    pub extract_images: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct ImageOptions {
    /// Output format: "webp", "jpeg", "png"
    #[serde(default = "default_webp")]
    pub format: String,
    /// Quality 1-100
    #[validate(range(min = 1, max = 100))]
    #[serde(default = "default_quality")]
    pub quality: u8,
}

impl Default for ImageOptions {
    fn default() -> Self {
        Self {
            format: default_webp(),
            quality: default_quality(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoOptions {
    /// Output format: "hls", "mp4"
    #[serde(default = "default_hls")]
    pub format: String,
    /// Optional resolution like "1920x1080", "1280x720"
    pub resolution: Option<String>,
}

impl Default for VideoOptions {
    fn default() -> Self {
        Self {
            format: default_hls(),
            resolution: None,
        }
    }
}

// Default value functions for serde
fn default_true() -> bool {
    true
}

fn default_webp() -> String {
    "webp".to_string()
}

fn default_quality() -> u8 {
    85
}

fn default_hls() -> String {
    "hls".to_string()
}

// ============================================================================
// Status Response DTOs
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct ProcessingStatusResponse {
    pub document_id: String,
    pub status: DocumentStatus,
    pub processing_progress: Option<ProcessingProgress>,
    pub processing_metadata: Option<ProcessingMetadata>,
    pub error_message: Option<String>,
    pub processing_attempts: i32,
    pub last_processing_attempt: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProcessingProgress {
    pub current_processor: Option<String>,
    pub processors_completed: Vec<String>,
    pub processors_remaining: Vec<String>,
    pub percent_complete: u8,
}

impl From<Document> for ProcessingStatusResponse {
    fn from(doc: Document) -> Self {
        // Calculate progress if processing
        let processing_progress = if matches!(doc.status, DocumentStatus::Processing) {
            Some(ProcessingProgress {
                current_processor: None, // Will be enhanced in future
                processors_completed: vec![],
                processors_remaining: vec![],
                percent_complete: 0,
            })
        } else {
            None
        };

        Self {
            document_id: doc.id,
            status: doc.status,
            processing_progress,
            processing_metadata: doc.processing_metadata,
            error_message: doc.error_message,
            processing_attempts: doc.processing_attempts,
            last_processing_attempt: doc
                .last_processing_attempt
                .map(|dt| dt.try_to_rfc3339_string().unwrap_or_default()),
            created_at: doc.created_at.to_rfc3339(),
            updated_at: doc.updated_at.to_rfc3339(),
        }
    }
}
