use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::fs;
use uuid::Uuid;

use crate::config::JwtConfig;

/// JWT service for token generation and validation
#[derive(Clone)]
pub struct JwtService {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    access_token_expiry_minutes: i64,
    refresh_token_expiry_days: i64,
}

/// Claims for access tokens (short-lived)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessTokenClaims {
    /// Subject (user ID)
    pub sub: String,
    /// Email
    pub email: String,
    /// Expiration time (Unix timestamp)
    pub exp: i64,
    /// Issued at (Unix timestamp)
    pub iat: i64,
    /// JWT ID (for blacklisting)
    pub jti: String,
}

/// Claims for refresh tokens (long-lived)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshTokenClaims {
    /// Subject (user ID)
    pub sub: String,
    /// Token ID (matches database record)
    pub jti: String,
    /// Expiration time (Unix timestamp)
    pub exp: i64,
    /// Issued at (Unix timestamp)
    pub iat: i64,
}

/// Token response returned to client
#[derive(Debug, Serialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: i64,
}

impl JwtService {
    /// Create a new JWT service by loading RSA keys from files
    pub fn new(config: &JwtConfig) -> Result<Self, anyhow::Error> {
        // Load private key for signing
        let private_key_pem = fs::read_to_string(&config.private_key_path)
            .map_err(|e| {
                anyhow::anyhow!(
                    "Failed to read private key from {}: {}",
                    config.private_key_path,
                    e
                )
            })?;

        let encoding_key = EncodingKey::from_rsa_pem(private_key_pem.as_bytes())
            .map_err(|e| anyhow::anyhow!("Failed to parse private key: {}", e))?;

        // Load public key for verification
        let public_key_pem = fs::read_to_string(&config.public_key_path)
            .map_err(|e| {
                anyhow::anyhow!(
                    "Failed to read public key from {}: {}",
                    config.public_key_path,
                    e
                )
            })?;

        let decoding_key = DecodingKey::from_rsa_pem(public_key_pem.as_bytes())
            .map_err(|e| anyhow::anyhow!("Failed to parse public key: {}", e))?;

        tracing::info!("JWT service initialized with RS256 keys");

        Ok(Self {
            encoding_key,
            decoding_key,
            access_token_expiry_minutes: config.access_token_expiry_minutes,
            refresh_token_expiry_days: config.refresh_token_expiry_days,
        })
    }

    /// Generate an access token for a user
    pub fn generate_access_token(
        &self,
        user_id: &str,
        email: &str,
    ) -> Result<String, anyhow::Error> {
        let now = Utc::now();
        let exp = now + Duration::minutes(self.access_token_expiry_minutes);

        let claims = AccessTokenClaims {
            sub: user_id.to_string(),
            email: email.to_string(),
            exp: exp.timestamp(),
            iat: now.timestamp(),
            jti: Uuid::new_v4().to_string(),
        };

        let header = Header::new(Algorithm::RS256);
        let token = encode(&header, &claims, &self.encoding_key)
            .map_err(|e| anyhow::anyhow!("Failed to encode access token: {}", e))?;

        Ok(token)
    }

    /// Generate a refresh token for a user
    pub fn generate_refresh_token(
        &self,
        user_id: &str,
        token_id: &str,
    ) -> Result<String, anyhow::Error> {
        let now = Utc::now();
        let exp = now + Duration::days(self.refresh_token_expiry_days);

        let claims = RefreshTokenClaims {
            sub: user_id.to_string(),
            jti: token_id.to_string(),
            exp: exp.timestamp(),
            iat: now.timestamp(),
        };

        let header = Header::new(Algorithm::RS256);
        let token = encode(&header, &claims, &self.encoding_key)
            .map_err(|e| anyhow::anyhow!("Failed to encode refresh token: {}", e))?;

        Ok(token)
    }

    /// Generate both access and refresh tokens
    pub fn generate_token_pair(
        &self,
        user_id: &str,
        email: &str,
    ) -> Result<(String, String, String), anyhow::Error> {
        let access_token = self.generate_access_token(user_id, email)?;
        let refresh_token_id = Uuid::new_v4().to_string();
        let refresh_token = self.generate_refresh_token(user_id, &refresh_token_id)?;

        Ok((access_token, refresh_token, refresh_token_id))
    }

    /// Validate and decode an access token
    pub fn validate_access_token(
        &self,
        token: &str,
    ) -> Result<AccessTokenClaims, anyhow::Error> {
        let mut validation = Validation::new(Algorithm::RS256);
        validation.validate_exp = true;

        let token_data = decode::<AccessTokenClaims>(token, &self.decoding_key, &validation)
            .map_err(|e| anyhow::anyhow!("Invalid access token: {}", e))?;

        Ok(token_data.claims)
    }

