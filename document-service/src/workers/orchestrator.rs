use crate::config::WorkerConfig;
use crate::models::{Document, ProcessingMetadata};
use crate::services::database::MongoDb;
use crate::services::storage::Storage;
use crate::workers::executor::CommandExecutor;
use crate::workers::processor::ProcessorRegistry;
use backoff::future::retry;
use backoff::ExponentialBackoff;
use chrono::Utc;
use mongodb::bson::doc;
use service_core::error::AppError;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct ProcessingJob {
    pub document_id: String,
    pub app_id: String,
    pub org_id: String,
    pub owner_id: String,
    pub mime_type: String,
    pub storage_key: String,
}

pub struct WorkerOrchestrator {
    config: WorkerConfig,
    db: MongoDb,
    storage: Arc<dyn Storage>,
    registry: Arc<ProcessorRegistry>,
    job_tx: mpsc::Sender<ProcessingJob>,
    job_rx: Option<mpsc::Receiver<ProcessingJob>>,
    shutdown_token: CancellationToken,
}

impl WorkerOrchestrator {
    pub fn new(
        config: WorkerConfig,
        db: MongoDb,
        storage: Arc<dyn Storage>,
    ) -> (Self, mpsc::Sender<ProcessingJob>) {
        let (job_tx, job_rx) = mpsc::channel(config.queue_size);
        let shutdown_token = CancellationToken::new();
        let registry = Arc::new(ProcessorRegistry::new());

        let orchestrator = Self {
            config,
            db,
            storage,
            registry,
            job_tx: job_tx.clone(),
            job_rx: Some(job_rx),
            shutdown_token,
        };

        (orchestrator, job_tx)
    }

    pub async fn start(mut self) {
        if !self.config.enabled {
            tracing::info!("Worker pool disabled by configuration");
            return;
        }

        let mut job_rx = self.job_rx.take().expect("start() can only be called once");

        tracing::info!(
            worker_count = self.config.worker_count,
            "Starting worker pool"
        );

        // Create workers
        let mut workers = Vec::new();
        for worker_id in 0..self.config.worker_count {
            workers.push(Worker {
                id: worker_id,
                db: self.db.clone(),
                storage: self.storage.clone(),
                registry: self.registry.clone(),
                executor: CommandExecutor::new(self.config.command_timeout()),
                temp_dir: self.config.temp_dir.clone(),
            });
        }

        let shutdown = self.shutdown_token.clone();

        // Spawn a single task to distribute jobs to workers
        tokio::spawn(async move {
            let mut next_worker = 0;

            loop {
                tokio::select! {
                    _ = shutdown.cancelled() => {
                        tracing::info!("Job distributor shutting down");
                        break;
                    }
                    job = job_rx.recv() => {
                        match job {
                            Some(job) => {
                                // Round-robin distribution
                                let worker = &workers[next_worker];
                                next_worker = (next_worker + 1) % workers.len();

                                tracing::info!(
                                    worker_id = worker.id,
                                    document_id = %job.document_id,
                                    "Dispatching job to worker"
                                );

                                // Clone worker and spawn processing task
                                let worker_clone = worker.clone();
                                tokio::spawn(async move {
                                    worker_clone.process_job(job).await;
                                });
                            }
                            None => {
                                tracing::info!("Channel closed, job distributor exiting");
                                break;
                            }
                        }
                    }
                }
            }
        });
    }

    pub fn enqueue(&self, job: ProcessingJob) -> Result<(), AppError> {
        self.job_tx
            .try_send(job)
            .map_err(|_| AppError::InternalError(anyhow::anyhow!("Job queue full")))
    }

    pub async fn shutdown(&self) {
        tracing::info!("Initiating worker pool shutdown");
        self.shutdown_token.cancel();
    }
}

#[derive(Clone)]
struct Worker {
    id: usize,
    db: MongoDb,
    storage: Arc<dyn Storage>,
    registry: Arc<ProcessorRegistry>,
    executor: CommandExecutor,
    temp_dir: PathBuf,
}

