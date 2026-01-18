use auth_service::{
    build_router,
    config::AuthConfig,
    models::{Organization, SanitizedUser, User},
    services::{EmailService, JwtService, MockBlacklist, MongoDb, SecurityAuditService},
    utils::{hash_password, Password},
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
    let security_audit = SecurityAuditService::new(db.clone());

    let state = AppState {
        config: config.clone(),
        db: db.clone(),
        email: email.clone(),
        jwt: jwt.clone(),
        auth_service,
        admin_service,
        security_audit,
        redis: redis.clone(),
        login_rate_limiter: login_limiter,
        register_rate_limiter: register_limiter,
        password_reset_rate_limiter: reset_limiter,
        app_token_rate_limiter: ip_limiter.clone(),
        client_rate_limiter: create_client_rate_limiter(),
        ip_rate_limiter: ip_limiter,
    };

    // 2. Create Organization (required for policy validation)
    let app_id = "test-app-id".to_string();
    let org_id = "test-org-id".to_string();
    let org = Organization::new(app_id.clone(), "Test Org".to_string());
    // Need to set org_id explicitly for the test
    let org = Organization {
        org_id: org_id.clone(),
        ..org
    };
    db.organizations().insert_one(&org, None).await.unwrap();

    // 3. Create User
    let password = "CurrentPassword123";
    let password_hash = hash_password(&Password::new(password.to_string())).unwrap();
    let user = User::new(
        app_id.clone(),
        org_id.clone(),
        "user_test@example.com".to_string(),
        password_hash.into_string(),
        Some("Initial Name".to_string()),
    );
    // Mark user as verified
    let user = User {
        verified: true,
        ..user
    };
    db.users().insert_one(&user, None).await.unwrap();

    let token = jwt
        .generate_access_token(&user.id, &user.app_id, &user.org_id, &user.email)
        .unwrap();

    // 4. Build Router
    let app = build_router(state).await.expect("Failed to build router");

    // 5. Test GET /users/me
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
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
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
                .extension(axum::extract::ConnectInfo(std::net::SocketAddr::from((
                    [127, 0, 0, 1],
                    8080,
                ))))
                .body(Body::from(format!(r#"{{"name": "{}"}}"#, new_name)))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let user_info: SanitizedUser = serde_json::from_slice(&body).unwrap();
    assert_eq!(user_info.name, Some(new_name.to_string()));

    // 6. Test POST /users/me/password
    let new_password = "NewPassword123";
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/users/me/password")
                .header("Authorization", format!("Bearer {}", token))
                .header("Content-Type", "application/json")
                .extension(axum::extract::ConnectInfo(std::net::SocketAddr::from((
                    [127, 0, 0, 1],
                    8080,
                ))))
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
