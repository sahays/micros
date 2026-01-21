//! Database operations for GenAI service.
//!
//! Handles session management and usage tracking via MongoDB.

use crate::models::{Session, UsageRecord};
use crate::services::metrics::{record_db_error, record_db_operation};
use futures::TryStreamExt;
use mongodb::{
    bson::{doc, DateTime as BsonDateTime},
    options::IndexOptions,
    Client as MongoClient, Collection, Database, IndexModel,
};
use service_core::error::AppError;
use std::time::Instant;

#[derive(Clone)]
pub struct GenaiDb {
    client: MongoClient,
    db: Database,
}

impl GenaiDb {
    #[tracing::instrument(skip_all, fields(database = %database))]
    pub async fn connect(uri: &str, database: &str) -> Result<Self, AppError> {
        tracing::info!("Connecting to MongoDB");
        let start = Instant::now();

        let client = MongoClient::with_uri_str(uri).await.map_err(|e| {
            tracing::error!(error = %e, "Failed to connect to MongoDB");
            record_db_error("connect", "admin");
            AppError::DatabaseError(anyhow::anyhow!(e.to_string()))
        })?;

        let db = client.database(database);
        let duration = start.elapsed();

        record_db_operation("connect", "admin", duration.as_secs_f64());
        tracing::info!(
            duration_ms = duration.as_millis(),
            "Successfully connected to MongoDB"
        );

        Ok(Self { client, db })
    }

    #[tracing::instrument(skip(self))]
    pub async fn initialize_indexes(&self) -> Result<(), AppError> {
        tracing::info!("Creating MongoDB indexes for genai-service");
        let start = Instant::now();

        // Session indexes
        self.create_session_indexes().await?;

        // Usage indexes
        self.create_usage_indexes().await?;

        let duration = start.elapsed();
        tracing::info!(
            duration_ms = duration.as_millis(),
            "Successfully created all MongoDB indexes"
        );

        Ok(())
    }

