use auth_service::{
    build_router,
    config::Config,
    middleware::{create_client_rate_limiter, create_ip_rate_limiter},
    services::{JwtService, MockBlacklist, MockEmailService, MongoDb},
    AppState,
};
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use mongodb::bson::doc;
use std::sync::Arc;
use tower::util::ServiceExt;
use uuid::Uuid;

async fn setup_test_config() -> (Config, String) {
    dotenvy::dotenv().ok();
    let mut config = Config::from_env().expect("Failed to load environment variables for test");
    let db_name = format!("test_auth_security_{}", Uuid::new_v4());
    config.mongodb.database = db_name.clone();
    config.log_level = "error".to_string();
    // Set explicit allowed origin for CORS test
    config.security.allowed_origins = vec!["http://allowed.com".to_string()];
    (config, db_name)
}

async fn teardown_test_db(uri: &str, db_name: &str) {
    let client = mongodb::Client::with_uri_str(uri).await.unwrap();
    client.database(db_name).drop(None).await.unwrap();
}

#[tokio::test]
async fn test_security_headers_and_cors() {
    let (config, db_name) = setup_test_config().await;
    let db = MongoDb::connect(&config.mongodb.uri, &config.mongodb.database)
        .await
        .unwrap();
    let state = AppState {
        config: config.clone(),
        db: db.clone(),
        email: Arc::new(MockEmailService),
        jwt: JwtService::new(&config.jwt).unwrap(),
        redis: Arc::new(MockBlacklist::new()),
        login_rate_limiter: create_ip_rate_limiter(100, 60),
        register_rate_limiter: create_ip_rate_limiter(100, 60),
        password_reset_rate_limiter: create_ip_rate_limiter(100, 60),
        app_token_rate_limiter: create_ip_rate_limiter(100, 60),
        client_rate_limiter: create_client_rate_limiter(),
        ip_rate_limiter: create_ip_rate_limiter(1000, 60),
    };
    let app = build_router(state).await.unwrap();

    // 1. Test Security Headers
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

    let headers = response.headers();
    assert_eq!(headers["x-content-type-options"], "nosniff");
    assert_eq!(headers["x-frame-options"], "DENY");
    assert!(headers.contains_key("content-security-policy"));

    // 2. Test CORS - Allowed Origin
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("OPTIONS")
                .uri("/health")
                .header("Origin", "http://allowed.com")
                .header("Access-Control-Request-Method", "GET")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers()["access-control-allow-origin"],
        "http://allowed.com"
    );

    // 3. Test CORS - Disallowed Origin
    let response = app
        .oneshot(
            Request::builder()
                .method("OPTIONS")
                .uri("/health")
                .header("Origin", "http://malicious.com")
                .header("Access-Control-Request-Method", "GET")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(!response
        .headers()
        .contains_key("access-control-allow-origin"));

    teardown_test_db(&config.mongodb.uri, &db_name).await;
}

#[tokio::test]
async fn test_input_validation_and_audit_logging() {
    let (config, db_name) = setup_test_config().await;
    let db = MongoDb::connect(&config.mongodb.uri, &config.mongodb.database)
        .await
        .unwrap();
    db.initialize_indexes().await.unwrap();

    let state = AppState {
        config: config.clone(),
        db: db.clone(),
        email: Arc::new(MockEmailService),
        jwt: JwtService::new(&config.jwt).unwrap(),
        redis: Arc::new(MockBlacklist::new()),
        login_rate_limiter: create_ip_rate_limiter(100, 60),
        register_rate_limiter: create_ip_rate_limiter(100, 60),
        password_reset_rate_limiter: create_ip_rate_limiter(100, 60),
        app_token_rate_limiter: create_ip_rate_limiter(100, 60),
        client_rate_limiter: create_client_rate_limiter(),
        ip_rate_limiter: create_ip_rate_limiter(1000, 60),
    };
    let app = build_router(state).await.unwrap();

    // 1. Test Password Length Validation (Short password)
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/register")
                .header("Content-Type", "application/json")
                .extension(axum::extract::ConnectInfo(std::net::SocketAddr::from((
                    [127, 0, 0, 1],
                    8080,
                ))))
                .body(Body::from(
                    r#"{"email":"valid@example.com", "password":"short", "name":"Test"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    // 2. Test Audit Logging Persistence after registration
    let email = "audit_test@example.com";
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/register")
                .header("Content-Type", "application/json")
                .extension(axum::extract::ConnectInfo(std::net::SocketAddr::from((
                    [127, 0, 0, 1],
                    8080,
                ))))
                .body(Body::from(format!(
                    r#"{{"email":"{}", "password":"longenoughpassword", "name":"Audit Test"}}"#,
                    email
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);

    // Give a moment for async logging
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let audit_log = db
        .audit_logs()
        .find_one(doc! { "event_type": "user_registration" }, None)
        .await
        .unwrap();
    assert!(audit_log.is_some());
    let log = audit_log.unwrap();
    assert_eq!(log.endpoint, "/auth/register");
    assert_eq!(log.method, "POST");

    teardown_test_db(&config.mongodb.uri, &db_name).await;
}

#[tokio::test]
async fn test_endpoint_specific_rate_limiting() {
    let (config, db_name) = setup_test_config().await;
    let db = MongoDb::connect(&config.mongodb.uri, &config.mongodb.database)
        .await
        .unwrap();

    // Set tight limit for registration
    let state = AppState {
        config: config.clone(),
        db: db.clone(),
        email: Arc::new(MockEmailService),
        jwt: JwtService::new(&config.jwt).unwrap(),
        redis: Arc::new(MockBlacklist::new()),
        login_rate_limiter: create_ip_rate_limiter(100, 60),
        register_rate_limiter: create_ip_rate_limiter(1, 60), // 1 per min
        password_reset_rate_limiter: create_ip_rate_limiter(100, 60),
        app_token_rate_limiter: create_ip_rate_limiter(100, 60),
        client_rate_limiter: create_client_rate_limiter(),
        ip_rate_limiter: create_ip_rate_limiter(1000, 60),
    };
    let app = build_router(state).await.unwrap();

    let reg_req = || {
        Request::builder()
            .method("POST")
            .uri("/auth/register")
            .header("Content-Type", "application/json")
            .extension(axum::extract::ConnectInfo(std::net::SocketAddr::from((
                [127, 0, 0, 1],
                8080,
            ))))
            .body(Body::from(
                r#"{"email":"ratelimit@example.com", "password":"password123"}"#,
            ))
            .unwrap()
    };

    // 1st request
    let response = app.clone().oneshot(reg_req()).await.unwrap();
    assert_ne!(response.status(), StatusCode::TOO_MANY_REQUESTS);

    // 2nd request (rate limited)
    let response = app.oneshot(reg_req()).await.unwrap();
    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);

    teardown_test_db(&config.mongodb.uri, &db_name).await;
}
