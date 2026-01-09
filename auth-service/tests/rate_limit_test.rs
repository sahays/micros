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

async fn setup_test_config() -> (AuthConfig, String) {
    dotenvy::dotenv().ok();
    let mut config = AuthConfig::from_env().expect("Failed to load environment variables for test");
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
    let email = Arc::new(email);
    let jwt = JwtService::new(&config.jwt).expect("Failed to create JWT service");
    let redis = Arc::new(MockBlacklist::new());

    let login_limiter = create_ip_rate_limiter(1, 60); // 1 per min
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
        jwt: jwt.clone(),
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

    // 2. Build Router
    let app = build_router(state).await.expect("Failed to build router");

    // 3. Test Request: Login (should be rate limited after 1 attempt)
    let login_req = || {
        Request::builder()
            .method("POST")
            .uri("/auth/login")
            .header("Content-Type", "application/json")
            .extension(axum::extract::ConnectInfo(std::net::SocketAddr::from((
                [127, 0, 0, 1],
                8080,
            ))))
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
    // assert!(response.headers().contains_key("x-ratelimit-limit"));

    // Cleanup
    teardown_test_db(&config.mongodb.uri, &db_name).await;
}
