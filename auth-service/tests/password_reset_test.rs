use auth_service::{
    build_router,
    config::AuthConfig,
    services::{EmailService, JwtService, MockBlacklist, MongoDb},
    AppState,
};
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use service_core::middleware::rate_limit::{create_client_rate_limiter, create_ip_rate_limiter};
use std::sync::Arc;
use tower::util::ServiceExt;
use uuid::Uuid;

// Helper to setup test config with a unique database
async fn setup_test_config() -> (AuthConfig, String) {
    // Load .env if available
    dotenvy::dotenv().ok();

    // Load config from environment
    let mut config = AuthConfig::from_env().expect("Failed to load environment variables for test");

    // Override database name with a random one for isolation
    let db_name = format!("test_auth_{}", Uuid::new_v4());
    config.mongodb.database = db_name.clone();
    config.log_level = "debug".to_string(); // Use debug for more info

    // Initialize tracing if not already initialized
    let _ = tracing_subscriber::fmt()
        .with_env_filter("debug")
        .try_init();

    (config, db_name)
}

async fn teardown_test_db(uri: &str, db_name: &str) {
    let client = mongodb::Client::with_uri_str(uri).await.unwrap();
    client.database(db_name).drop(None).await.unwrap();
}

#[tokio::test]
async fn test_password_reset_flow() {
    // 1. Setup
    let (config, db_name) = setup_test_config().await;

    // 2. Setup Services
    let db = MongoDb::connect(&config.mongodb.uri, &config.mongodb.database)
        .await
        .expect("Failed to connect to DB");

    // Ensure indexes are created
    db.initialize_indexes()
        .await
        .expect("Failed to init indexes");

    // Note: EmailService requires valid Gmail creds in env.
    // In a real CI, you might mock this, but for now we assume env is set.
    let email = EmailService::new(&config.gmail).expect("Failed to create email service");
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

    // 3. Build Router
    let app = build_router(state).await.expect("Failed to build router");

    // 4. Test Case 1: Password Reset Request for non-existent user
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/password-reset/request")
                .header("Content-Type", "application/json")
                .extension(axum::extract::ConnectInfo(std::net::SocketAddr::from((
                    [127, 0, 0, 1],
                    8080,
                ))))
                .body(Body::from(r#"{"email": "nonexistent@example.com"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    // Expect 200 OK (Security: prevent enumeration)
    assert_eq!(response.status(), StatusCode::OK);

    // 5. Cleanup
    teardown_test_db(&config.mongodb.uri, &db_name).await;
}
