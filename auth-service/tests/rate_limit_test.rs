use auth_service::{
    build_router,
    config::Config,
    middleware::{
        create_ip_rate_limiter, create_login_rate_limiter, create_password_reset_rate_limiter,
    },
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

async fn setup_test_config() -> (Config, String) {
    dotenvy::dotenv().ok();
    let mut config = Config::from_env().expect("Failed to load environment variables for test");
    let db_name = format!("test_auth_rate_limit_{}", Uuid::new_v4());
    config.mongodb.database = db_name.clone();
    config.log_level = "error".to_string();
    (config, db_name)
}

async fn teardown_test_db(uri: &str, db_name: &str) {
    let client = mongodb::Client::with_uri_str(uri).await.unwrap();
    client.database(db_name).drop(None).await.unwrap();
}

#[tokio::test]
async fn test_rate_limit_headers() {
    // 1. Setup
    let (config, db_name) = setup_test_config().await;

    let db = MongoDb::connect(&config.mongodb.uri, &config.mongodb.database)
        .await
        .expect("Failed to connect to DB");

    let email = EmailService::new(&config.gmail).expect("Failed to create email service");
    let jwt = JwtService::new(&config.jwt).expect("Failed to create JWT service");
    let redis = Arc::new(MockBlacklist::new());

    let login_limiter = create_login_rate_limiter(1, 60); // 1 per min
    let reset_limiter = create_password_reset_rate_limiter(3, 3600);
    let ip_limiter = create_ip_rate_limiter(100, 60);

    let state = AppState {
        config: config.clone(),
        db,
        email,
        jwt: jwt.clone(),
        redis,
        login_rate_limiter: login_limiter,
        password_reset_rate_limiter: reset_limiter,
        ip_rate_limiter: ip_limiter,
    };

    // 2. Build Router
    let app = build_router(state).await.expect("Failed to build router");

    // 3. Test Request: Login (should be rate limited after 1 attempt)
    let login_req = || {
        Request::builder()
            .method("POST")
            .uri("/auth/login")
            .header("Content-Type", "application/json")
            .body(Body::from(
                r#"{"email": "test@example.com", "password": "p"}"#,
            ))
            .unwrap()
    };

    // 1st request
    let response = app.clone().oneshot(login_req()).await.unwrap();
    // Might be 401 or 200 depending on DB, but not 429
    assert_ne!(response.status(), StatusCode::TOO_MANY_REQUESTS);

    // 2nd request (rate limited)
    let response = app.oneshot(login_req()).await.unwrap();
    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);

    // Check headers
    assert!(response.headers().contains_key("retry-after"));
    assert!(response.headers().contains_key("x-ratelimit-limit"));

    // Cleanup
    teardown_test_db(&config.mongodb.uri, &db_name).await;
}
