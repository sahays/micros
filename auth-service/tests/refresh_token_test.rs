use auth_service::{
    build_router,
    config::Config,
    init_tracing,
    middleware::{
        create_ip_rate_limiter, create_login_rate_limiter, create_password_reset_rate_limiter,
    },
    models::{RefreshToken, User},
    services::{EmailService, JwtService, MockBlacklist, MongoDb, TokenBlacklist},
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
    let db_name = format!("test_auth_refresh_{}", Uuid::new_v4());
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
async fn test_refresh_token_flow() {
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
    let jwt = JwtService::new(&config.jwt).expect("Failed to create JWT service");
    let redis = Arc::new(MockBlacklist::new());

    let login_limiter = create_login_rate_limiter(5, 60);
    let reset_limiter = create_password_reset_rate_limiter(3, 3600);
    let ip_limiter = create_ip_rate_limiter(100, 60);

    let state = AppState {
        config: config.clone(),
        db: db.clone(),
        email,
        jwt: jwt.clone(),
        redis: redis.clone(),
        login_rate_limiter: login_limiter,
        password_reset_rate_limiter: reset_limiter,
        ip_rate_limiter: ip_limiter,
    };

    // 3. Create Test User
    let user = User {
        id: Uuid::new_v4().to_string(),
        email: "test_refresh@example.com".to_string(),
        password_hash: "hash".to_string(),
        name: Some("Test User".to_string()),
        verified: true,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    db.users().insert_one(&user, None).await.unwrap();

    // 4. Generate Initial Refresh Token
    let refresh_token_id = Uuid::new_v4().to_string();
    let refresh_token_str = jwt
        .generate_refresh_token(&user.id, &refresh_token_id)
        .unwrap();

    let refresh_token_model =
        RefreshToken::new_with_id(refresh_token_id, user.id.clone(), &refresh_token_str, 7);
    db.refresh_tokens()
        .insert_one(&refresh_token_model, None)
        .await
        .unwrap();

    // 5. Build Router
    let app = build_router(state).await.expect("Failed to build router");

    // 6. Test Refresh
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/refresh")
                .header("Content-Type", "application/json")
                .body(Body::from(format!(
                    r#"{{"refresh_token": "{}"}}"#,
                    refresh_token_str
                )))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body_json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    let new_access_token = body_json["access_token"].as_str().unwrap();
    let new_refresh_token = body_json["refresh_token"].as_str().unwrap();

    // 7. Verify Tokens
    assert!(jwt.validate_access_token(new_access_token).is_ok());
    assert_ne!(refresh_token_str, new_refresh_token);

    // 8. Verify Old Token is Revoked
    let old_token_in_db = db
        .refresh_tokens()
        .find_one(doc! { "_id": &refresh_token_model.id }, None)
        .await
        .unwrap()
        .unwrap();
    assert!(old_token_in_db.revoked);

    // 10. Test Logout
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/logout")
                .header("Content-Type", "application/json")
                .header("Authorization", format!("Bearer {}", new_access_token))
                .body(Body::from(format!(
                    r#"{{"refresh_token": "{}"}}"#,
                    new_refresh_token
                )))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    // Verify new refresh token is also revoked
    let new_claims = jwt.validate_refresh_token(new_refresh_token).unwrap();
    let new_token_in_db = db
        .refresh_tokens()
        .find_one(doc! { "_id": &new_claims.jti }, None)
        .await
        .unwrap()
        .unwrap();
    assert!(new_token_in_db.revoked);

    // Verify access token is blacklisted
    let access_claims = jwt.validate_access_token(new_access_token).unwrap();
    assert!(redis.is_blacklisted(&access_claims.jti).await.unwrap());

    // 11. Cleanup
    teardown_test_db(&config.mongodb.uri, &db_name).await;
}
