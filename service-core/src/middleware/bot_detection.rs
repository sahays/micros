use isbot::Bots;
use axum::{extract::Request, http::HeaderMap, middleware::Next, response::Response, http::Method};
use crate::error::AppError;
use tracing::warn;

pub async fn bot_detection_middleware(
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Result<Response, AppError> {
    let bots = Bots::default();
    if request.method() == Method::OPTIONS {
        return Ok(next.run(request).await);
    }

    if request.uri().path() == "/health" || request.uri().path() == "/metrics" {
        return Ok(next.run(request).await);
    }

    if headers.contains_key("X-Signature") || headers.contains_key("x-signature") {
        return Ok(next.run(request).await);
    }

    let user_agent = headers
        .get("User-Agent")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");

    let mut score = 0;

    if user_agent.is_empty() {
        score += 50;
    } else {
        if bots.is_bot(user_agent) {
            score += 100;
        }

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
                score += 70;
            } else if missing_headers == 1 {
                score += 30;
            }
        }
    }

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