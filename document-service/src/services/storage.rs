use async_trait::async_trait;
use service_core::error::AppError;
use std::path::PathBuf;
use tokio::fs;

#[async_trait]
pub trait Storage: Send + Sync {
    async fn upload(&self, key: &str, data: Vec<u8>) -> Result<(), AppError>;
    async fn download(&self, key: &str) -> Result<Vec<u8>, AppError>;
    async fn delete(&self, key: &str) -> Result<(), AppError>;
}

pub struct LocalStorage {
    base_path: PathBuf,
}

impl LocalStorage {
    pub async fn new(base_path: impl Into<PathBuf>) -> Result<Self, AppError> {
        let base_path = base_path.into();
        if !base_path.exists() {
            fs::create_dir_all(&base_path).await.map_err(|e| {
                tracing::error!("Failed to create storage directory {:?}: {}", base_path, e);
                AppError::from(e)
            })?;
        }
        Ok(Self { base_path })
    }
}

#[async_trait]
impl Storage for LocalStorage {
    async fn upload(&self, key: &str, data: Vec<u8>) -> Result<(), AppError> {
        let path = self.base_path.join(key);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await.map_err(|e| {
                tracing::error!(
                    "Failed to create parent directory {:?} for file {}: {}",
                    parent,
                    key,
                    e
                );
                AppError::from(e)
            })?;
        }
        fs::write(&path, data).await.map_err(|e| {
            tracing::error!("Failed to write file {:?} for key {}: {}", path, key, e);
            AppError::from(e)
        })?;
        Ok(())
    }

    async fn download(&self, key: &str) -> Result<Vec<u8>, AppError> {
        let path = self.base_path.join(key);
        let data = fs::read(&path).await.map_err(|e| {
            tracing::error!("Failed to read file {:?} for key {}: {}", path, key, e);
            AppError::from(e)
        })?;
        Ok(data)
    }

    async fn delete(&self, key: &str) -> Result<(), AppError> {
        let path = self.base_path.join(key);
        if path.exists() {
            fs::remove_file(&path).await.map_err(|e| {
                tracing::error!("Failed to delete file {:?} for key {}: {}", path, key, e);
                AppError::from(e)
            })?;
        }
        Ok(())
    }
}
