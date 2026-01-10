use crate::models::Document;
use mongodb::{
    bson::doc, options::IndexOptions, Client as MongoClient, Collection, Database, IndexModel,
};
use service_core::error::AppError;

#[derive(Clone)]
pub struct MongoDb {
    client: MongoClient,
    db: Database,
}

impl MongoDb {
    pub async fn connect(uri: &str, database: &str) -> Result<Self, AppError> {
        tracing::info!(uri = %uri, "Connecting to MongoDB");
        let client = MongoClient::with_uri_str(uri).await.map_err(|e| {
            tracing::error!("Failed to connect to MongoDB at {}: {}", uri, e);
            AppError::from(e)
        })?;
        let db = client.database(database);
        tracing::info!(database = %database, "Successfully connected to MongoDB database");
        Ok(Self { client, db })
    }

    pub async fn initialize_indexes(&self) -> Result<(), AppError> {
        tracing::info!("Creating MongoDB indexes for document-service");

        let documents = self.documents();

        // Index on owner_id for fast lookup
        let owner_id_index = IndexModel::builder()
            .keys(doc! { "owner_id": 1 })
            .options(
                IndexOptions::builder()
                    .name("owner_id_lookup".to_string())
                    .build(),
            )
            .build();

        documents
            .create_index(owner_id_index, None)
            .await
            .map_err(|e| {
                tracing::error!(
                    "Failed to create owner_id index on documents collection: {}",
                    e
                );
                AppError::from(e)
            })?;
        tracing::info!("Created index on documents.owner_id");

        Ok(())
    }

    pub async fn health_check(&self) -> Result<(), AppError> {
        self.client
            .database("admin")
            .run_command(doc! { "ping": 1 }, None)
            .await
            .map_err(|e| {
                tracing::error!("MongoDB health check failed: {}", e);
                AppError::from(e)
            })?;
        Ok(())
    }

    pub fn documents(&self) -> Collection<Document> {
        self.db.collection("documents")
    }

    pub fn client(&self) -> &MongoClient {
        &self.client
    }

    pub fn database(&self) -> &Database {
        &self.db
    }
}
