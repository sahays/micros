//! Razorpay payment provider client.
//!
//! Implements Razorpay's Orders API for payment initiation and
//! signature verification for payment confirmation.

use crate::config::RazorpayConfig;
use anyhow::{anyhow, Result};
use hmac::{Hmac, Mac};
use reqwest::Client;
use secrecy::ExposeSecret;
use serde::{Deserialize, Serialize};
use sha2::Sha256;

/// Razorpay client for interacting with the Razorpay API.
#[derive(Clone)]
pub struct RazorpayClient {
    client: Client,
    config: RazorpayConfig,
}

/// Request to create a Razorpay order.
#[derive(Debug, Serialize)]
pub struct CreateOrderRequest {
    /// Amount in smallest currency unit (paise for INR).
    pub amount: u64,
    /// Currency code (e.g., "INR").
    pub currency: String,
    /// Receipt ID for tracking (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub receipt: Option<String>,
    /// Notes for the order (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<serde_json::Value>,
}

/// Response from Razorpay order creation.
#[derive(Debug, Deserialize)]
pub struct RazorpayOrder {
    /// Razorpay order ID.
    pub id: String,
    /// Entity type (always "order").
    pub entity: String,
    /// Amount in smallest currency unit.
    pub amount: u64,
    /// Amount paid so far.
    pub amount_paid: u64,
    /// Amount due.
    pub amount_due: u64,
    /// Currency code.
    pub currency: String,
    /// Receipt ID.
    pub receipt: Option<String>,
    /// Order status.
    pub status: String,
    /// Number of payment attempts.
    pub attempts: u32,
    /// Notes attached to the order.
    pub notes: Option<serde_json::Value>,
    /// Creation timestamp.
    pub created_at: u64,
}

/// Razorpay API error response.
#[derive(Debug, Deserialize)]
pub struct RazorpayError {
    pub error: RazorpayErrorDetail,
}

#[derive(Debug, Deserialize)]
pub struct RazorpayErrorDetail {
    pub code: String,
    pub description: String,
    pub source: Option<String>,
    pub step: Option<String>,
    pub reason: Option<String>,
}

/// Payment verification parameters.
#[derive(Debug)]
pub struct PaymentVerification {
    pub razorpay_order_id: String,
    pub razorpay_payment_id: String,
    pub razorpay_signature: String,
}

/// Razorpay webhook event.
#[derive(Debug, Deserialize)]
pub struct WebhookEvent {
    pub entity: String,
    pub account_id: String,
    pub event: String,
    pub contains: Vec<String>,
    pub payload: WebhookPayload,
    pub created_at: u64,
}

#[derive(Debug, Deserialize)]
pub struct WebhookPayload {
    pub payment: Option<WebhookPaymentEntity>,
    pub order: Option<WebhookOrderEntity>,
}

#[derive(Debug, Deserialize)]
pub struct WebhookPaymentEntity {
    pub entity: PaymentEntity,
}

#[derive(Debug, Deserialize)]
pub struct WebhookOrderEntity {
    pub entity: RazorpayOrder,
}

/// Razorpay payment entity.
#[derive(Debug, Deserialize)]
pub struct PaymentEntity {
    pub id: String,
    pub entity: String,
    pub amount: u64,
    pub currency: String,
    pub status: String,
    pub order_id: Option<String>,
    pub method: Option<String>,
    pub description: Option<String>,
    pub email: Option<String>,
    pub contact: Option<String>,
    pub created_at: u64,
    pub captured: Option<bool>,
}

impl RazorpayClient {
    /// Create a new Razorpay client.
    pub fn new(config: RazorpayConfig) -> Self {
        Self {
            client: Client::new(),
            config,
        }
    }

    /// Check if Razorpay is configured (credentials are set).
    pub fn is_configured(&self) -> bool {
        !self.config.key_id.is_empty() && !self.config.key_secret.expose_secret().is_empty()
    }

    /// Create a new order in Razorpay.
    ///
    /// # Arguments
    /// * `amount` - Amount in smallest currency unit (paise for INR)
    /// * `currency` - Currency code (e.g., "INR")
    /// * `receipt` - Optional receipt ID for tracking
    /// * `notes` - Optional notes
    pub async fn create_order(
        &self,
        amount: u64,
        currency: &str,
        receipt: Option<String>,
        notes: Option<serde_json::Value>,
    ) -> Result<RazorpayOrder> {
        if !self.is_configured() {
            return Err(anyhow!("Razorpay credentials not configured"));
        }

        let request = CreateOrderRequest {
            amount,
            currency: currency.to_string(),
            receipt,
            notes,
        };

        let url = format!("{}/orders", self.config.api_base_url);

        let response = self
            .client
            .post(&url)
            .basic_auth(
                &self.config.key_id,
                Some(self.config.key_secret.expose_secret()),
            )
            .json(&request)
            .send()
            .await?;

        let status = response.status();
        let body = response.text().await?;

        tracing::debug!(status = %status, body = %body, "Razorpay create_order response");

        if status.is_success() {
            let order: RazorpayOrder = serde_json::from_str(&body)?;
            tracing::info!(
                order_id = %order.id,
                amount = order.amount,
                currency = %order.currency,
                "Razorpay order created"
            );
            Ok(order)
        } else {
            let error: RazorpayError =
                serde_json::from_str(&body).unwrap_or_else(|_| RazorpayError {
                    error: RazorpayErrorDetail {
                        code: "UNKNOWN".to_string(),
                        description: body.clone(),
                        source: None,
                        step: None,
                        reason: None,
                    },
                });
            tracing::error!(
                code = %error.error.code,
                description = %error.error.description,
                "Razorpay order creation failed"
            );
            Err(anyhow!(
                "Razorpay error: {} - {}",
                error.error.code,
                error.error.description
            ))
        }
    }

