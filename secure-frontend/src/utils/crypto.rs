use hex;
use hmac::{Hmac, Mac};
use secrecy::{ExposeSecret, Secret};
use sha2::{Digest, Sha256};

type HmacSha256 = Hmac<Sha256>;

pub fn compute_body_hash(body: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(body);
    let result = hasher.finalize();
    hex::encode(result)
}

pub fn create_signature(
    secret: &Secret<String>,
    method: &str,
    path: &str,
    timestamp: u64,
    nonce: &str,
    body_hash: &str,
) -> String {
    let payload = format!("{}|{}|{}|{}|{}", method, path, timestamp, nonce, body_hash);
    let mut mac = HmacSha256::new_from_slice(secret.expose_secret().as_bytes())
        .expect("HMAC can take key of any size");
    mac.update(payload.as_bytes());
    let result = mac.finalize();
    hex::encode(result.into_bytes())
}
