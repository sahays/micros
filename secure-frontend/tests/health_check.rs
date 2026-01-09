use secure_frontend::services::auth_client::AuthClient;
use secure_frontend::startup::build_router;
use std::sync::Arc;
use tower::util::ServiceExt;
use axum::{
    body::Body,
    http::{Request, StatusCode},
};

use secrecy::Secret;

#[tokio::test]
async fn health_check_works() {
    // 1. Setup (mock auth client config)
    let auth_config = secure_frontend::config::AuthServiceSettings {
        url: "http://localhost:8081".to_string(),
        client_id: "test_client".to_string(),
        signing_secret: Secret::new("test_secret".to_string()),
    };
    let auth_client = Arc::new(AuthClient::new(auth_config));

    let app = build_router(auth_client);

    // 2. Request
    let response = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // 3. Assert
    assert_eq!(response.status(), StatusCode::OK);
}