    /// Fetch an existing order by ID.
    pub async fn get_order(&self, order_id: &str) -> Result<RazorpayOrder> {
        if !self.is_configured() {
            return Err(anyhow!("Razorpay credentials not configured"));
        }

        let url = format!("{}/orders/{}", self.config.api_base_url, order_id);

        let response = self
            .client
            .get(&url)
            .basic_auth(
                &self.config.key_id,
                Some(self.config.key_secret.expose_secret()),
            )
            .send()
            .await?;

        let status = response.status();
        let body = response.text().await?;

        if status.is_success() {
            let order: RazorpayOrder = serde_json::from_str(&body)?;
            Ok(order)
        } else {
            Err(anyhow!("Failed to fetch Razorpay order: {}", body))
        }
    }

    /// Verify payment signature from Razorpay checkout.
    ///
    /// The signature is computed as:
    /// `HMAC-SHA256(order_id + "|" + payment_id, key_secret)`
    pub fn verify_payment_signature(&self, verification: &PaymentVerification) -> Result<bool> {
        let payload = format!(
            "{}|{}",
            verification.razorpay_order_id, verification.razorpay_payment_id
        );

        let expected_signature =
            self.compute_signature(&payload, self.config.key_secret.expose_secret())?;

        let is_valid = expected_signature == verification.razorpay_signature;

        if is_valid {
            tracing::info!(
                order_id = %verification.razorpay_order_id,
                payment_id = %verification.razorpay_payment_id,
                "Payment signature verified successfully"
            );
        } else {
            tracing::warn!(
                order_id = %verification.razorpay_order_id,
                payment_id = %verification.razorpay_payment_id,
                "Payment signature verification failed"
            );
        }

        Ok(is_valid)
    }

    /// Verify webhook signature.
    ///
    /// The signature is computed as:
    /// `HMAC-SHA256(request_body, webhook_secret)`
    pub fn verify_webhook_signature(&self, body: &str, signature: &str) -> Result<bool> {
        let expected_signature =
            self.compute_signature(body, self.config.webhook_secret.expose_secret())?;

        let is_valid = expected_signature == signature;

        if !is_valid {
            tracing::warn!("Webhook signature verification failed");
        }

        Ok(is_valid)
    }

    /// Parse webhook event from request body.
    pub fn parse_webhook_event(&self, body: &str) -> Result<WebhookEvent> {
        let event: WebhookEvent = serde_json::from_str(body)?;
        Ok(event)
    }

    /// Compute HMAC-SHA256 signature.
    fn compute_signature(&self, payload: &str, secret: &str) -> Result<String> {
        type HmacSha256 = Hmac<Sha256>;
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
            .map_err(|_| anyhow!("Invalid key length"))?;
        mac.update(payload.as_bytes());
        let result = mac.finalize();
        Ok(hex::encode(result.into_bytes()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use secrecy::Secret;

    fn test_config() -> RazorpayConfig {
        RazorpayConfig {
            key_id: "rzp_test_123".to_string(),
            key_secret: Secret::new("test_secret".to_string()),
            webhook_secret: Secret::new("webhook_secret".to_string()),
            api_base_url: "https://api.razorpay.com/v1".to_string(),
        }
    }

    #[test]
    fn test_is_configured() {
        let client = RazorpayClient::new(test_config());
        assert!(client.is_configured());

        let empty_config = RazorpayConfig {
            key_id: "".to_string(),
            key_secret: Secret::new("".to_string()),
            webhook_secret: Secret::new("".to_string()),
            api_base_url: "".to_string(),
        };
        let client = RazorpayClient::new(empty_config);
        assert!(!client.is_configured());
    }

    #[test]
    fn test_payment_signature_verification() {
        let config = RazorpayConfig {
            key_id: "rzp_test_123".to_string(),
            key_secret: Secret::new("my_secret_key".to_string()),
            webhook_secret: Secret::new("webhook_secret".to_string()),
            api_base_url: "https://api.razorpay.com/v1".to_string(),
        };
        let client = RazorpayClient::new(config);

        // Compute expected signature manually
        let payload = "order_123|pay_456";
        let expected = client.compute_signature(payload, "my_secret_key").unwrap();

        let verification = PaymentVerification {
            razorpay_order_id: "order_123".to_string(),
            razorpay_payment_id: "pay_456".to_string(),
            razorpay_signature: expected,
        };

        assert!(client.verify_payment_signature(&verification).unwrap());
    }

    #[test]
    fn test_invalid_signature() {
        let client = RazorpayClient::new(test_config());

        let verification = PaymentVerification {
            razorpay_order_id: "order_123".to_string(),
            razorpay_payment_id: "pay_456".to_string(),
            razorpay_signature: "invalid_signature".to_string(),
        };

        assert!(!client.verify_payment_signature(&verification).unwrap());
    }
}
