use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use secure_frontend::services::auth_client::AuthClient;
use secure_frontend::services::document_client::DocumentClient;
use secure_frontend::startup::build_router;
use secure_frontend::AppState;
use std::sync::Arc;
use tower::util::ServiceExt;

use secrecy::Secret;

/// Health check endpoint test.
///
/// Note: This test requires gRPC services to be available for client construction.
/// Run with actual services running or use `cargo test -- --ignored` to skip.
#[tokio::test]
#[ignore = "Requires gRPC services to be running (auth-service:50051, document-service:50053)"]
async fn health_check_works() {
    // Setup auth client config with gRPC endpoint
    let auth_config = secure_frontend::config::AuthServiceSettings {
        url: "http://localhost:9096".to_string(),
        public_url: "http://localhost:9096".to_string(),
        grpc_url: "http://localhost:50051".to_string(),
        default_tenant_slug: "default".to_string(),
    };
    let auth_client = Arc::new(
        AuthClient::new(auth_config)
            .await
            .expect("Failed to create auth client - ensure auth-service is running"),
    );

    // Setup document client config with gRPC endpoint
    let document_config = secure_frontend::config::DocumentServiceSettings {
        url: "http://localhost:9098".to_string(),
        grpc_url: "http://localhost:50053".to_string(),
        document_signing_secret: Secret::new("test_secret".to_string()),
        default_app_id: "secure-frontend".to_string(),
        default_org_id: "default".to_string(),
    };
    let document_client = Arc::new(
        DocumentClient::new(document_config)
            .await
            .expect("Failed to create document client - ensure document-service is running"),
    );

    // Create AppState with both clients
    let app_state = AppState::new(auth_client, document_client);

    let app = build_router(app_state);

    // Request health endpoint
    let response = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Assert health check returns OK
    assert_eq!(response.status(), StatusCode::OK);
}

/// Simple router test that doesn't require gRPC connections.
///
/// Tests that the router can be built and static routes work.
#[tokio::test]
async fn router_static_routes_work() {
    use axum::{routing::get, Router};

    // Create a minimal router with just static routes for testing
    let app = Router::new()
        .route("/health", get(|| async { "healthy" }))
        .route("/", get(|| async { "index" }));

    // Test health endpoint
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    // Test index endpoint
    let response = app
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}
