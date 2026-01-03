use mongodb::{
    bson::doc,
    options::{ClientOptions, IndexOptions},
    Client, Collection, Database, IndexModel,
};
use std::time::Duration;

use crate::models::{RefreshToken, User, VerificationToken};

#[derive(Clone)]
pub struct MongoDb {
    client: Client,
    db: Database,
}

impl MongoDb {
    pub async fn connect(uri: &str, database: &str) -> Result<Self, anyhow::Error> {
        tracing::info!("Connecting to MongoDB at {}", uri);

        let mut client_options = ClientOptions::parse(uri).await?;

        // Configure connection pool
        client_options.max_pool_size = Some(10);
        client_options.min_pool_size = Some(2);
        client_options.connect_timeout = Some(Duration::from_secs(5));
        client_options.server_selection_timeout = Some(Duration::from_secs(5));

        let client = Client::with_options(client_options)?;

        // Test connection
        client
            .database("admin")
            .run_command(doc! { "ping": 1 }, None)
            .await?;

        let db = client.database(database);

        tracing::info!("Successfully connected to MongoDB database: {}", database);

        Ok(Self { client, db })
    }

    pub async fn initialize_indexes(&self) -> Result<(), anyhow::Error> {
        tracing::info!("Creating MongoDB indexes");

        // Users collection indexes
        let users = self.users();

        // Unique index on email
        let email_index = IndexModel::builder()
            .keys(doc! { "email": 1 })
            .options(
                IndexOptions::builder()
                    .unique(true)
                    .name("email_unique".to_string())
                    .build(),
            )
            .build();

        users.create_index(email_index, None).await?;
        tracing::info!("Created unique index on users.email");

        // Verification tokens collection indexes
        let tokens = self.verification_tokens();

        // Index on token for fast lookup
        let token_index = IndexModel::builder()
            .keys(doc! { "token": 1 })
            .options(
                IndexOptions::builder()
                    .name("token_lookup".to_string())
                    .build(),
            )
            .build();

        tokens.create_index(token_index, None).await?;

        // TTL index on expires_at for automatic cleanup
        let expiry_index = IndexModel::builder()
            .keys(doc! { "expires_at": 1 })
            .options(
                IndexOptions::builder()
                    .expire_after(Duration::from_secs(0))
                    .name("token_expiry_ttl".to_string())
                    .build(),
            )
            .build();

        tokens.create_index(expiry_index, None).await?;
        tracing::info!("Created indexes on verification_tokens collection");

        // Refresh tokens collection indexes
        let refresh_tokens = self.refresh_tokens();

        // Index on user_id for fast lookup of user's refresh tokens
        let user_id_index = IndexModel::builder()
            .keys(doc! { "user_id": 1 })
            .options(
                IndexOptions::builder()
                    .name("user_id_lookup".to_string())
                    .build(),
            )
            .build();

        refresh_tokens.create_index(user_id_index, None).await?;

        // Index on token_hash for fast lookup
        let refresh_token_index = IndexModel::builder()
            .keys(doc! { "token_hash": 1 })
            .options(
                IndexOptions::builder()
                    .name("refresh_token_hash_lookup".to_string())
                    .build(),
            )
            .build();

        refresh_tokens
            .create_index(refresh_token_index, None)
            .await?;

        // TTL index on expires_at for automatic cleanup
        let refresh_expiry_index = IndexModel::builder()
            .keys(doc! { "expires_at": 1 })
            .options(
                IndexOptions::builder()
                    .expire_after(Duration::from_secs(0))
                    .name("refresh_token_expiry_ttl".to_string())
                    .build(),
            )
            .build();

        refresh_tokens
            .create_index(refresh_expiry_index, None)
            .await?;
        tracing::info!("Created indexes on refresh_tokens collection");

        Ok(())
    }

    pub async fn health_check(&self) -> Result<(), anyhow::Error> {
        self.client
            .database("admin")
            .run_command(doc! { "ping": 1 }, None)
            .await?;
        Ok(())
    }

    pub fn users(&self) -> Collection<User> {
        self.db.collection("users")
    }

    pub fn verification_tokens(&self) -> Collection<VerificationToken> {
        self.db.collection("verification_tokens")
    }

    pub fn refresh_tokens(&self) -> Collection<RefreshToken> {
        self.db.collection("refresh_tokens")
    }

    pub fn client(&self) -> &Client {
        &self.client
    }

    pub fn database(&self) -> &Database {
        &self.db
    }
}
