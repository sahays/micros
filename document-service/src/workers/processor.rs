use crate::dtos::ProcessingOptions;
use crate::models::{Document, ProcessingMetadata};
use crate::workers::executor::CommandExecutor;
use async_trait::async_trait;
use service_core::error::AppError;
use std::path::Path;

#[async_trait]
pub trait Processor: Send + Sync {
    fn supported_mime_types(&self) -> Vec<&str>;

    async fn process(
        &self,
        document: &Document,
        file_path: &Path,
        executor: &CommandExecutor,
        options: &ProcessingOptions,
    ) -> Result<ProcessingMetadata, AppError>;
}

pub struct ProcessorRegistry {
    processors: Vec<Box<dyn Processor>>,
}

impl Default for ProcessorRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ProcessorRegistry {
    pub fn new() -> Self {
        use crate::workers::{ImageProcessor, PdfProcessor, VideoProcessor};

        Self {
            processors: vec![
                Box::new(PdfProcessor::new()),
                Box::new(ImageProcessor::new()),
                Box::new(VideoProcessor::new()),
            ],
        }
    }

    pub fn find_processor(&self, mime_type: &str) -> Option<&dyn Processor> {
        self.processors
            .iter()
            .find(|p| p.supported_mime_types().contains(&mime_type))
            .map(|b| b.as_ref())
    }
}
