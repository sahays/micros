use crate::error::AppError;
use crate::utils::signature::verify_signature;
use async_trait::async_trait;
use axum::{
    body::Body,
    extract::{Request, State},
    http::HeaderMap,
    middleware::Next,
    response::Response,
};
use http_body_util::BodyExt;
use serde::Deserialize;

#[derive(Clone, Debug, Default)]
pub struct SignatureConfig {
    pub require_signatures: bool,
    pub excluded_paths: Vec<String>,
}

#[async_trait]
pub trait SignatureStore: Send + Sync {
    async fn validate_nonce(&self, nonce: &str) -> Result<bool, AppError>;
    async fn get_signing_secret(&self, client_id: &str) -> Result<Option<String>, AppError>;
}

#[derive(Deserialize)]
struct SignatureQuery {
    client_id: Option<String>,
    timestamp: Option<String>,
    nonce: Option<String>,
    signature: Option<String>,
}

pub async fn signature_validation_middleware<S>(
    State(state): State<S>,
    req: Request,
    next: Next,
) -> Result<Response, AppError>
where
    S: AsRef<SignatureConfig> + SignatureStore + Clone + Send + Sync + 'static,
{
    let config = state.as_ref();
    let path = req.uri().path();

    if config
        .excluded_paths
        .iter()
        .any(|p| path == p || path.starts_with(p))
    {
        return Ok(next.run(req).await);
    }

    if !config.require_signatures {
        let has_header = req.headers().contains_key("X-Signature");
        let has_query = req
            .uri()
            .query()
            .map(|q| q.contains("signature="))
            .unwrap_or(false);
        if !has_header && !has_query {
            return Ok(next.run(req).await);
        }
    }

    let (client_id, timestamp_str, nonce, signature) = extract_auth_data(&req)?;

    let timestamp: i64 = timestamp_str
        .parse()
        .map_err(|_| AppError::AuthError(anyhow::anyhow!("Invalid timestamp format")))?;

    let now = chrono::Utc::now().timestamp();
    if (now - timestamp).abs() > 60 {
        return Err(AppError::AuthError(anyhow::anyhow!(
            "Request timestamp expired"
        )));
    }

    if !state.validate_nonce(&nonce).await? {
        return Err(AppError::AuthError(anyhow::anyhow!(
            "Replay detected (nonce used)"
        )));
    }

    let secret = state.get_signing_secret(&client_id).await?;
    let secret = secret.ok_or_else(|| AppError::AuthError(anyhow::anyhow!("Invalid Client ID")))?;

    let (parts, body) = req.into_parts();
    let bytes = body
        .collect()
        .await
        .map_err(|e| AppError::InternalError(anyhow::anyhow!("Failed to read body: {}", e)))?
        .to_bytes();

    let body_str = std::str::from_utf8(&bytes).unwrap_or("");

    let method = parts.method.as_str();
    let path = parts.uri.path();

    let is_valid = verify_signature(
        &secret, method, path, timestamp, &nonce, body_str, &signature,
    )
    .map_err(|e| AppError::InternalError(anyhow::anyhow!("Signature verification error: {}", e)))?;

    if !is_valid {
        return Err(AppError::AuthError(anyhow::anyhow!("Invalid signature")));
    }

    let req = Request::from_parts(parts, Body::from(bytes));
    Ok(next.run(req).await)
}

fn extract_auth_data(req: &Request) -> Result<(String, String, String, String), AppError> {
    let headers = req.headers();

    if headers.contains_key("X-Signature") {
        let client_id = get_header(headers, "X-Client-ID")?;
        let timestamp = get_header(headers, "X-Timestamp")?;
        let nonce = get_header(headers, "X-Nonce")?;
        let signature = get_header(headers, "X-Signature")?;
        return Ok((client_id, timestamp, nonce, signature));
    }

    if let Some(query) = req.uri().query() {
        let params: SignatureQuery = serde_urlencoded::from_str(query)
            .map_err(|_| AppError::AuthError(anyhow::anyhow!("Invalid query parameters")))?;

        if let (Some(cid), Some(ts), Some(n), Some(sig)) = (
            params.client_id,
            params.timestamp,
            params.nonce,
            params.signature,
        ) {
            return Ok((cid, ts, n, sig));
        }
    }

    Err(AppError::AuthError(anyhow::anyhow!(
        "Missing signature data (headers or query params)"
    )))
}

fn get_header(headers: &HeaderMap, key: &str) -> Result<String, AppError> {
    headers
        .get(key)
        .ok_or_else(|| AppError::AuthError(anyhow::anyhow!("Missing header: {}", key)))?
        .to_str()
        .map(|s| s.to_string())
        .map_err(|_| AppError::AuthError(anyhow::anyhow!("Invalid header format: {}", key)))
}
