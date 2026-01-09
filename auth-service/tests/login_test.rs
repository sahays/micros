use service_core::middleware::rate_limit::{create_client_rate_limiter, create_ip_rate_limiter};
use auth_service::{
    build_router,
    config::AuthConfig,
    models::{RefreshToken, User},
    services::{EmailService, JwtService, MockBlacklist, MongoDb},
    utils::{hash_password, Password},
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

async fn setup_test_config() -> (AuthConfig, String) {
    dotenvy::dotenv().ok();
    let mut config = AuthConfig::from_env().expect("Failed to load environment variables for test");
    let db_name = format!("test_auth_login_{}", Uuid::new_v4());
    config.mongodb.database = db_name.clone();
    config.log_level = "error".to_string();
    (config, db_name)
}

async fn teardown_test_db(uri: &str, db_name: &str) {
    let client = mongodb::Client::with_uri_str(uri).await.unwrap();
    client.database(db_name).drop(None).await.unwrap();
}

#[tokio::test]
async fn test_login_creates_hashed_refresh_token() {
    // 1. Setup
    let (config, db_name) = setup_test_config().await;

    // 2. Setup Services
    let db = MongoDb::connect(&config.mongodb.uri, &config.mongodb.database)
        .await
        .expect("Failed to connect to DB");
    db.initialize_indexes()
        .await
        .expect("Failed to init indexes");

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
    // 3. Create Test User
    let password = "test_password_123";
    let password_hash = hash_password(&Password::new(password.to_string())).unwrap();
    let user = User {
        id: Uuid::new_v4().to_string(),
        email: "test_login@example.com".to_string(),
        password_hash: password_hash.into_string(),
        name: Some("Test User".to_string()),
        verified: true,
        google_id: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    db.users().insert_one(&user, None).await.unwrap();

    // 4. Build Router
    let app = build_router(state).await.expect("Failed to build router");

    // 5. Perform Login
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/login")
                .header("Content-Type", "application/json")
                .extension(axum::extract::ConnectInfo(std::net::SocketAddr::from((
                    [127, 0, 0, 1],
                    8080,
                ))))
                .body(Body::from(format!(
                    r#"{{"email": "{}", "password": "{}"}}"#,
                    user.email, password
                )))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    // 6. Verify Refresh Token in DB
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let refresh_token_str = body_json["refresh_token"].as_str().unwrap();

    // Find token in DB by user_id
    let stored_token = db
        .refresh_tokens()
        .find_one(doc! { "user_id": &user.id }, None)
        .await
        .unwrap()
        .expect("Refresh token not found in DB");

    // Verify hashing: stored hash should NOT equal raw token string
    assert_ne!(stored_token.token_hash, refresh_token_str);

    // Verify correct hash
    let expected_hash = RefreshToken::hash_token(refresh_token_str);
    assert_eq!(stored_token.token_hash, expected_hash);

    // 7. Cleanup
    teardown_test_db(&config.mongodb.uri, &db_name).await;
}
