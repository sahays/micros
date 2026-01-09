use auth_service::{
    build_router,
    config::AuthConfig,
    middleware::{create_client_rate_limiter, create_ip_rate_limiter},
    services::{EmailService, JwtService, MockBlacklist, MongoDb},
    AppState,
};
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use std::sync::Arc;
use tower::util::ServiceExt;
use uuid::Uuid;

async fn setup_test_config() -> (AuthConfig, String) {
    dotenvy::dotenv().ok();
    let mut config = AuthConfig::from_env().expect("Failed to load environment variables for test");
    let db_name = format!("test_auth_bot_{}", Uuid::new_v4());
    config.mongodb.database = db_name.clone();
    config.log_level = "error".to_string();
    (config, db_name)
}

async fn teardown_test_db(uri: &str, db_name: &str) {
    let client = mongodb::Client::with_uri_str(uri).await.unwrap();
    client.database(db_name).drop(None).await.unwrap();
}

async fn create_app(config: AuthConfig) -> axum::Router {
    let db = MongoDb::connect(&config.mongodb.uri, &config.mongodb.database)
        .await
        .expect("Failed to connect to DB");

    let email = EmailService::new(&config.gmail).expect("Failed to create email service");
    let email = Arc::new(email);
    let jwt = JwtService::new(&config.jwt).expect("Failed to create JWT service");
    let redis = Arc::new(MockBlacklist::new());

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
        login_rate_limiter: create_ip_rate_limiter(5, 60),
        register_rate_limiter: create_ip_rate_limiter(5, 60),
        password_reset_rate_limiter: create_ip_rate_limiter(3, 3600),
        app_token_rate_limiter: ip_limiter.clone(),
        client_rate_limiter: create_client_rate_limiter(),
        ip_rate_limiter: ip_limiter,
    };

    build_router(state).await.expect("Failed to build router")
}

#[tokio::test]
async fn test_bot_detection_known_bot() {
    let (config, db_name) = setup_test_config().await;
    let app = create_app(config.clone()).await;

    // Googlebot should be blocked (Score +100)
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/login")
                .header(
                    "User-Agent",
                    "Googlebot/2.1 (+http://www.google.com/bot.html)",
                )
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    teardown_test_db(&config.mongodb.uri, &db_name).await;
}

#[tokio::test]
async fn test_bot_detection_suspicious_browser() {
    let (config, db_name) = setup_test_config().await;
    let app = create_app(config.clone()).await;

    // Claims to be Chrome, but missing standard headers
    // Should NOT be forbidden (Score 70 < 100)
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/login")
                .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/58.0.3029.110 Safari/537.36")
                // No Accept, No Accept-Language
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_ne!(response.status(), StatusCode::FORBIDDEN);

    teardown_test_db(&config.mongodb.uri, &db_name).await;
}

#[tokio::test]
async fn test_bot_detection_clean_browser() {
    let (config, db_name) = setup_test_config().await;
    let app = create_app(config.clone()).await;

    // Legit browser
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/login")
                .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36")
                .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,*/*;q=0.8")
                .header("Accept-Language", "en-US,en;q=0.5")
                .header("Accept-Encoding", "gzip, deflate, br")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_ne!(response.status(), StatusCode::FORBIDDEN);

    teardown_test_db(&config.mongodb.uri, &db_name).await;
}

#[tokio::test]
async fn test_bot_detection_options_allowed() {
    let (config, db_name) = setup_test_config().await;
    let app = create_app(config.clone()).await;

    // OPTIONS request (CORS) - even with Bot UA
    let response = app
        .oneshot(
            Request::builder()
                .method("OPTIONS")
                .uri("/auth/login")
                .header("User-Agent", "Googlebot/2.1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_ne!(response.status(), StatusCode::FORBIDDEN);

    teardown_test_db(&config.mongodb.uri, &db_name).await;
}
