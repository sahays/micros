//! Gmail API client using Google Service Account with domain-wide delegation.
//!
//! Sends emails via the Gmail REST API by impersonating a domain user.
//! Uses JWT (RS256) to obtain access tokens, with automatic caching.

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use chrono::Utc;
use jsonwebtoken::{encode, EncodingKey, Header};
use lettre::message::header::ContentType;
use lettre::Message;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Configuration for the Gmail API client.
#[derive(Debug, Clone)]
pub struct GmailApiConfig {
    pub service_account_key_path: String,
    pub sender_email: String,
    pub sender_name: String,
    pub enabled: bool,
}

/// Deserialized Google service account key file.
#[derive(Debug, Clone, Deserialize)]
struct ServiceAccountKey {
    client_email: String,
    private_key: String,
    token_uri: String,
}

/// JWT claims for Google OAuth2 token exchange.
#[derive(Debug, Serialize)]
struct JwtClaims {
    iss: String,
    sub: String,
    scope: String,
    aud: String,
    iat: i64,
    exp: i64,
}

/// Cached access token with expiry.
#[derive(Debug, Clone)]
struct CachedToken {
    access_token: String,
    expires_at: i64,
}

/// Google token response.
#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    expires_in: i64,
}

/// Gmail API client that sends emails by impersonating a domain user
/// via a Google service account with domain-wide delegation.
pub struct GmailApiClient {
    service_account_key: ServiceAccountKey,
    sender_email: String,
    sender_name: String,
    http_client: Client,
    cached_token: Arc<RwLock<Option<CachedToken>>>,
}

impl GmailApiClient {
    /// Create a new Gmail API client from configuration.
    pub fn new(config: &GmailApiConfig) -> Result<Self, anyhow::Error> {
        let key_path = Path::new(&config.service_account_key_path);
        let key_data = std::fs::read_to_string(key_path).map_err(|e| {
            anyhow::anyhow!(
                "Failed to read service account key file '{}': {}",
                config.service_account_key_path,
                e
            )
        })?;

        let service_account_key: ServiceAccountKey =
            serde_json::from_str(&key_data).map_err(|e| {
                anyhow::anyhow!("Failed to parse service account key JSON: {}", e)
            })?;

        let http_client = Client::new();

        tracing::info!(
            sender = %config.sender_email,
            service_account = %service_account_key.client_email,
            "Gmail API client initialized"
        );

        Ok(Self {
            service_account_key,
            sender_email: config.sender_email.clone(),
            sender_name: config.sender_name.clone(),
            http_client,
            cached_token: Arc::new(RwLock::new(None)),
        })
    }

    /// Get a valid access token, refreshing if expired or not yet cached.
    async fn get_access_token(&self) -> Result<String, anyhow::Error> {
        // Check cache first
        {
            let cache = self.cached_token.read().await;
            if let Some(ref token) = *cache {
                let now = Utc::now().timestamp();
                // 60-second safety margin before expiry
                if token.expires_at - 60 > now {
                    return Ok(token.access_token.clone());
                }
            }
        }

        // Token expired or not cached — mint a new JWT and exchange it
        let now = Utc::now().timestamp();
        let claims = JwtClaims {
            iss: self.service_account_key.client_email.clone(),
            sub: self.sender_email.clone(),
            scope: "https://www.googleapis.com/auth/gmail.send".to_string(),
            aud: self.service_account_key.token_uri.clone(),
            iat: now,
            exp: now + 3600, // 1 hour
        };

        let header = Header::new(jsonwebtoken::Algorithm::RS256);
        let encoding_key = EncodingKey::from_rsa_pem(
            self.service_account_key.private_key.as_bytes(),
        )
        .map_err(|e| anyhow::anyhow!("Failed to create RSA encoding key: {}", e))?;

        let jwt_token = encode(&header, &claims, &encoding_key)
            .map_err(|e| anyhow::anyhow!("Failed to encode JWT: {}", e))?;

        // Exchange JWT for access token
        let response = self
            .http_client
            .post(&self.service_account_key.token_uri)
            .form(&[
                ("grant_type", "urn:ietf:params:oauth:grant-type:jwt-bearer"),
                ("assertion", &jwt_token),
            ])
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Token exchange request failed: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "Token exchange failed with status {}: {}",
                status,
                body
            ));
        }

        let token_response: TokenResponse = response
            .json()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to parse token response: {}", e))?;

        let cached = CachedToken {
            access_token: token_response.access_token.clone(),
            expires_at: now + token_response.expires_in,
        };

        // Update cache
        {
            let mut cache = self.cached_token.write().await;
            *cache = Some(cached);
        }

        Ok(token_response.access_token)
    }

    /// Send an email via the Gmail API.
    pub async fn send_email(
        &self,
        to: &str,
        subject: &str,
        body_text: &str,
        body_html: &str,
        from_name: Option<&str>,
        reply_to: Option<&str>,
    ) -> Result<(), anyhow::Error> {
        let display_name = from_name.unwrap_or(&self.sender_name);
        let from_address = format!("{} <{}>", display_name, self.sender_email);

        let mut builder = Message::builder()
            .from(
                from_address
                    .parse()
                    .map_err(|e: lettre::address::AddressError| {
                        anyhow::anyhow!("Invalid from address: {}", e)
                    })?,
            )
            .to(to
                .parse()
                .map_err(|e: lettre::address::AddressError| {
                    anyhow::anyhow!("Invalid to address: {}", e)
                })?)
            .subject(subject);

        if let Some(reply_to_addr) = reply_to {
            builder = builder.reply_to(
                reply_to_addr
                    .parse()
                    .map_err(|e: lettre::address::AddressError| {
                        anyhow::anyhow!("Invalid reply-to address: {}", e)
                    })?,
            );
        }

        let message = builder
            .multipart(
                lettre::message::MultiPart::alternative()
                    .singlepart(
                        lettre::message::SinglePart::builder()
                            .header(ContentType::TEXT_PLAIN)
                            .body(body_text.to_string()),
                    )
                    .singlepart(
                        lettre::message::SinglePart::builder()
                            .header(ContentType::TEXT_HTML)
                            .body(body_html.to_string()),
                    ),
            )
            .map_err(|e| anyhow::anyhow!("Failed to build email message: {}", e))?;

        self.send_raw_email(&message).await
    }

    /// Send a pre-built lettre Message via the Gmail API.
    pub async fn send_raw_email(&self, message: &Message) -> Result<(), anyhow::Error> {
        let raw_email = URL_SAFE_NO_PAD.encode(message.formatted());

        let access_token = self.get_access_token().await?;

        let url = format!(
            "https://gmail.googleapis.com/gmail/v1/users/{}/messages/send",
            self.sender_email
        );

        let body = serde_json::json!({ "raw": raw_email });

        let response = self
            .http_client
            .post(&url)
            .bearer_auth(&access_token)
            .json(&body)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Gmail API request failed: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "Gmail API send failed with status {}: {}",
                status,
                body
            ));
        }

        Ok(())
    }

    /// Get the sender email address.
    pub fn sender_email(&self) -> &str {
        &self.sender_email
    }

    /// Get the sender display name.
    pub fn sender_name(&self) -> &str {
        &self.sender_name
    }
}