    /// Validate and decode a refresh token
    pub fn validate_refresh_token(
        &self,
        token: &str,
    ) -> Result<RefreshTokenClaims, anyhow::Error> {
        let mut validation = Validation::new(Algorithm::RS256);
        validation.validate_exp = true;

        let token_data = decode::<RefreshTokenClaims>(token, &self.decoding_key, &validation)
            .map_err(|e| anyhow::anyhow!("Invalid refresh token: {}", e))?;

        Ok(token_data.claims)
    }

    /// Get access token expiry in seconds (for client info)
    pub fn access_token_expiry_seconds(&self) -> i64 {
        self.access_token_expiry_minutes * 60
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn create_test_keys() -> Result<(NamedTempFile, NamedTempFile), anyhow::Error> {
        // Generate test RSA key pair
        let private_key = r#"-----BEGIN RSA PRIVATE KEY-----
MIIEpAIBAAKCAQEAu1SU1LfVLPHCozMxH2Mo4lgOEePzNm0tRgeLezV6ffAt0gun
VTLw7onLRnrq0/IzKT6WR0yNu3KjdIQJzkn9FS5lL+h9yPGYEPUhLl4YQZXT/r6B
Z6YJwBEYR9fxqBPH1CJnPv4Lk7qfQpK4lCd/k3s3jCQJFTgk6z9GNsKh6hI7WtQ7
a/Z3pJ2V1pYx9TcZQiJhVVJCRoT1IxHLVxV1PwTCh6tXbUv/8U/7a1TfQPrPpJ0V
fN8gPMZLQdVvNW6JwG5ZkT1mHWRNqsKpvgJCFCwwVG3C6vYfJOJPv0J1WkUB2jv+
e2OHiN0d3LQPMkfTLz/kL1KCJqBQV6QKLWlqBwIDAQABAoIBAC2qHPCPD0D2NzFs
bPATH8dmLMPvgR4nY7DjABfH0iFLTxIXjDDY0mQxqYWPx0GEGPPt0g7Q3FLKC7gG
fBFVQtC9NWQFLvNiMJjzQBIxCT6zDLo2LwqyPQENYq6oZiLBPLTmBn6g7pqcCRkg
5j5RxmW2p5bPEKvG4GhCKkPL1bI4aPZ4Q2XFpPPLXaL0tNvvXdmXHq0uFHzvxJXH
nqGWwOgR7Js9RI8mNk7d8PQ3Y/VRMmQnV8VVVS5QGHfz8PvKyqrPNbNHCqk3A8WX
YrNdVYGxJ7fQRVfgv1vJR/fJpjTBRLs9xCZqLLqkh3bqPJHB8aLbFE8mPxLaPJXW
Y8fNqCECgYEA3h2Q8vxrFZZLB4Cq9S0WnV2TqLh7bOCg7nWHCLPLU0Vk9PGqNQUk
9FqBHWKNVxYOVPPXG8dJ4mBNqD2dGkQPDwRnWlRAKqQNqkXCe0sLlqWGg0hVV1cQ
IVqJCON3HUKzGcC3fLuV8nfQHQHvO9gQn8r1VCLLYqRNYvnPXLPHgr0CgYEA2IbJ
n0vPxGCfqGMZf8Kd8JnO9TKHZcE7QYNLDqGxLfRm7TqDdj0gKJzFhWdMVF1Hs7B7
WLBfBP9SfM0mKLVqN5TW0VPqdBZvP6V6N3KrHQ1LoqBB+oLvHBvLcSZLbZLbdcWX
G9QQRpQqL7XqLfMwqGNz7VGBpMTBfLPcQsT9AjMCgYEAxCR5K+rH8tL3fNOKLT3L
sJzGGnBFQa0jZpX0MmBHGhZGgLqPJdLpE4G8sGqBaXOKQqvNqhL6GWr7Vpo+0bqN
xYQMF6y8xm6U6TVJdP6oPaTPi8g8ULfkWLhJ7DLj0c3LaW6VJkl0pXLCvB9kPTXV
kL5YmfWFbYq2NqPK1e3qpqUCgYEAxOKNNRPQ5LVOJMqFqH1lGQGQZl5kR7XNVVVZ
YZQH0fWJcL7MQpgqHJfVIAMXHSQKvKqR1LkKzQvqVxqHPgLm3BcQfNV8YFrvUn7m
xvNB0f0nVqNqYLBQZRyQvLQhRfGJKYGtNHJMZkRTTgGLLYlM6VKBd1L7KGPVmMGz
HQCQe4MCgYBcL8GWqMCqPfR9s3j1hqL7RwKJTRLXhQJ0LG1R8ULBfMRRdJdxJXPQ
Y8WLXJPZ2NBWqvgKJPFGLQaQfHqABJmGqEHVvLGKMC8qQOy1lJYzRVh+PqCYxCY9
YN8eCxvHYzT7fYN8x/SqLWnVD6P3uR3d3F1P3Lh0qNE3ZHNL0QHfJg==
-----END RSA PRIVATE KEY-----"#;

        let public_key = r#"-----BEGIN PUBLIC KEY-----
MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAu1SU1LfVLPHCozMxH2Mo
4lgOEePzNm0tRgeLezV6ffAt0gunVTLw7onLRnrq0/IzKT6WR0yNu3KjdIQJzkn9
FS5lL+h9yPGYEPUhLl4YQZXT/r6BZ6YJwBEYR9fxqBPH1CJnPv4Lk7qfQpK4lCd/
k3s3jCQJFTgk6z9GNsKh6hI7WtQ7a/Z3pJ2V1pYx9TcZQiJhVVJCRoT1IxHLVxV1
PwTCh6tXbUv/8U/7a1TfQPrPpJ0VfN8gPMZLQdVvNW6JwG5ZkT1mHWRNqsKpvgJC
FCwwVG3C6vYfJOJPv0J1WkUB2jv+e2OHiN0d3LQPMkfTLz/kL1KCJqBQV6QKLWlq
BwIDAQAB
-----END PUBLIC KEY-----"#;

        let mut private_file = NamedTempFile::new()?;
        private_file.write_all(private_key.as_bytes())?;

        let mut public_file = NamedTempFile::new()?;
        public_file.write_all(public_key.as_bytes())?;

        Ok((private_file, public_file))
    }

    #[test]
    fn test_jwt_service_creation() -> Result<(), anyhow::Error> {
        let (private_file, public_file) = create_test_keys()?;

        let config = JwtConfig {
            private_key_path: private_file.path().to_str().unwrap().to_string(),
            public_key_path: public_file.path().to_str().unwrap().to_string(),
            access_token_expiry_minutes: 15,
            refresh_token_expiry_days: 7,
        };

        let service = JwtService::new(&config)?;
        assert_eq!(service.access_token_expiry_minutes, 15);
        assert_eq!(service.refresh_token_expiry_days, 7);

        Ok(())
    }

    #[test]
    fn test_access_token_generation_and_validation() -> Result<(), anyhow::Error> {
        let (private_file, public_file) = create_test_keys()?;

        let config = JwtConfig {
            private_key_path: private_file.path().to_str().unwrap().to_string(),
            public_key_path: public_file.path().to_str().unwrap().to_string(),
            access_token_expiry_minutes: 15,
            refresh_token_expiry_days: 7,
        };

        let service = JwtService::new(&config)?;

        let token = service.generate_access_token("user_123", "test@example.com")?;
        assert!(!token.is_empty());

        let claims = service.validate_access_token(&token)?;
        assert_eq!(claims.sub, "user_123");
        assert_eq!(claims.email, "test@example.com");

        Ok(())
    }

    #[test]
    fn test_refresh_token_generation_and_validation() -> Result<(), anyhow::Error> {
        let (private_file, public_file) = create_test_keys()?;

        let config = JwtConfig {
            private_key_path: private_file.path().to_str().unwrap().to_string(),
            public_key_path: public_file.path().to_str().unwrap().to_string(),
            access_token_expiry_minutes: 15,
            refresh_token_expiry_days: 7,
        };

        let service = JwtService::new(&config)?;

        let token_id = "token_abc";
        let token = service.generate_refresh_token("user_123", token_id)?;
        assert!(!token.is_empty());

        let claims = service.validate_refresh_token(&token)?;
        assert_eq!(claims.sub, "user_123");
        assert_eq!(claims.jti, token_id);

        Ok(())
    }

    #[test]
    fn test_token_pair_generation() -> Result<(), anyhow::Error> {
        let (private_file, public_file) = create_test_keys()?;

        let config = JwtConfig {
            private_key_path: private_file.path().to_str().unwrap().to_string(),
            public_key_path: public_file.path().to_str().unwrap().to_string(),
            access_token_expiry_minutes: 15,
            refresh_token_expiry_days: 7,
        };

        let service = JwtService::new(&config)?;

        let (access_token, refresh_token, refresh_token_id) =
            service.generate_token_pair("user_123", "test@example.com")?;

        assert!(!access_token.is_empty());
        assert!(!refresh_token.is_empty());
        assert!(!refresh_token_id.is_empty());

        let access_claims = service.validate_access_token(&access_token)?;
        assert_eq!(access_claims.sub, "user_123");

        let refresh_claims = service.validate_refresh_token(&refresh_token)?;
        assert_eq!(refresh_claims.jti, refresh_token_id);

        Ok(())
    }
}
