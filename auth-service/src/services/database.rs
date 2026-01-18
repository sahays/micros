use mongodb::{
    bson::doc, options::IndexOptions, Client as MongoClient, Collection, Database, IndexModel,
};
use service_core::error::AppError;
use std::time::Duration;

use super::security_audit::SecurityAuditLog;
use crate::models::{
    AuditLog, Client, Organization, RefreshToken, ServiceAccount, User, VerificationToken,
};

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

        users.create_index(email_index, None).await.map_err(|e| {
            tracing::error!("Failed to create email index on users collection: {}", e);
            AppError::from(e)
        })?;
        tracing::info!("Created unique index on users.email");

        // Tenant-scoped compound index on (app_id, org_id, email)
        // This will become the primary unique constraint after migration
        // For now, it's non-unique to allow migration of existing users
        let tenant_email_index = IndexModel::builder()
            .keys(doc! { "app_id": 1, "org_id": 1, "email": 1 })
            .options(
                IndexOptions::builder()
                    .name("tenant_email_lookup".to_string())
                    .build(),
            )
            .build();

        users
            .create_index(tenant_email_index, None)
            .await
            .map_err(|e| {
                tracing::error!(
                    "Failed to create tenant_email index on users collection: {}",
                    e
                );
                AppError::from(e)
            })?;
        tracing::info!("Created index on users.(app_id, org_id, email)");

        // Index on (app_id, org_id) for listing users by tenant
        let tenant_index = IndexModel::builder()
            .keys(doc! { "app_id": 1, "org_id": 1 })
            .options(
                IndexOptions::builder()
                    .name("tenant_lookup".to_string())
                    .build(),
            )
            .build();

        users.create_index(tenant_index, None).await.map_err(|e| {
            tracing::error!("Failed to create tenant index on users collection: {}", e);
            AppError::from(e)
        })?;
        tracing::info!("Created index on users.(app_id, org_id)");

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

        tokens.create_index(token_index, None).await.map_err(|e| {
            tracing::error!(
                "Failed to create token index on verification_tokens collection: {}",
                e
            );
            AppError::from(e)
        })?;

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

        tokens.create_index(expiry_index, None).await.map_err(|e| {
            tracing::error!(
                "Failed to create TTL index on verification_tokens collection: {}",
                e
            );
            AppError::from(e)
        })?;
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

        refresh_tokens
            .create_index(user_id_index, None)
            .await
            .map_err(|e| {
                tracing::error!(
                    "Failed to create user_id index on refresh_tokens collection: {}",
                    e
                );
                AppError::from(e)
            })?;

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
            .await
            .map_err(|e| {
                tracing::error!(
                    "Failed to create token_hash index on refresh_tokens collection: {}",
                    e
                );
                AppError::from(e)
            })?;

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
            .await
            .map_err(|e| {
                tracing::error!(
                    "Failed to create TTL index on refresh_tokens collection: {}",
                    e
                );
                AppError::from(e)
            })?;
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

        clients
            .create_index(client_id_index, None)
            .await
            .map_err(|e| {
                tracing::error!(
                    "Failed to create client_id index on clients collection: {}",
                    e
                );
                AppError::from(e)
            })?;
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
            .await
            .map_err(|e| {
                tracing::error!(
                    "Failed to create service_id index on service_accounts collection: {}",
                    e
                );
                AppError::from(e)
            })?;
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
            .await
            .map_err(|e| {
                tracing::error!(
                    "Failed to create api_key_lookup_hash index on service_accounts collection: {}",
                    e
                );
                AppError::from(e)
            })?;
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
            .await
            .map_err(|e| {
                tracing::error!("Failed to create previous_api_key_lookup_hash index on service_accounts collection: {}", e);
                AppError::from(e)
            })?;
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

        audit_logs
            .create_index(service_id_index, None)
            .await
            .map_err(|e| {
                tracing::error!(
                    "Failed to create service_id index on audit_logs collection: {}",
                    e
                );
                AppError::from(e)
            })?;

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

        audit_logs
            .create_index(ttl_index, None)
            .await
            .map_err(|e| {
                tracing::error!("Failed to create TTL index on audit_logs collection: {}", e);
                AppError::from(e)
            })?;
        tracing::info!("Created indexes on audit_logs collection");

        // Organizations collection indexes
        let organizations = self.organizations();

        // Unique index on org_id
        let org_id_index = IndexModel::builder()
            .keys(doc! { "org_id": 1 })
            .options(
                IndexOptions::builder()
                    .unique(true)
                    .name("org_id_unique".to_string())
                    .build(),
            )
            .build();

        organizations
            .create_index(org_id_index, None)
            .await
            .map_err(|e| {
                tracing::error!(
                    "Failed to create org_id index on organizations collection: {}",
                    e
                );
                AppError::from(e)
            })?;

        // Unique compound index on (app_id, name) - org names unique within an app
        let app_name_index = IndexModel::builder()
            .keys(doc! { "app_id": 1, "name": 1 })
            .options(
                IndexOptions::builder()
                    .unique(true)
                    .name("app_id_name_unique".to_string())
                    .build(),
            )
            .build();

        organizations
            .create_index(app_name_index, None)
            .await
            .map_err(|e| {
                tracing::error!(
                    "Failed to create app_id_name index on organizations collection: {}",
                    e
                );
                AppError::from(e)
            })?;

        // Index on app_id for listing orgs by app
        let app_id_index = IndexModel::builder()
            .keys(doc! { "app_id": 1 })
            .options(
                IndexOptions::builder()
                    .name("app_id_lookup".to_string())
                    .build(),
            )
            .build();

        organizations
            .create_index(app_id_index, None)
            .await
            .map_err(|e| {
                tracing::error!(
                    "Failed to create app_id index on organizations collection: {}",
                    e
                );
                AppError::from(e)
            })?;
        tracing::info!("Created indexes on organizations collection");

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

    pub fn organizations(&self) -> Collection<Organization> {
        self.db.collection("organizations")
    }

    pub fn security_audit_logs(&self) -> Collection<SecurityAuditLog> {
        self.db.collection("security_audit_logs")
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
    ServiceAccount, "service_accounts", find_service_account_by_id, "service_id", str;
    Organization, "organizations", find_organization_by_id, "org_id", str
);

