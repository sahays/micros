use askama::Template;
use axum::response::IntoResponse;

#[derive(Template)]
#[template(path = "index.html")]
pub struct IndexTemplate {}

pub async fn index() -> impl IntoResponse {
    IndexTemplate {}
}

pub async fn health_check() -> &'static str {
    "OK"
}