impl Worker {
    async fn process_job(&self, job: ProcessingJob) {
        let document_id = job.document_id.clone();
        let start = Instant::now();

        tracing::info!(
            worker_id = self.id,
            document_id = %document_id,
            mime_type = %job.mime_type,
            "Processing job started"
        );

        // Retry logic with exponential backoff
        let backoff = ExponentialBackoff {
            max_elapsed_time: Some(Duration::from_secs(300)),
            ..Default::default()
        };

        let result = retry(backoff, || async {
            self.process_with_retry(&job)
                .await
                .map_err(backoff::Error::transient)
        })
        .await;

        match result {
            Ok(metadata) => {
                self.update_document_success(&document_id, metadata).await;

                tracing::info!(
                    worker_id = self.id,
                    document_id = %document_id,
                    duration_ms = start.elapsed().as_millis(),
                    "Processing succeeded"
                );
            }
            Err(e) => {
                self.update_document_failure(&document_id, e.to_string())
                    .await;

                tracing::error!(
                    worker_id = self.id,
                    document_id = %document_id,
                    error = %e,
                    "Processing failed after retries"
                );
            }
        }
    }

    async fn process_with_retry(
        &self,
        job: &ProcessingJob,
    ) -> Result<ProcessingMetadata, AppError> {
        // 1. Download file from storage
        let temp_file = self
            .temp_dir
            .join(format!("{}_{}", job.document_id, Uuid::new_v4()));

        let data = self
            .storage
            .download(&job.storage_key)
            .await
            .map_err(|e| {
                tracing::error!(
                    document_id = %job.document_id,
                    storage_key = %job.storage_key,
                    error = %e,
                    "Failed to download file from storage"
                );
                e
            })?;

        tokio::fs::write(&temp_file, data).await.map_err(|e| {
            AppError::InternalError(anyhow::anyhow!("Failed to write temp file: {}", e))
        })?;

        tracing::debug!(
            document_id = %job.document_id,
            temp_file = ?temp_file,
            "Downloaded file to temp location"
        );

        // 2. Find processor
        let processor = self
            .registry
            .find_processor(&job.mime_type)
            .ok_or_else(|| {
                AppError::BadRequest(anyhow::anyhow!("Unsupported file type: {}", job.mime_type))
            })?;

        // 3. Process document
        // We create a placeholder document just for the processor interface
        let document = Document::new(
            job.app_id.clone(),
            job.org_id.clone(),
            job.owner_id.clone(),
            "temp".to_string(),
            job.mime_type.clone(),
            0,
            job.storage_key.clone(),
        );

        let metadata = processor
            .process(&document, &temp_file, &self.executor)
            .await
            .map_err(|e| {
                tracing::error!(
                    document_id = %job.document_id,
                    mime_type = %job.mime_type,
                    error = %e,
                    "Document processing failed"
                );
                e
            })?;

        // 4. Cleanup temp file
        if let Err(e) = tokio::fs::remove_file(&temp_file).await {
            tracing::warn!(
                document_id = %job.document_id,
                temp_file = ?temp_file,
                error = %e,
                "Failed to remove temp file after processing"
            );
        }

        Ok(metadata)
    }

    async fn update_document_success(&self, doc_id: &str, metadata: ProcessingMetadata) {
        let bson_metadata = match mongodb::bson::to_bson(&metadata) {
            Ok(b) => b,
            Err(e) => {
                tracing::error!(
                    document_id = doc_id,
                    error = %e,
                    "Failed to serialize processing metadata to BSON, marking document as failed"
                );
                self.update_document_failure(
                    doc_id,
                    format!("BSON serialization error: {}", e),
                )
                .await;
                return;
            }
        };

        let update = doc! {
            "$set": {
                "status": "ready",
                "processing_metadata": bson_metadata,
                "updated_at": mongodb::bson::DateTime::from_chrono(Utc::now()),
            }
        };

        if let Err(e) = self
            .db
            .documents()
            .update_one(doc! { "_id": doc_id }, update, None)
            .await
        {
            tracing::error!(
                document_id = doc_id,
                error = %e,
                "Failed to update document with success status"
            );
        } else {
            tracing::info!(document_id = doc_id, "Document updated with success status");
        }
    }

    async fn update_document_failure(&self, doc_id: &str, error: String) {
        let update = doc! {
            "$set": {
                "status": "failed",
                "error_message": error,
                "updated_at": mongodb::bson::DateTime::from_chrono(Utc::now()),
            }
        };

        if let Err(e) = self
            .db
            .documents()
            .update_one(doc! { "_id": doc_id }, update, None)
            .await
        {
            tracing::error!(
                document_id = doc_id,
                error = %e,
                "Failed to update document with failure status"
            );
        } else {
            tracing::info!(document_id = doc_id, "Document updated with failure status");
        }
    }
}
