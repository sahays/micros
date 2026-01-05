use auth_service::{
    config::Config,
    middleware::{admin_auth_middleware, create_client_rate_limiter, create_ip_rate_limiter},
    models::{Client, ClientType},
    services::{JwtService, MockBlacklist, MockEmailService, MongoDb},
    utils::{hash_password, Password},
    AppState,
};
use axum::{
    body::Body,
    http::{Request, StatusCode},
    middleware::from_fn_with_state,
    routing::post,
    Router,
};
use std::sync::Arc;
use tower::util::ServiceExt;
use uuid::Uuid;

async fn setup_test_config() -> (Config, String) {
    dotenvy::dotenv().ok();
    let mut config = Config::from_env().expect("Failed to load environment variables for test");
    let db_name = format!("test_rotation_revocation_{}", Uuid::new_v4());
    config.mongodb.database = db_name.clone();
    config.log_level = "error".to_string();
    (config, db_name)
}

async fn teardown_test_db(uri: &str, db_name: &str) {
    let client = mongodb::Client::with_uri_str(uri).await.unwrap();
    client.database(db_name).drop(None).await.unwrap();
}

#[tokio::test]
async fn test_client_rotation_and_revocation() {
    // 1. Setup
    let (config, db_name) = setup_test_config().await;

    let db = MongoDb::connect(&config.mongodb.uri, &config.mongodb.database)
        .await
        .expect("Failed to connect to DB");
    db.initialize_indexes().await.unwrap();

    let email = MockEmailService;
    let email = Arc::new(email);
    let jwt = JwtService::new(&config.jwt).expect("Failed to create JWT service");
    let redis = Arc::new(MockBlacklist::new());

    let login_limiter = create_ip_rate_limiter(5, 60);
    let register_limiter = create_ip_rate_limiter(5, 60);
    let reset_limiter = create_ip_rate_limiter(3, 3600);
    let ip_limiter = create_ip_rate_limiter(100, 60);

    let auth_service = auth_service::services::AuthService::new(
        db.clone(),
        email.clone(),
        jwt.clone(),
        redis.clone(),
    );
    let admin_service = auth_service::services::admin::AdminService::new(db.clone(), redis.clone());

    let state = AppState {
        config: config.clone(),
        db: db.clone(),
        email: email.clone(),
        jwt,
        auth_service,
        admin_service,
        redis,
        login_rate_limiter: login_limiter,
        register_rate_limiter: register_limiter,
        password_reset_rate_limiter: reset_limiter,
        app_token_rate_limiter: ip_limiter.clone(),
        client_rate_limiter: create_client_rate_limiter(),
        ip_rate_limiter: ip_limiter,
    };

    // 2. Build App
    let admin_routes = Router::new()
        .route(
            "/auth/admin/clients/:client_id/rotate",
            post(auth_service::handlers::admin::rotate_client_secret),
        )
        .route(
            "/auth/admin/clients/:client_id",
            axum::routing::delete(auth_service::handlers::admin::revoke_client),
        )
        .layer(from_fn_with_state(state.clone(), admin_auth_middleware));

    let app = Router::new()
        .route(
            "/auth/app/token",
            post(auth_service::handlers::app::app_token),
        )
        .merge(admin_routes)
        .with_state(state);

    // 3. Create Test Client
    let client_id = "rotation_test_client";
    let old_secret = "old_secret_123";
    let old_secret_hash = hash_password(&Password::new(old_secret.to_string())).unwrap();
    let client = Client::new(
        client_id.to_string(),
        old_secret_hash.into_string(),
        "dummy_signing_secret".to_string(),
        "Test App".to_string(),
        ClientType::Service,
        100,
        vec![],
    );
    db.clients().insert_one(&client, None).await.unwrap();

    // 4. Rotate Secret (Admin)
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/auth/admin/clients/{}/rotate", client_id))
                .header("X-Admin-Api-Key", &config.security.admin_api_key)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let new_secret = body_json["new_client_secret"].as_str().unwrap();

    // 5. Verify: Both secrets work for token exchange (grace period)

    // Old secret works
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/app/token")
                .header("Content-Type", "application/json")
                .body(Body::from(format!(
                    r#"{{"client_id": "{}", "client_secret": "{}", "grant_type": "client_credentials"}}"#,
                    client_id, old_secret
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // New secret works
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/app/token")
                .header("Content-Type", "application/json")
                .body(Body::from(format!(
                    r#"{{"client_id": "{}", "client_secret": "{}", "grant_type": "client_credentials"}}"#,
                    client_id, new_secret
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // 6. Revoke Client (Admin)
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/auth/admin/clients/{}", client_id))
                .header("X-Admin-Api-Key", &config.security.admin_api_key)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // 7. Verify: Client cannot get token anymore
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/app/token")
                .header("Content-Type", "application/json")
                .body(Body::from(format!(
                    r#"{{"client_id": "{}", "client_secret": "{}", "grant_type": "client_credentials"}}"#,
                    client_id, new_secret
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN); // Or UNAUTHORIZED depending on implementation

    // Cleanup
    teardown_test_db(&config.mongodb.uri, &db_name).await;
}
