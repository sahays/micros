use auth_service::{
    build_router,
    config::Config,
    middleware::{create_login_rate_limiter, create_password_reset_rate_limiter, create_ip_rate_limiter},
    services::{EmailService, JwtService, MongoDb, MockBlacklist},
    AppState,
    handlers::admin::CreateClientResponse,
    models::{Client, ClientType},
};
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use tower::util::ServiceExt;
use uuid::Uuid;
use std::sync::Arc;
use mongodb::bson::doc;

async fn setup_test_config() -> (Config, String) {
    dotenvy::dotenv().ok();
    // We might fail if ADMIN_API_KEY is not in env, so we handle that or mock it
    // But Config::from_env() checks for it.
    // For test, we can set the env var before calling from_env, or mock the config struct manually.
    std::env::set_var("ADMIN_API_KEY", "test_admin_key");
    
    let mut config = Config::from_env().expect("Failed to load environment variables for test");
    let db_name = format!("test_auth_admin_client_{}", Uuid::new_v4());
    config.mongodb.database = db_name.clone();
    config.log_level = "error".to_string();
    (config, db_name)
}

async fn teardown_test_db(uri: &str, db_name: &str) {
    let client = mongodb::Client::with_uri_str(uri).await.unwrap();
    client.database(db_name).drop(None).await.unwrap();
}

#[tokio::test]
async fn test_create_client_flow() {
    // 1. Setup
    let (config, db_name) = setup_test_config().await;

    let db = MongoDb::connect(&config.mongodb.uri, &config.mongodb.database)
        .await
        .expect("Failed to connect to DB");
    
    // Initialize indexes (important for unique client_id)
    db.initialize_indexes().await.expect("Failed to initialize indexes");
    
    let email = EmailService::new(&config.gmail).expect("Failed to create email service");
    let jwt = JwtService::new(&config.jwt).expect("Failed to create JWT service");
    let redis = Arc::new(MockBlacklist::new());
    
    let login_limiter = create_login_rate_limiter(5, 60);
    let reset_limiter = create_password_reset_rate_limiter(3, 3600);
    let ip_limiter = create_ip_rate_limiter(100, 60);

    let state = AppState {
        config: config.clone(),
        db: db.clone(),
        email,
        jwt,
        redis,
        login_rate_limiter: login_limiter,
        password_reset_rate_limiter: reset_limiter,
        ip_rate_limiter: ip_limiter,
    };

    let app = build_router(state).await.expect("Failed to build router");

    // 2. Test: Missing API Key
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/admin/clients")
                .header("Content-Type", "application/json")
                .body(Body::from(r#"{
                    "app_name": "Test App",
                    "app_type": "web",
                    "rate_limit_per_min": 100,
                    "allowed_origins": ["http://localhost:8080"]
                }"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // 3. Test: Valid API Key
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/admin/clients")
                .header("Content-Type", "application/json")
                .header("X-Admin-Api-Key", "test_admin_key")
                .body(Body::from(r#"{
                    "app_name": "Test App",
                    "app_type": "web",
                    "rate_limit_per_min": 100,
                    "allowed_origins": ["http://localhost:8080"]
                }"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let client_res: CreateClientResponse = serde_json::from_slice(&body).unwrap();

    assert!(!client_res.client_id.is_empty());
    assert!(!client_res.client_secret.is_empty());
    assert_eq!(client_res.app_name, "Test App");
    assert_eq!(client_res.app_type, ClientType::Web);

    // 4. Verify DB
    let stored_client = db
        .clients()
        .find_one(doc! { "client_id": &client_res.client_id }, None)
        .await
        .unwrap()
        .expect("Client not found in DB");

    assert_eq!(stored_client.app_name, "Test App");
    assert_ne!(stored_client.client_secret_hash, client_res.client_secret); // Hash should not equal plain secret
    assert!(stored_client.client_secret_hash.starts_with("$argon2"));

    // Cleanup
    teardown_test_db(&config.mongodb.uri, &db_name).await;
}
