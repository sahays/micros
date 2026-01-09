use isbot::Bots;
use service_core::axum::{extract::Request, http::HeaderMap, middleware::Next, response::Response};
use service_core::error::AppError;
use tracing::warn;

/// Middleware to detect and block bots based on heuristic analysis.
///
/// checks:
/// 1. Known bot User-Agents (using `isbot` crate).
/// 2. Missing standard headers for clients claiming to be browsers.
/// 3. Empty User-Agent strings.
///
/// Exemptions:
/// - Service-to-service calls (identified by presence of X-Signature header)
pub async fn bot_detection_middleware(
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Result<Response, AppError> {
    let bots = Bots::default();
    // Skip OPTIONS requests (CORS preflight)
    if request.method() == service_core::axum::http::Method::OPTIONS {
        return Ok(next.run(request).await);
    }

    // Skip Health Check and Metrics
    if request.uri().path() == "/health" || request.uri().path() == "/metrics" {
        return Ok(next.run(request).await);
    }

    // Skip service-to-service calls (identified by signature headers)
    // Service clients use request signing for authentication
    if headers.contains_key("X-Signature") || headers.contains_key("x-signature") {
        return Ok(next.run(request).await);
    }

    let user_agent = headers
        .get("User-Agent")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");

    let mut score = 0;

    // 1. Check for empty User-Agent
    if user_agent.is_empty() {
        // Empty UA is suspicious for public facing endpoints, but maybe okay for some API clients.
        // We'll give it a score but maybe not block immediately unless other factors apply,
        // or strictly block if we enforce UA.
        score += 50;
    } else {
        // 2. Check for known bots
        if bots.is_bot(user_agent) {
            score += 100;
        }

        // 3. Heuristic Analysis for "Browsers"
        // If it looks like a browser (Mozilla/5.0...), it should behave like one.
        if user_agent.starts_with("Mozilla/") {
            let mut missing_headers = 0;
            if !headers.contains_key("Accept") {
                missing_headers += 1;
            }
            if !headers.contains_key("Accept-Language") {
                missing_headers += 1;
            }
            if !headers.contains_key("Accept-Encoding") {
                missing_headers += 1;
            }

            if missing_headers >= 2 {
                score += 70; // High suspicion if multiple standard headers are missing
            } else if missing_headers == 1 {
                score += 30;
            }
        }
    }

    // Threshold for blocking
    if score >= 100 {
        warn!(
            user_agent = %user_agent,
            score = %score,
            path = %request.uri(),
            "Blocking suspected bot request"
        );
        return Err(AppError::Forbidden(anyhow::anyhow!("Bot detected")));
    }

    Ok(next.run(request).await)
}
