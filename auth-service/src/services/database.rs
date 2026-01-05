use mongodb::{
    bson::doc, options::IndexOptions, Client as MongoClient, Collection, Database, IndexModel,
};
use std::time::Duration;

use crate::models::{AuditLog, Client, RefreshToken, ServiceAccount, User, VerificationToken};

#[derive(Clone)]
pub struct MongoDb {
    client: MongoClient,
    db: Database,
}

impl MongoDb {
    pub async fn connect(uri: &str, database: &str) -> Result<Self, anyhow::Error> {
        tracing::info!(uri = %uri, "Connecting to MongoDB");
        let client = MongoClient::with_uri_str(uri).await?;
        let db = client.database(database);
        tracing::info!(database = %database, "Successfully connected to MongoDB database");
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

        // Clients collection indexes
        let clients = self.clients();

        // Unique index on client_id
        let client_id_index = IndexModel::builder()
            .keys(doc! { "client_id": 1 })
            .options(
                IndexOptions::builder()
                    .unique(true)
                    .name("client_id_unique".to_string())
                    .build(),
            )
            .build();

        clients.create_index(client_id_index, None).await?;
        tracing::info!("Created unique index on clients.client_id");

        // Service accounts collection indexes
        let service_accounts = self.service_accounts();

        // Unique index on service_id
        let service_id_index = IndexModel::builder()
            .keys(doc! { "service_id": 1 })
            .options(
                IndexOptions::builder()
                    .unique(true)
                    .name("service_id_unique".to_string())
                    .build(),
            )
            .build();

        service_accounts
            .create_index(service_id_index, None)
            .await?;
        tracing::info!("Created unique index on service_accounts.service_id");

        // Unique index on api_key_lookup_hash
        let api_key_lookup_index = IndexModel::builder()
            .keys(doc! { "api_key_lookup_hash": 1 })
            .options(
                IndexOptions::builder()
                    .unique(true)
                    .name("api_key_lookup_hash_unique".to_string())
                    .build(),
            )
            .build();

        service_accounts
            .create_index(api_key_lookup_index, None)
            .await?;
        tracing::info!("Created unique index on service_accounts.api_key_lookup_hash");

        // Sparse unique index on previous_api_key_lookup_hash
        let prev_api_key_lookup_index = IndexModel::builder()
            .keys(doc! { "previous_api_key_lookup_hash": 1 })
            .options(
                IndexOptions::builder()
                    .unique(true)
                    .sparse(true)
                    .name("prev_api_key_lookup_hash_unique".to_string())
                    .build(),
            )
            .build();

        service_accounts
            .create_index(prev_api_key_lookup_index, None)
            .await?;
        tracing::info!(
            "Created sparse unique index on service_accounts.previous_api_key_lookup_hash"
        );

        // Audit logs collection indexes
        let audit_logs = self.audit_logs();

        // Index on service_id for lookup
        let service_id_index = IndexModel::builder()
            .keys(doc! { "service_id": 1 })
            .options(
                IndexOptions::builder()
                    .name("service_id_audit_lookup".to_string())
                    .build(),
            )
            .build();

        audit_logs.create_index(service_id_index, None).await?;

        // TTL index on timestamp for 30 days retention
        let ttl_index = IndexModel::builder()
            .keys(doc! { "timestamp": 1 })
            .options(
                IndexOptions::builder()
                    .expire_after(Duration::from_secs(30 * 24 * 60 * 60))
                    .name("audit_log_ttl".to_string())
                    .build(),
            )
            .build();

        audit_logs.create_index(ttl_index, None).await?;
        tracing::info!("Created indexes on audit_logs collection");

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

    pub fn clients(&self) -> Collection<Client> {
        self.db.collection("clients")
    }

    pub fn service_accounts(&self) -> Collection<ServiceAccount> {
        self.db.collection("service_accounts")
    }

    pub fn audit_logs(&self) -> Collection<AuditLog> {
        self.db.collection("audit_logs")
    }

    pub fn client(&self) -> &MongoClient {
        &self.client
    }

    pub fn database(&self) -> &Database {
        &self.db
    }
}

/// Macro to generate type-safe find_by_* methods for MongoDB collections.
///
/// Usage:
/// ```rust,ignore
/// impl_find_by!(
///     User, "users", find_user_by_email, "email", String;
///     VerificationToken, "verification_tokens", find_token_by_token, "token", String
/// );
/// ```
macro_rules! impl_find_by {
    ($($model:ty, $collection_name:expr, $method_name:ident, $field_name:expr, $field_type:ty);+ $(;)?) => {
        impl MongoDb {
            $(
                pub async fn $method_name(&self, value: &$field_type) -> Result<Option<$model>, mongodb::error::Error> {
                    self.db.collection::<$model>($collection_name)
                        .find_one(doc! { $field_name: value }, None)
                        .await
                }
            )+
        }
    };
}

impl_find_by!(
    User, "users", find_user_by_email, "email", str;
    User, "users", find_user_by_id, "_id", str;
    VerificationToken, "verification_tokens", find_token_by_token, "token", str;
    RefreshToken, "refresh_tokens", find_refresh_token_by_id, "_id", str;
    Client, "clients", find_client_by_id, "client_id", str;
    ServiceAccount, "service_accounts", find_service_account_by_id, "service_id", str
);
