use async_trait::async_trait;
use redis::{aio::ConnectionManager, Client};

#[async_trait]
pub trait TokenBlacklist: Send + Sync {
    async fn blacklist_token(
        &self,
        token_jti: &str,
        expiry_seconds: i64,
    ) -> Result<(), anyhow::Error>;
    async fn is_blacklisted(&self, token_jti: &str) -> Result<bool, anyhow::Error>;
    async fn health_check(&self) -> Result<(), anyhow::Error>;
}

#[derive(Clone)]
pub struct RedisService {
    _client: Client,
    manager: ConnectionManager,
}

impl RedisService {
    pub async fn new(config: &crate::config::RedisConfig) -> Result<Self, anyhow::Error> {
        tracing::info!(url = %config.url, "Connecting to Redis");
        let client = Client::open(config.url.clone())?;

        // Use ConnectionManager for automatic reconnection
        let manager = client.get_connection_manager().await?;

        tracing::info!("Successfully connected to Redis");

        Ok(Self {
            _client: client,
            manager,
        })
    }
}

#[async_trait]
impl TokenBlacklist for RedisService {
    async fn health_check(&self) -> Result<(), anyhow::Error> {
        let mut conn = self.manager.clone();
        redis::cmd("PING")
            .query_async(&mut conn)
            .await
            .map_err(|e| anyhow::anyhow!("Redis health check failed: {}", e))
    }

    /// Add a token to the blacklist with an expiry
    async fn blacklist_token(
        &self,
        token_jti: &str,
        expiry_seconds: i64,
    ) -> Result<(), anyhow::Error> {
        let mut conn = self.manager.clone();
        let key = format!("blacklist:{}", token_jti);

        redis::cmd("SET")
            .arg(&key)
            .arg("revoked")
            .arg("EX")
            .arg(expiry_seconds)
            .query_async(&mut conn)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to blacklist token: {}", e))
    }

    /// Check if a token is in the blacklist
    async fn is_blacklisted(&self, token_jti: &str) -> Result<bool, anyhow::Error> {
        let mut conn = self.manager.clone();
        let key = format!("blacklist:{}", token_jti);

        let exists: bool = redis::cmd("EXISTS")
            .arg(&key)
            .query_async(&mut conn)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to check blacklist: {}", e))?;

        Ok(exists)
    }
}

pub struct MockBlacklist {
    pub blacklisted_tokens: std::sync::Mutex<std::collections::HashSet<String>>,
}

impl Default for MockBlacklist {
    fn default() -> Self {
        Self::new()
    }
}

impl MockBlacklist {
    pub fn new() -> Self {
        Self {
            blacklisted_tokens: std::sync::Mutex::new(std::collections::HashSet::new()),
        }
    }
}

#[async_trait]
impl TokenBlacklist for MockBlacklist {
    async fn health_check(&self) -> Result<(), anyhow::Error> {
        Ok(())
    }

    async fn blacklist_token(
        &self,
        token_jti: &str,
        _expiry_seconds: i64,
    ) -> Result<(), anyhow::Error> {
        self.blacklisted_tokens
            .lock()
            .unwrap()
            .insert(token_jti.to_string());
        Ok(())
    }

    async fn is_blacklisted(&self, token_jti: &str) -> Result<bool, anyhow::Error> {
        Ok(self.blacklisted_tokens.lock().unwrap().contains(token_jti))
    }
}
