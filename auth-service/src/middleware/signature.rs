use axum::{
    body::Body,
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use http_body_util::BodyExt;
use mongodb::bson::doc;
use serde_json::json;

use crate::{utils::signature::verify_signature, AppState};

type AuthError = (StatusCode, Json<serde_json::Value>);

#[derive(serde::Deserialize)]
struct SignatureQuery {
    client_id: Option<String>,
    timestamp: Option<String>,
    nonce: Option<String>,
    signature: Option<String>,
}

pub async fn signature_validation_middleware(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Result<Response, Response> {
    // 0. Excluded paths (Health, JWKS, Email Verification, OAuth)
    let path = req.uri().path();
    if path == "/health"
        || path == "/.well-known/jwks.json"
        || path.starts_with("/auth/verify")
        || path.starts_with("/auth/google")
    {
        return Ok(next.run(req).await);
    }

    // 1. Check if signatures are required
    if !state.config.security.require_signatures {
        // Check if neither header nor query param present
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

    // 2. Extract Data (Headers or Query Params)
    let (client_id, timestamp_str, nonce, signature) =
        extract_auth_data(&req).map_err(|e| e.into_response())?;

    // 3. Validate Timestamp
    let timestamp: i64 = timestamp_str.parse().map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Invalid timestamp format"})),
        )
            .into_response()
    })?;

    let now = chrono::Utc::now().timestamp();
    if (now - timestamp).abs() > 60 {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(json!({"error": "Request timestamp expired"})),
        )
            .into_response());
    }

    // 4. Validate Nonce (Replay Attack Prevention)
    let nonce_key = format!("nonce:{}", nonce);
    let val = state.redis.get_cache(&nonce_key).await.map_err(|e| {
        tracing::error!("Redis error: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Internal server error"})),
        )
            .into_response()
    })?;

    if val.is_some() {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(json!({"error": "Replay detected (nonce used)"})),
        )
            .into_response());
    }

    // Set nonce with TTL (120s)
    state
        .redis
        .set_cache(&nonce_key, "1", 120)
        .await
        .map_err(|e| {
            tracing::error!("Redis error setting nonce: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Internal server error"})),
            )
                .into_response()
        })?;

    // 5. Fetch Client
    let client = state
        .db
        .clients()
        .find_one(doc! { "client_id": &client_id }, None)
        .await
        .map_err(|e| {
            tracing::error!("DB error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Internal server error"})),
            )
                .into_response()
        })?;

    let client = client.ok_or_else(|| {
        (
            StatusCode::UNAUTHORIZED,
            Json(json!({"error": "Invalid Client ID"})),
        )
            .into_response()
    })?;

    // 6. Read Body
    let (parts, body) = req.into_parts();
    let bytes = body
        .collect()
        .await
        .map_err(|e| {
            tracing::error!("Body read error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to read body"})),
            )
                .into_response()
        })?
        .to_bytes();

    let body_str = std::str::from_utf8(&bytes).unwrap_or("");

    // 7. Verify Signature
    let method = parts.method.as_str();
    let path = parts.uri.path();

    let is_valid = verify_signature(
        &client.signing_secret,
        method,
        path,
        timestamp,
        &nonce,
        body_str,
        &signature,
    )
    .map_err(|e| {
        tracing::error!("Signature verification error: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Signature verification failed"})),
        )
            .into_response()
    })?;

    if !is_valid {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(json!({"error": "Invalid signature"})),
        )
            .into_response());
    }

    // 8. Reconstruct Request
    let req = Request::from_parts(parts, Body::from(bytes));

    Ok(next.run(req).await)
}

fn extract_auth_data(req: &Request) -> Result<(String, String, String, String), AuthError> {
    let headers = req.headers();

    // Try Headers first
    if headers.contains_key("X-Signature") {
        let client_id = get_header(headers, "X-Client-ID")?;
        let timestamp = get_header(headers, "X-Timestamp")?;
        let nonce = get_header(headers, "X-Nonce")?;
        let signature = get_header(headers, "X-Signature")?;
        return Ok((client_id, timestamp, nonce, signature));
    }

    // Try Query Params
    if let Some(query) = req.uri().query() {
        let params: SignatureQuery = serde_urlencoded::from_str(query).map_err(|_| {
            (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": "Invalid query parameters"})),
            )
        })?;

        if let (Some(cid), Some(ts), Some(n), Some(sig)) = (
            params.client_id,
            params.timestamp,
            params.nonce,
            params.signature,
        ) {
            return Ok((cid, ts, n, sig));
        }
    }

    Err((
        StatusCode::UNAUTHORIZED,
        Json(json!({"error": "Missing signature data (headers or query params)"})),
    ))
}

fn get_header(headers: &HeaderMap, key: &str) -> Result<String, AuthError> {
    headers
        .get(key)
        .ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": format!("Missing header: {}", key)})),
            )
        })?
        .to_str()
        .map(|s| s.to_string())
        .map_err(|_| {
            (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": format!("Invalid header format: {}", key)})),
            )
        })
}
