//! Database operations for GenAI service.
//!
//! Handles session management and usage tracking via MongoDB.

use crate::models::{Session, UsageRecord};
use futures::TryStreamExt;
use mongodb::{
    bson::{doc, DateTime as BsonDateTime},
    options::IndexOptions,
    Client as MongoClient, Collection, Database, IndexModel,
};
use service_core::error::AppError;

#[derive(Clone)]
pub struct GenaiDb {
    client: MongoClient,
    db: Database,
}

impl GenaiDb {
    pub async fn connect(uri: &str, database: &str) -> Result<Self, AppError> {
        tracing::info!(uri = %uri, "Connecting to MongoDB");
        let client = MongoClient::with_uri_str(uri).await.map_err(|e| {
            tracing::error!("Failed to connect to MongoDB at {}: {}", uri, e);
            AppError::DatabaseError(anyhow::anyhow!(e.to_string()))
        })?;
        let db = client.database(database);
        tracing::info!(database = %database, "Successfully connected to MongoDB database");
        Ok(Self { client, db })
    }

    pub async fn initialize_indexes(&self) -> Result<(), AppError> {
        tracing::info!("Creating MongoDB indexes for genai-service");

        // Session indexes
        self.create_session_indexes().await?;

        // Usage indexes
        self.create_usage_indexes().await?;

        tracing::info!("Successfully created all MongoDB indexes");
        Ok(())
    }

    async fn create_session_indexes(&self) -> Result<(), AppError> {
        let sessions = self.sessions();

        // Unique index on session_id
        let session_id_index = IndexModel::builder()
            .keys(doc! { "session_id": 1 })
            .options(
                IndexOptions::builder()
                    .name("session_id_idx".to_string())
                    .unique(true)
                    .build(),
            )
            .build();

        sessions
            .create_index(session_id_index, None)
            .await
            .map_err(|e| {
                tracing::error!("Failed to create session_id index: {}", e);
                AppError::DatabaseError(anyhow::anyhow!(e.to_string()))
            })?;

        // Index on tenant_id for multi-tenant queries
        let tenant_id_index = IndexModel::builder()
            .keys(doc! { "tenant_id": 1 })
            .options(
                IndexOptions::builder()
                    .name("tenant_id_idx".to_string())
                    .build(),
            )
            .build();

        sessions
            .create_index(tenant_id_index, None)
            .await
            .map_err(|e| {
                tracing::error!("Failed to create tenant_id index: {}", e);
                AppError::DatabaseError(anyhow::anyhow!(e.to_string()))
            })?;

        // Index on user_id for user-specific queries
        let user_id_index = IndexModel::builder()
            .keys(doc! { "user_id": 1 })
            .options(
                IndexOptions::builder()
                    .name("user_id_idx".to_string())
                    .build(),
            )
            .build();

        sessions
            .create_index(user_id_index, None)
            .await
            .map_err(|e| {
                tracing::error!("Failed to create user_id index: {}", e);
                AppError::DatabaseError(anyhow::anyhow!(e.to_string()))
            })?;

        // Index on created_at for time-based cleanup
        let created_at_index = IndexModel::builder()
            .keys(doc! { "created_at": -1 })
            .options(
                IndexOptions::builder()
                    .name("created_at_idx".to_string())
                    .build(),
            )
            .build();

        sessions
            .create_index(created_at_index, None)
            .await
            .map_err(|e| {
                tracing::error!("Failed to create created_at index: {}", e);
                AppError::DatabaseError(anyhow::anyhow!(e.to_string()))
            })?;

        Ok(())
    }

    async fn create_usage_indexes(&self) -> Result<(), AppError> {
        let usage = self.usage();

        // Compound index for tenant + time range queries
        let tenant_time_index = IndexModel::builder()
            .keys(doc! { "tenant_id": 1, "timestamp": -1 })
            .options(
                IndexOptions::builder()
                    .name("tenant_time_idx".to_string())
                    .build(),
            )
            .build();

        usage
            .create_index(tenant_time_index, None)
            .await
            .map_err(|e| {
                tracing::error!("Failed to create tenant_time index: {}", e);
                AppError::DatabaseError(anyhow::anyhow!(e.to_string()))
            })?;

        // Index for user + time range queries
        let user_time_index = IndexModel::builder()
            .keys(doc! { "user_id": 1, "timestamp": -1 })
            .options(
                IndexOptions::builder()
                    .name("user_time_idx".to_string())
                    .build(),
            )
            .build();

        usage
            .create_index(user_time_index, None)
            .await
            .map_err(|e| {
                tracing::error!("Failed to create user_time index: {}", e);
                AppError::DatabaseError(anyhow::anyhow!(e.to_string()))
            })?;

        // Index on model for per-model aggregations
        let model_index = IndexModel::builder()
            .keys(doc! { "model": 1 })
            .options(
                IndexOptions::builder()
                    .name("model_idx".to_string())
                    .build(),
            )
            .build();

        usage.create_index(model_index, None).await.map_err(|e| {
            tracing::error!("Failed to create model index: {}", e);
            AppError::DatabaseError(anyhow::anyhow!(e.to_string()))
        })?;

        Ok(())
    }

