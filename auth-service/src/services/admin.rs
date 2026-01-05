use crate::{
    dtos::admin::{
        CreateClientRequest, CreateClientResponse, CreateServiceAccountRequest,
        CreateServiceAccountResponse, RotateSecretResponse, RotateServiceKeyResponse,
    },
    models::{AuditLog, Client, ServiceAccount},
    services::{MongoDb, ServiceError, TokenBlacklist},
    utils::{hash_password, Password},
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use mongodb::bson::doc;
use rand::Rng;
use std::sync::Arc;

#[derive(Clone)]
pub struct AdminService {
    db: MongoDb,
    redis: Arc<dyn TokenBlacklist>,
}

impl AdminService {
    pub fn new(db: MongoDb, redis: Arc<dyn TokenBlacklist>) -> Self {
        Self { db, redis }
    }

    pub async fn create_client(
        &self,
        req: CreateClientRequest,
    ) -> Result<CreateClientResponse, ServiceError> {
        let client_id = uuid::Uuid::new_v4().to_string();

        let client_secret = generate_random_encoded_bytes(32);
        let signing_secret = generate_random_encoded_bytes(32);

        let secret_hash = hash_password(&Password::new(client_secret.clone())).map_err(|e| {
            ServiceError::Internal(anyhow::anyhow!("Failed to hash client secret: {}", e))
        })?;

        let client = Client::new(
            client_id.clone(),
            secret_hash.into_string(),
            signing_secret.clone(),
            req.app_name.clone(),
            req.app_type,
            req.rate_limit_per_min,
            req.allowed_origins,
        );

        self.db
            .clients()
            .insert_one(&client, None)
            .await
            .map_err(ServiceError::Database)?;

        tracing::info!(
            client_id = %client.client_id,
            app_name = %client.app_name,
            "New client registered"
        );

        Ok(CreateClientResponse {
            client_id,
            client_secret,
            signing_secret,
            app_name: client.app_name,
            app_type: client.app_type,
        })
    }

    pub async fn rotate_client_secret(
        &self,
        client_id: String,
    ) -> Result<RotateSecretResponse, ServiceError> {
        let client = self
            .db
            .find_client_by_id(&client_id)
            .await
            .map_err(ServiceError::Database)?
            .ok_or(ServiceError::UserNotFound)?; // Using UserNotFound as a generic "resource not found" for now, or could add ResourceNotFound

        let new_client_secret = generate_random_encoded_bytes(32);
        let new_signing_secret = generate_random_encoded_bytes(32);

        let new_secret_hash =
            hash_password(&Password::new(new_client_secret.clone())).map_err(|e| {
                ServiceError::Internal(anyhow::anyhow!("Failed to hash new client secret: {}", e))
            })?;

        let now = chrono::Utc::now();
        let expiry = now + chrono::Duration::hours(24);

        self.db
            .clients()
            .update_one(
                doc! { "client_id": &client_id },
                doc! {
                    "$set": {
                        "client_secret_hash": new_secret_hash.into_string(),
                        "signing_secret": &new_signing_secret,
                        "previous_client_secret_hash": client.client_secret_hash,
                        "previous_secret_expiry": expiry,
                        "updated_at": now
                    }
                },
                None,
            )
            .await
            .map_err(ServiceError::Database)?;

        tracing::info!(client_id = %client_id, "Client secret rotated");

        Ok(RotateSecretResponse {
            client_id,
            new_client_secret,
            new_signing_secret,
            previous_secret_expiry: expiry,
        })
    }

    pub async fn revoke_client(&self, client_id: String) -> Result<(), ServiceError> {
        let result = self
            .db
            .clients()
            .update_one(
                doc! { "client_id": &client_id },
                doc! {
                    "$set": {
                        "enabled": false,
                        "updated_at": mongodb::bson::DateTime::from_chrono(chrono::Utc::now())
                    }
                },
                None,
            )
            .await
            .map_err(ServiceError::Database)?;

        if result.matched_count == 0 {
            return Err(ServiceError::UserNotFound);
        }

        Ok(())
    }

    pub async fn create_service_account(
        &self,
        req: CreateServiceAccountRequest,
        environment: &crate::config::Environment,
    ) -> Result<CreateServiceAccountResponse, ServiceError> {
        let prefix = match environment {
            crate::config::Environment::Prod => "svc_live_",
            crate::config::Environment::Dev => "svc_test_",
        };

        let random_part = generate_random_encoded_bytes(32);
        let api_key = format!("{}{}", prefix, random_part);

        let key_hash = hash_password(&Password::new(api_key.clone())).map_err(|e| {
            ServiceError::Internal(anyhow::anyhow!("Failed to hash API key: {}", e))
        })?;

        let lookup_hash = ServiceAccount::calculate_lookup_hash(&api_key);

        let service_account = ServiceAccount::new(
            req.service_name.clone(),
            key_hash.into_string(),
            lookup_hash,
            req.scopes.clone(),
        );

        let service_id = service_account.service_id.clone();

        self.db
            .service_accounts()
            .insert_one(&service_account, None)
            .await
            .map_err(ServiceError::Database)?;

        tracing::info!(
            service_id = %service_id,
            service_name = %service_account.service_name,
            "New service account registered"
        );

        Ok(CreateServiceAccountResponse {
            service_id,
            api_key,
            service_name: service_account.service_name,
            scopes: service_account.scopes,
        })
    }

    pub async fn rotate_service_key(
        &self,
        service_id: String,
        environment: &crate::config::Environment,
    ) -> Result<RotateServiceKeyResponse, ServiceError> {
        let account = self
            .db
            .find_service_account_by_id(&service_id)
            .await
            .map_err(ServiceError::Database)?
            .ok_or(ServiceError::UserNotFound)?;

        let prefix = match environment {
            crate::config::Environment::Prod => "svc_live_",
            crate::config::Environment::Dev => "svc_test_",
        };

        let random_part = generate_random_encoded_bytes(32);
        let new_api_key = format!("{}{}", prefix, random_part);

        let new_key_hash = hash_password(&Password::new(new_api_key.clone())).map_err(|e| {
            ServiceError::Internal(anyhow::anyhow!("Failed to hash new API key: {}", e))
        })?;

        let new_lookup_hash = ServiceAccount::calculate_lookup_hash(&new_api_key);

        let now = chrono::Utc::now();
        let expiry = now + chrono::Duration::days(7);

        self.db
            .service_accounts()
            .update_one(
                doc! { "service_id": &service_id },
                doc! {
                    "$set": {
                        "api_key_hash": new_key_hash.into_string(),
                        "api_key_lookup_hash": new_lookup_hash,
                        "previous_api_key_hash": account.api_key_hash.clone(),
                        "previous_api_key_lookup_hash": account.api_key_lookup_hash.clone(),
                        "previous_key_expiry": expiry,
                        "updated_at": now
                    }
                },
                None,
            )
            .await
            .map_err(ServiceError::Database)?;

        let old_cache_key = format!("svc_auth:{}", account.api_key_lookup_hash);
        let _ = self.redis.set_cache(&old_cache_key, "", 0).await;

        tracing::info!(service_id = %service_id, "Service API key rotated");

        Ok(RotateServiceKeyResponse {
            service_id,
            new_api_key,
            previous_key_expiry: expiry,
        })
    }

    pub async fn revoke_service_account(&self, service_id: String) -> Result<(), ServiceError> {
        let account = self
            .db
            .find_service_account_by_id(&service_id)
            .await
            .map_err(ServiceError::Database)?
            .ok_or(ServiceError::UserNotFound)?;

        let cache_key = format!("svc_auth:{}", account.api_key_lookup_hash);
        let _ = self.redis.set_cache(&cache_key, "", 0).await;
        if let Some(prev_hash) = account.previous_api_key_lookup_hash {
            let prev_cache_key = format!("svc_auth:{}", prev_hash);
            let _ = self.redis.set_cache(&prev_cache_key, "", 0).await;
        }

        self.db
            .service_accounts()
            .update_one(
                doc! { "service_id": &service_id },
                doc! {
                    "$set": {
                        "enabled": false,
                        "updated_at": mongodb::bson::DateTime::from_chrono(chrono::Utc::now())
                    }
                },
                None,
            )
            .await
            .map_err(ServiceError::Database)?;

        tracing::info!(service_id = %service_id, "Service account revoked");

        Ok(())
    }

    pub async fn get_service_audit_log(
        &self,
        service_id: String,
    ) -> Result<Vec<AuditLog>, ServiceError> {
        use mongodb::options::FindOptions;

        let filter = doc! { "service_id": service_id };
        let find_options = FindOptions::builder()
            .sort(doc! { "timestamp": -1 })
            .limit(100)
            .build();

        let mut cursor = self
            .db
            .audit_logs()
            .find(filter, find_options)
            .await
            .map_err(ServiceError::Database)?;

        let mut logs = Vec::new();
        while cursor.advance().await.map_err(ServiceError::Database)? {
            logs.push(
                cursor
                    .deserialize_current()
                    .map_err(|e| ServiceError::Internal(anyhow::anyhow!(e)))?,
            );
        }

        Ok(logs)
    }
}

fn generate_random_encoded_bytes(size: usize) -> String {
    let mut rng = rand::thread_rng();
    let mut bytes = vec![0u8; size];
    rng.fill(&mut bytes[..]);
    URL_SAFE_NO_PAD.encode(bytes)
}
