use crate::AppState;
use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Redirect, Response},
};
use tower_sessions::Session;

pub async fn auth_middleware(
    State(_state): State<AppState>,
    session: Session,
    request: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let access_token: Option<String> = session.get("access_token").await.unwrap_or(None);

    if access_token.is_none() {
        return Ok(Redirect::to("/login").into_response());
    }

    Ok(next.run(request).await)
}