    #[tracing::instrument(skip(self))]
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
                tracing::error!(error = %e, index = "session_id_idx", "Failed to create index");
                record_db_error("create_index", "sessions");
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
                tracing::error!(error = %e, index = "tenant_id_idx", "Failed to create index");
                record_db_error("create_index", "sessions");
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
                tracing::error!(error = %e, index = "user_id_idx", "Failed to create index");
                record_db_error("create_index", "sessions");
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
                tracing::error!(error = %e, index = "created_at_idx", "Failed to create index");
                record_db_error("create_index", "sessions");
                AppError::DatabaseError(anyhow::anyhow!(e.to_string()))
            })?;

        Ok(())
    }

    #[tracing::instrument(skip(self))]
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
                tracing::error!(error = %e, index = "tenant_time_idx", "Failed to create index");
                record_db_error("create_index", "usage");
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
                tracing::error!(error = %e, index = "user_time_idx", "Failed to create index");
                record_db_error("create_index", "usage");
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
            tracing::error!(error = %e, index = "model_idx", "Failed to create index");
            record_db_error("create_index", "usage");
            AppError::DatabaseError(anyhow::anyhow!(e.to_string()))
        })?;

        Ok(())
    }

    #[tracing::instrument(skip(self))]
    pub async fn health_check(&self) -> Result<(), AppError> {
        let start = Instant::now();

        let result = self
            .client
            .database("admin")
            .run_command(doc! { "ping": 1 }, None)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "MongoDB health check failed");
                record_db_error("ping", "admin");
                AppError::DatabaseError(anyhow::anyhow!(e.to_string()))
            });

        let duration = start.elapsed();
        record_db_operation("ping", "admin", duration.as_secs_f64());

        result.map(|_| ())
    }

    // Collection accessors

    pub fn sessions(&self) -> Collection<Session> {
        self.db.collection("sessions")
    }

    pub fn usage(&self) -> Collection<UsageRecord> {
        self.db.collection("usage")
    }

    // Session operations

    #[tracing::instrument(skip(self, session), fields(session_id = %session.session_id))]
    pub async fn insert_session(&self, session: &Session) -> Result<(), AppError> {
        let start = Instant::now();

        let result = self
            .sessions()
            .insert_one(session, None)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to insert session");
                record_db_error("insert", "sessions");
                AppError::DatabaseError(anyhow::anyhow!(e.to_string()))
            });

        let duration = start.elapsed();
        record_db_operation("insert", "sessions", duration.as_secs_f64());

        tracing::debug!(duration_ms = duration.as_millis(), "Session inserted");
        result.map(|_| ())
    }

    #[tracing::instrument(skip(self), fields(session_id = %session_id))]
    pub async fn find_session(&self, session_id: &str) -> Result<Option<Session>, AppError> {
        let start = Instant::now();

        let result = self
            .sessions()
            .find_one(doc! { "session_id": session_id }, None)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to find session");
                record_db_error("find_one", "sessions");
                AppError::DatabaseError(anyhow::anyhow!(e.to_string()))
            });

        let duration = start.elapsed();
        record_db_operation("find_one", "sessions", duration.as_secs_f64());

        if let Ok(ref session) = result {
            tracing::debug!(
                duration_ms = duration.as_millis(),
                found = session.is_some(),
                "Session lookup completed"
            );
        }

        result
    }

    #[tracing::instrument(skip(self), fields(session_id = %session_id))]
    pub async fn update_session(
        &self,
        session_id: &str,
        message_count: i32,
        total_input_tokens: i32,
        total_output_tokens: i32,
    ) -> Result<(), AppError> {
        let start = Instant::now();
        let now = BsonDateTime::now();

        let result = self
            .sessions()
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
                tracing::error!(error = %e, "Failed to update session");
                record_db_error("update_one", "sessions");
                AppError::DatabaseError(anyhow::anyhow!(e.to_string()))
            });

        let duration = start.elapsed();
        record_db_operation("update_one", "sessions", duration.as_secs_f64());

        tracing::debug!(
            duration_ms = duration.as_millis(),
            message_count = message_count,
            "Session updated"
        );

        result.map(|_| ())
    }

    #[tracing::instrument(skip(self), fields(session_id = %session_id))]
    pub async fn delete_session(&self, session_id: &str) -> Result<bool, AppError> {
        let start = Instant::now();

        let result = self
            .sessions()
            .delete_one(doc! { "session_id": session_id }, None)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to delete session");
                record_db_error("delete_one", "sessions");
                AppError::DatabaseError(anyhow::anyhow!(e.to_string()))
            })?;

        let duration = start.elapsed();
        record_db_operation("delete_one", "sessions", duration.as_secs_f64());

        let deleted = result.deleted_count > 0;
        tracing::debug!(
            duration_ms = duration.as_millis(),
            deleted = deleted,
            "Session delete completed"
        );

        Ok(deleted)
    }

    /// Add a message to an existing session and update usage.
    #[tracing::instrument(skip(self, message), fields(session_id = %session_id, input_tokens, output_tokens))]
    pub async fn add_session_message(
        &self,
        session_id: &str,
        message: &crate::models::SessionMessage,
        input_tokens: i32,
        output_tokens: i32,
    ) -> Result<(), AppError> {
        let start = Instant::now();
        let now = BsonDateTime::now();

        // Convert message to BSON
        let message_doc = mongodb::bson::to_document(message).map_err(|e| {
            tracing::error!(error = %e, "Failed to serialize message");
            AppError::DatabaseError(anyhow::anyhow!(e.to_string()))
        })?;

        let result = self
            .sessions()
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
                tracing::error!(error = %e, "Failed to add message to session");
                record_db_error("update_one", "sessions");
                AppError::DatabaseError(anyhow::anyhow!(e.to_string()))
            });

        let duration = start.elapsed();
        record_db_operation("update_one", "sessions", duration.as_secs_f64());

        tracing::debug!(
            duration_ms = duration.as_millis(),
            role = %message.role,
            "Message added to session"
        );

        result.map(|_| ())
    }

    // Usage operations

    #[tracing::instrument(skip(self, record), fields(
        request_id = %record.request_id,
        tenant_id = %record.tenant_id,
        model = %record.model
    ))]
    pub async fn record_usage(&self, record: &UsageRecord) -> Result<(), AppError> {
        let start = Instant::now();

        let result = self.usage().insert_one(record, None).await.map_err(|e| {
            tracing::error!(error = %e, "Failed to record usage");
            record_db_error("insert", "usage");
            AppError::DatabaseError(anyhow::anyhow!(e.to_string()))
        });

        let duration = start.elapsed();
        record_db_operation("insert", "usage", duration.as_secs_f64());

        tracing::debug!(
            duration_ms = duration.as_millis(),
            input_tokens = record.input_tokens,
            output_tokens = record.output_tokens,
            "Usage recorded"
        );

        result.map(|_| ())
    }

    #[tracing::instrument(skip(self), fields(tenant_id = ?tenant_id, user_id = ?user_id))]
    pub async fn get_usage(
        &self,
        tenant_id: Option<&str>,
        user_id: Option<&str>,
        start_time: chrono::DateTime<chrono::Utc>,
        end_time: chrono::DateTime<chrono::Utc>,
    ) -> Result<Vec<UsageRecord>, AppError> {
        let start = Instant::now();

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
            tracing::error!(error = %e, "Failed to query usage");
            record_db_error("find", "usage");
            AppError::DatabaseError(anyhow::anyhow!(e.to_string()))
        })?;

        let records: Vec<UsageRecord> = cursor.try_collect().await.map_err(|e| {
            tracing::error!(error = %e, "Failed to collect usage records");
            record_db_error("cursor_collect", "usage");
            AppError::DatabaseError(anyhow::anyhow!(e.to_string()))
        })?;

        let duration = start.elapsed();
        record_db_operation("find", "usage", duration.as_secs_f64());

        tracing::debug!(
            duration_ms = duration.as_millis(),
            record_count = records.len(),
            "Usage query completed"
        );

        Ok(records)
    }
}
