use async_trait::async_trait;
use service_core::error::AppError;
use std::path::PathBuf;
use tokio::fs;
use aws_sdk_s3::Client as S3Client;
use aws_sdk_s3::primitives::ByteStream;

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
            fs::create_dir_all(&base_path).await?;
        }
        Ok(Self { base_path })
    }
}

#[async_trait]
impl Storage for LocalStorage {
    async fn upload(&self, key: &str, data: Vec<u8>) -> Result<(), AppError> {
        let path = self.base_path.join(key);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await?;
        }
        fs::write(path, data).await?;
        Ok(())
    }

    async fn download(&self, key: &str) -> Result<Vec<u8>, AppError> {
        let path = self.base_path.join(key);
        let data = fs::read(path).await?;
        Ok(data)
    }

    async fn delete(&self, key: &str) -> Result<(), AppError> {
        let path = self.base_path.join(key);
        if path.exists() {
            fs::remove_file(path).await?;
        }
        Ok(())
    }
}

pub struct S3Storage {
    client: S3Client,
    bucket: String,
}

impl S3Storage {
    pub fn new(client: S3Client, bucket: String) -> Self {
        Self { client, bucket }
    }
}

#[async_trait]
impl Storage for S3Storage {
    async fn upload(&self, key: &str, data: Vec<u8>) -> Result<(), AppError> {
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .body(ByteStream::from(data))
            .send()
            .await
            .map_err(|e| AppError::InternalError(anyhow::anyhow!("S3 upload failed: {}", e)))?;
        Ok(())
    }

    async fn download(&self, key: &str) -> Result<Vec<u8>, AppError> {
        let output = self.client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| AppError::InternalError(anyhow::anyhow!("S3 download failed: {}", e)))?;

        let data = output.body.collect().await
            .map_err(|e| AppError::InternalError(anyhow::anyhow!("S3 body collection failed: {}", e)))?
            .into_bytes()
            .to_vec();
        
        Ok(data)
    }

    async fn delete(&self, key: &str) -> Result<(), AppError> {
        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| AppError::InternalError(anyhow::anyhow!("S3 delete failed: {}", e)))?;
        Ok(())
    }
}
