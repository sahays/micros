use auth_service::{
    build_router,
    config::Config,
    middleware::{
        create_client_rate_limiter, create_ip_rate_limiter, create_login_rate_limiter, create_password_reset_rate_limiter,
    },
    services::{EmailService, JwtService, MockBlacklist, MongoDb, TokenBlacklist},
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
    let db_name = format!("test_auth_introspect_{}", Uuid::new_v4());
    config.mongodb.database = db_name.clone();
    config.log_level = "error".to_string();
    (config, db_name)
}

async fn teardown_test_db(uri: &str, db_name: &str) {
    let client = mongodb::Client::with_uri_str(uri).await.unwrap();
    client.database(db_name).drop(None).await.unwrap();
}

#[tokio::test]
async fn test_introspection_flow() {
    // 1. Setup
    let (config, db_name) = setup_test_config().await;

    let db = MongoDb::connect(&config.mongodb.uri, &config.mongodb.database)
        .await
        .expect("Failed to connect to DB");

    let email = EmailService::new(&config.gmail).expect("Failed to create email service");
    let jwt = JwtService::new(&config.jwt).expect("Failed to create JWT service");
    let redis = Arc::new(MockBlacklist::new());

    let login_limiter = create_login_rate_limiter(5, 60);
    let reset_limiter = create_password_reset_rate_limiter(3, 3600);
    let ip_limiter = create_ip_rate_limiter(100, 60);

    let state = AppState {
        config: config.clone(),
        db,
        email,
        jwt: jwt.clone(),
        redis: redis.clone(),
        login_rate_limiter: login_limiter,
        password_reset_rate_limiter: reset_limiter,
        app_token_rate_limiter: ip_limiter.clone(),
        client_rate_limiter: create_client_rate_limiter(),
        ip_rate_limiter: ip_limiter,
    };

    // 2. Build Router
    let app = build_router(state).await.expect("Failed to build router");

    // 3. Generate Valid Token
    let user_id = "user_123";
    let user_email = "test@example.com";
    let token = jwt.generate_access_token(user_id, user_email).unwrap();

    // 4. Test Active Token
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/introspect")
                .header("Content-Type", "application/json")
                .body(Body::from(format!(r#"{{"token": "{}"}}"#, token)))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body_json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert!(body_json["active"].as_bool().unwrap());
    assert_eq!(body_json["sub"], user_id);
    assert_eq!(body_json["email"], user_email);

    // 5. Test Blacklisted Token
    let claims = jwt.validate_access_token(&token).unwrap();
    redis.blacklist_token(&claims.jti, 3600).await.unwrap();

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/introspect")
                .header("Content-Type", "application/json")
                .body(Body::from(format!(r#"{{"token": "{}"}}"#, token)))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body_json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert!(!body_json["active"].as_bool().unwrap());
    assert!(body_json["sub"].is_null());

    // 6. Test Invalid Token
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/introspect")
                .header("Content-Type", "application/json")
                .body(Body::from(r#"{"token": "invalid_token"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body_json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert!(!body_json["active"].as_bool().unwrap());

    // Cleanup
    teardown_test_db(&config.mongodb.uri, &db_name).await;
}