    pub async fn health_check(&self) -> Result<(), AppError> {
        self.client
            .database("admin")
            .run_command(doc! { "ping": 1 }, None)
            .await
            .map_err(|e| {
                tracing::error!("MongoDB health check failed: {}", e);
                AppError::DatabaseError(anyhow::anyhow!(e.to_string()))
            })?;
        Ok(())
    }

    // Collection accessors

    pub fn sessions(&self) -> Collection<Session> {
        self.db.collection("sessions")
    }

    pub fn usage(&self) -> Collection<UsageRecord> {
        self.db.collection("usage")
    }

    // Session operations

    pub async fn insert_session(&self, session: &Session) -> Result<(), AppError> {
        self.sessions()
            .insert_one(session, None)
            .await
            .map_err(|e| {
                tracing::error!("Failed to insert session: {}", e);
                AppError::DatabaseError(anyhow::anyhow!(e.to_string()))
            })?;
        Ok(())
    }

    pub async fn find_session(&self, session_id: &str) -> Result<Option<Session>, AppError> {
        self.sessions()
            .find_one(doc! { "session_id": session_id }, None)
            .await
            .map_err(|e| {
                tracing::error!("Failed to find session: {}", e);
                AppError::DatabaseError(anyhow::anyhow!(e.to_string()))
            })
    }

    pub async fn update_session(
        &self,
        session_id: &str,
        message_count: i32,
        total_input_tokens: i32,
        total_output_tokens: i32,
    ) -> Result<(), AppError> {
        let now = BsonDateTime::now();

        self.sessions()
            .update_one(
                doc! { "session_id": session_id },
                doc! {
                    "$set": {
                        "message_count": message_count,
                        "total_input_tokens": total_input_tokens,
                        "total_output_tokens": total_output_tokens,
                        "updated_at": now
                    }
                },
                None,
            )
            .await
            .map_err(|e| {
                tracing::error!("Failed to update session: {}", e);
                AppError::DatabaseError(anyhow::anyhow!(e.to_string()))
            })?;

        Ok(())
    }

    pub async fn delete_session(&self, session_id: &str) -> Result<bool, AppError> {
        let result = self
            .sessions()
            .delete_one(doc! { "session_id": session_id }, None)
            .await
            .map_err(|e| {
                tracing::error!("Failed to delete session: {}", e);
                AppError::DatabaseError(anyhow::anyhow!(e.to_string()))
            })?;

        Ok(result.deleted_count > 0)
    }

    /// Add a message to an existing session and update usage.
    pub async fn add_session_message(
        &self,
        session_id: &str,
        message: &crate::models::SessionMessage,
        input_tokens: i32,
        output_tokens: i32,
    ) -> Result<(), AppError> {
        let now = BsonDateTime::now();

        // Convert message to BSON
        let message_doc = mongodb::bson::to_document(message).map_err(|e| {
            tracing::error!("Failed to serialize message: {}", e);
            AppError::DatabaseError(anyhow::anyhow!(e.to_string()))
        })?;

        self.sessions()
            .update_one(
                doc! { "session_id": session_id },
                doc! {
                    "$push": { "messages": message_doc },
                    "$inc": {
                        "message_count": 1,
                        "total_input_tokens": input_tokens,
                        "total_output_tokens": output_tokens
                    },
                    "$set": { "updated_at": now }
                },
                None,
            )
            .await
            .map_err(|e| {
                tracing::error!("Failed to add message to session: {}", e);
                AppError::DatabaseError(anyhow::anyhow!(e.to_string()))
            })?;

        Ok(())
    }

    // Usage operations

    pub async fn record_usage(&self, record: &UsageRecord) -> Result<(), AppError> {
        self.usage().insert_one(record, None).await.map_err(|e| {
            tracing::error!("Failed to record usage: {}", e);
            AppError::DatabaseError(anyhow::anyhow!(e.to_string()))
        })?;
        Ok(())
    }

    pub async fn get_usage(
        &self,
        tenant_id: Option<&str>,
        user_id: Option<&str>,
        start_time: chrono::DateTime<chrono::Utc>,
        end_time: chrono::DateTime<chrono::Utc>,
    ) -> Result<Vec<UsageRecord>, AppError> {
        let mut filter = doc! {
            "timestamp": {
                "$gte": BsonDateTime::from_millis(start_time.timestamp_millis()),
                "$lte": BsonDateTime::from_millis(end_time.timestamp_millis())
            }
        };

        if let Some(tid) = tenant_id {
            filter.insert("tenant_id", tid);
        }

        if let Some(uid) = user_id {
            filter.insert("user_id", uid);
        }

        let cursor = self.usage().find(filter, None).await.map_err(|e| {
            tracing::error!("Failed to query usage: {}", e);
            AppError::DatabaseError(anyhow::anyhow!(e.to_string()))
        })?;

        let records: Vec<UsageRecord> = cursor.try_collect().await.map_err(|e| {
            tracing::error!("Failed to collect usage records: {}", e);
            AppError::DatabaseError(anyhow::anyhow!(e.to_string()))
        })?;

        Ok(records)
    }
}
