use auth_service::{
    build_router,
    config::Config,
    middleware::{create_login_rate_limiter, create_password_reset_rate_limiter, create_ip_rate_limiter},
    models::{User, SanitizedUser},
    services::{EmailService, JwtService, MongoDb, MockBlacklist},
    utils::{hash_password, Password},
    AppState,
};
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use tower::util::ServiceExt;
use uuid::Uuid;
use std::sync::Arc;

async fn setup_test_config() -> (Config, String) {
    dotenvy::dotenv().ok();
    let mut config = Config::from_env().expect("Failed to load environment variables for test");
    let db_name = format!("test_auth_user_{}", Uuid::new_v4());
    config.mongodb.database = db_name.clone();
    config.log_level = "error".to_string();
    (config, db_name)
}

async fn teardown_test_db(uri: &str, db_name: &str) {
    let client = mongodb::Client::with_uri_str(uri).await.unwrap();
    client.database(db_name).drop(None).await.unwrap();
}

#[tokio::test]
async fn test_user_profile_flow() {
    // 1. Setup
    let (config, db_name) = setup_test_config().await;

    let db = MongoDb::connect(&config.mongodb.uri, &config.mongodb.database)
        .await
        .expect("Failed to connect to DB");
    
    let email_service = EmailService::new(&config.gmail).expect("Failed to create email service");
    let jwt = JwtService::new(&config.jwt).expect("Failed to create JWT service");
    let redis = Arc::new(MockBlacklist::new());
    
    let login_limiter = create_login_rate_limiter(5, 60);
    let reset_limiter = create_password_reset_rate_limiter(3, 3600);
    let ip_limiter = create_ip_rate_limiter(100, 60);

    let state = AppState {
        config: config.clone(),
        db: db.clone(),
        email: email_service,
        jwt: jwt.clone(),
        redis,
        login_rate_limiter: login_limiter,
        password_reset_rate_limiter: reset_limiter,
        ip_rate_limiter: ip_limiter,
    };

    // 2. Create User
    let password = "current_password_123";
    let password_hash = hash_password(&Password::new(password.to_string())).unwrap();
    let user = User {
        id: Uuid::new_v4().to_string(),
        email: "user_test@example.com".to_string(),
        password_hash: password_hash.into_string(),
        name: Some("Initial Name".to_string()),
        verified: true,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    db.users().insert_one(&user, None).await.unwrap();

    let token = jwt.generate_access_token(&user.id, &user.email).unwrap();

    // 3. Build Router
    let app = build_router(state).await.expect("Failed to build router");

    // 4. Test GET /users/me
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/users/me")
                .header("Authorization", format!("Bearer {}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let user_info: SanitizedUser = serde_json::from_slice(&body).unwrap();
    assert_eq!(user_info.email, user.email);
    assert_eq!(user_info.name, user.name);

    // 5. Test PATCH /users/me
    let new_name = "Updated Name";
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri("/users/me")
                .header("Authorization", format!("Bearer {}", token))
                .header("Content-Type", "application/json")
                .body(Body::from(format!(r#"{{"name": "{}"}}"#, new_name)))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let user_info: SanitizedUser = serde_json::from_slice(&body).unwrap();
    assert_eq!(user_info.name, Some(new_name.to_string()));

    // 6. Test POST /users/me/password
    let new_password = "new_password_123";
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/users/me/password")
                .header("Authorization", format!("Bearer {}", token))
                .header("Content-Type", "application/json")
                .body(Body::from(format!(
                    r#"{{"current_password": "{}", "new_password": "{}"}}"#,
                    password, new_password
                )))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    // Cleanup
    teardown_test_db(&config.mongodb.uri, &db_name).await;
}
