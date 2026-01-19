use crate::models::{Channel, Notification, NotificationStatus};
use futures::TryStreamExt;
use mongodb::{
    bson::{doc, DateTime as BsonDateTime},
    options::IndexOptions,
    Client as MongoClient, Collection, Database, IndexModel,
};
use service_core::error::AppError;

#[derive(Clone)]
pub struct NotificationDb {
    client: MongoClient,
    db: Database,
}

impl NotificationDb {
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
        tracing::info!("Creating MongoDB indexes for notification-service");

        let notifications = self.notifications();

        // Index on status for querying by status
        let status_index = IndexModel::builder()
            .keys(doc! { "status": 1 })
            .options(
                IndexOptions::builder()
                    .name("status_idx".to_string())
                    .build(),
            )
            .build();

        notifications
            .create_index(status_index, None)
            .await
            .map_err(|e| {
                tracing::error!("Failed to create status index: {}", e);
                AppError::DatabaseError(anyhow::anyhow!(e.to_string()))
            })?;

        // Index on channel for filtering by channel type
        let channel_index = IndexModel::builder()
            .keys(doc! { "channel": 1 })
            .options(
                IndexOptions::builder()
                    .name("channel_idx".to_string())
                    .build(),
            )
            .build();

        notifications
            .create_index(channel_index, None)
            .await
            .map_err(|e| {
                tracing::error!("Failed to create channel index: {}", e);
                AppError::DatabaseError(anyhow::anyhow!(e.to_string()))
            })?;

        // Index on created_utc for time-based queries (descending for recent first)
        let created_index = IndexModel::builder()
            .keys(doc! { "created_utc": -1 })
            .options(
                IndexOptions::builder()
                    .name("created_utc_idx".to_string())
                    .build(),
            )
            .build();

        notifications
            .create_index(created_index, None)
            .await
            .map_err(|e| {
                tracing::error!("Failed to create created_utc index: {}", e);
                AppError::DatabaseError(anyhow::anyhow!(e.to_string()))
            })?;

        // Index on notification_id for quick lookups
        let notification_id_index = IndexModel::builder()
            .keys(doc! { "notification_id": 1 })
            .options(
                IndexOptions::builder()
                    .name("notification_id_idx".to_string())
                    .unique(true)
                    .build(),
            )
            .build();

        notifications
            .create_index(notification_id_index, None)
            .await
            .map_err(|e| {
                tracing::error!("Failed to create notification_id index: {}", e);
                AppError::DatabaseError(anyhow::anyhow!(e.to_string()))
            })?;

        // Index on metadata.user_id for user-specific queries
        let user_id_index = IndexModel::builder()
            .keys(doc! { "metadata.user_id": 1 })
            .options(
                IndexOptions::builder()
                    .name("metadata_user_id_idx".to_string())
                    .sparse(true)
                    .build(),
            )
            .build();

        notifications
            .create_index(user_id_index, None)
            .await
            .map_err(|e| {
                tracing::error!("Failed to create metadata.user_id index: {}", e);
                AppError::DatabaseError(anyhow::anyhow!(e.to_string()))
            })?;

        // Index on metadata.tenant_id for tenant-specific queries
        let tenant_id_index = IndexModel::builder()
            .keys(doc! { "metadata.tenant_id": 1 })
            .options(
                IndexOptions::builder()
                    .name("metadata_tenant_id_idx".to_string())
                    .sparse(true)
                    .build(),
            )
            .build();

        notifications
            .create_index(tenant_id_index, None)
            .await
            .map_err(|e| {
                tracing::error!("Failed to create metadata.tenant_id index: {}", e);
                AppError::DatabaseError(anyhow::anyhow!(e.to_string()))
            })?;

        tracing::info!("Successfully created all MongoDB indexes");
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

    pub fn notifications(&self) -> Collection<Notification> {
        self.db.collection("notifications")
    }

    pub async fn insert(&self, notification: &Notification) -> Result<(), AppError> {
        self.notifications()
            .insert_one(notification, None)
            .await
            .map_err(|e| {
                tracing::error!("Failed to insert notification: {}", e);
                AppError::DatabaseError(anyhow::anyhow!(e.to_string()))
            })?;
        Ok(())
    }

    pub async fn find_by_id(
        &self,
        notification_id: &str,
    ) -> Result<Option<Notification>, AppError> {
        self.notifications()
            .find_one(doc! { "notification_id": notification_id }, None)
            .await
            .map_err(|e| {
                tracing::error!("Failed to find notification: {}", e);
                AppError::DatabaseError(anyhow::anyhow!(e.to_string()))
            })
    }

    pub async fn update_status(
        &self,
        notification_id: &str,
        status: NotificationStatus,
        provider_id: Option<&str>,
        error_message: Option<&str>,
    ) -> Result<(), AppError> {
        let now = BsonDateTime::now();

        let mut update = doc! {
            "$set": {
                "status": status.to_string(),
            }
        };

        // Add timestamp based on status
        match status {
            NotificationStatus::Sent => {
                update
                    .get_document_mut("$set")
                    .unwrap()
                    .insert("sent_utc", now);
                if let Some(pid) = provider_id {
                    update
                        .get_document_mut("$set")
                        .unwrap()
                        .insert("provider_id", pid);
                }
            }
            NotificationStatus::Delivered => {
                update
                    .get_document_mut("$set")
                    .unwrap()
                    .insert("delivered_utc", now);
            }
            NotificationStatus::Failed => {
                update
                    .get_document_mut("$set")
                    .unwrap()
                    .insert("failed_utc", now);
                if let Some(err) = error_message {
                    update
                        .get_document_mut("$set")
                        .unwrap()
                        .insert("error_message", err);
                }
            }
            NotificationStatus::Queued => {}
        }

        self.notifications()
            .update_one(doc! { "notification_id": notification_id }, update, None)
            .await
            .map_err(|e| {
                tracing::error!("Failed to update notification status: {}", e);
                AppError::DatabaseError(anyhow::anyhow!(e.to_string()))
            })?;

        Ok(())
    }

    pub async fn list(
        &self,
        channel: Option<Channel>,
        status: Option<NotificationStatus>,
        limit: i64,
        offset: u64,
    ) -> Result<Vec<Notification>, AppError> {
        let mut filter = doc! {};

        if let Some(ch) = channel {
            filter.insert("channel", ch.to_string());
        }

        if let Some(st) = status {
            filter.insert("status", st.to_string());
        }

        let find_options = mongodb::options::FindOptions::builder()
            .sort(doc! { "created_utc": -1 })
            .limit(limit)
            .skip(offset)
            .build();

        let cursor = self
            .notifications()
            .find(filter, find_options)
            .await
            .map_err(|e| {
                tracing::error!("Failed to list notifications: {}", e);
                AppError::DatabaseError(anyhow::anyhow!(e.to_string()))
            })?;

        let notifications: Vec<Notification> = cursor.try_collect().await.map_err(|e| {
            tracing::error!("Failed to collect notifications: {}", e);
            AppError::DatabaseError(anyhow::anyhow!(e.to_string()))
        })?;

        Ok(notifications)
    }
}