// Tenant-scoped query methods
impl MongoDb {
    /// Find a user by email within a specific tenant (app_id + org_id).
    pub async fn find_user_by_email_in_tenant(
        &self,
        app_id: &str,
        org_id: &str,
        email: &str,
    ) -> Result<Option<User>, mongodb::error::Error> {
        self.users()
            .find_one(
                doc! {
                    "app_id": app_id,
                    "org_id": org_id,
                    "email": email
                },
                None,
            )
            .await
    }

    /// Find a user by ID within a specific tenant.
    pub async fn find_user_by_id_in_tenant(
        &self,
        app_id: &str,
        org_id: &str,
        user_id: &str,
    ) -> Result<Option<User>, mongodb::error::Error> {
        self.users()
            .find_one(
                doc! {
                    "app_id": app_id,
                    "org_id": org_id,
                    "_id": user_id
                },
                None,
            )
            .await
    }

    /// Find all organizations for an app.
    pub async fn find_organizations_by_app(
        &self,
        app_id: &str,
    ) -> Result<Vec<Organization>, mongodb::error::Error> {
        use futures::TryStreamExt;
        self.organizations()
            .find(doc! { "app_id": app_id, "enabled": true }, None)
            .await?
            .try_collect()
            .await
    }

    /// Find organization by app_id and org_id.
    pub async fn find_organization_in_app(
        &self,
        app_id: &str,
        org_id: &str,
    ) -> Result<Option<Organization>, mongodb::error::Error> {
        self.organizations()
            .find_one(
                doc! {
                    "app_id": app_id,
                    "org_id": org_id
                },
                None,
            )
            .await
    }
}
