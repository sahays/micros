use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

type HmacSha256 = Hmac<Sha256>;

/// Generate HMAC-SHA256 signature
///
/// Format: HMAC-SHA256(method|path|timestamp|nonce|body_hash, secret)
pub fn generate_signature(
    secret: &str,
    method: &str,
    path: &str,
    timestamp: i64,
    nonce: &str,
    body: &str,
) -> Result<String, anyhow::Error> {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .map_err(|e| anyhow::anyhow!("Invalid key length: {}", e))?;

    // Calculate body hash (SHA256)
    let body_hash = hex::encode(Sha256::digest(body.as_bytes()));

    // Construct payload
    let payload = format!("{}|{}|{}|{}|{}", method, path, timestamp, nonce, body_hash);

    mac.update(payload.as_bytes());
    let result = mac.finalize();

    Ok(hex::encode(result.into_bytes()))
}

/// Verify HMAC-SHA256 signature using constant-time comparison
pub fn verify_signature(
    secret: &str,
    method: &str,
    path: &str,
    timestamp: i64,
    nonce: &str,
    body: &str,
    signature: &str,
) -> Result<bool, anyhow::Error> {
    let expected_signature = generate_signature(secret, method, path, timestamp, nonce, body)?;

    // Constant time comparison
    let expected_bytes = expected_signature.as_bytes();
    let signature_bytes = signature.as_bytes();

    if expected_bytes.len() != signature_bytes.len() {
        return Ok(false);
    }

    Ok(expected_bytes.ct_eq(signature_bytes).into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signature_generation_and_verification() {
        let secret = "my_secret_key";
        let method = "POST";
        let path = "/api/v1/resource";
        let timestamp = 1678886400;
        let nonce = "random_nonce_123";
        let body = r#"{"foo":"bar"}"#;

        let signature = generate_signature(secret, method, path, timestamp, nonce, body).unwrap();
        assert!(!signature.is_empty());

        let is_valid =
            verify_signature(secret, method, path, timestamp, nonce, body, &signature).unwrap();
        assert!(is_valid);
    }

    #[test]
    fn test_invalid_signature() {
        let secret = "my_secret_key";
        let method = "POST";
        let path = "/api/v1/resource";
        let timestamp = 1678886400;
        let nonce = "random_nonce_123";
        let body = r#"{"foo":"bar"}"#;

        let signature = generate_signature(secret, method, path, timestamp, nonce, body).unwrap();
        let invalid_signature = format!("a{}", &signature[1..]);

        let is_valid = verify_signature(
            secret,
            method,
            path,
            timestamp,
            nonce,
            body,
            &invalid_signature,
        )
        .unwrap();
        assert!(!is_valid);
    }

    #[test]
    fn test_tampered_body() {
        let secret = "my_secret_key";
        let method = "POST";
        let path = "/api/v1/resource";
        let timestamp = 1678886400;
        let nonce = "random_nonce_123";
        let body = r#"{"foo":"bar"}"#;

        let signature = generate_signature(secret, method, path, timestamp, nonce, body).unwrap();

        let modified_body = r#"{"foo":"baz"}"#;
        let is_valid = verify_signature(
            secret,
            method,
            path,
            timestamp,
            nonce,
            modified_body,
            &signature,
        )
        .unwrap();
        assert!(!is_valid);
    }
}
