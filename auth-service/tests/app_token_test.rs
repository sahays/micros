use auth_service::{
    build_router,
    config::Config,
    middleware::{
        create_client_rate_limiter, create_ip_rate_limiter, create_login_rate_limiter,
        create_password_reset_rate_limiter,
    },
    models::{Client, ClientType},
    services::{EmailService, JwtService, MockBlacklist, MongoDb},
    utils::{hash_password, Password},
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
    let db_name = format!("test_auth_app_token_{}", Uuid::new_v4());
    config.mongodb.database = db_name.clone();
    config.log_level = "error".to_string();
    (config, db_name)
}

async fn teardown_test_db(uri: &str, db_name: &str) {
    let client = mongodb::Client::with_uri_str(uri).await.unwrap();
    client.database(db_name).drop(None).await.unwrap();
}

#[tokio::test]
async fn test_app_token_success() {
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
        redis,
        login_rate_limiter: login_limiter,
        password_reset_rate_limiter: reset_limiter,
        app_token_rate_limiter: ip_limiter.clone(),
        client_rate_limiter: create_client_rate_limiter(),
        ip_rate_limiter: ip_limiter,
    };

    // 3. Create Test Client
    let client_id = "test_client_id";
    let client_secret = "test_client_secret";
    let client_secret_hash = hash_password(&Password::new(client_secret.to_string())).unwrap();

    let client = Client::new(
        client_id.to_string(),
        client_secret_hash.into_string(),
        "Test App".to_string(),
        ClientType::Service,
        100,
        vec!["http://localhost".to_string()],
    );
    db.clients().insert_one(&client, None).await.unwrap();

    // 4. Build Router
    let app = build_router(state).await.expect("Failed to build router");

    // 5. Perform Token Request
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/app/token")
                .header("Content-Type", "application/json")
                .body(Body::from(format!(
                    r#"{{"client_id": "{}", "client_secret": "{}", "grant_type": "client_credentials"}}"#,
                    client_id, client_secret
                )))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    // 6. Verify Token
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body_json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    let access_token = body_json["access_token"]
        .as_str()
        .expect("access_token missing");
    assert_eq!(body_json["token_type"], "Bearer");
    assert_eq!(body_json["expires_in"], 3600);

    // Validate claims
    let claims = jwt
        .validate_app_token(access_token)
        .expect("Invalid app token");
    assert_eq!(claims.client_id, client_id);
    assert_eq!(claims.sub, client_id);
    assert_eq!(claims.name, "Test App");
    assert_eq!(claims.typ, "app");

    // 7. Cleanup
    teardown_test_db(&config.mongodb.uri, &db_name).await;
}

#[tokio::test]
async fn test_app_token_invalid_secret() {
    // 1. Setup
    let (config, db_name) = setup_test_config().await;

    // 2. Setup Services
    let db = MongoDb::connect(&config.mongodb.uri, &config.mongodb.database)
        .await
        .expect("Failed to connect to DB");
    db.initialize_indexes()
        .await
        .expect("Failed to init indexes");

    let state = AppState {
        config: config.clone(),
        db: db.clone(),
        email: EmailService::new(&config.gmail).unwrap(),
        jwt: JwtService::new(&config.jwt).unwrap(),
        redis: Arc::new(MockBlacklist::new()),
        login_rate_limiter: create_login_rate_limiter(5, 60),
        password_reset_rate_limiter: create_password_reset_rate_limiter(3, 3600),
        app_token_rate_limiter: create_ip_rate_limiter(10, 60),
        client_rate_limiter: create_client_rate_limiter(),
        ip_rate_limiter: create_ip_rate_limiter(100, 60),
    };

    // 3. Create Test Client
    let client_id = "test_client_id_invalid";
    let client_secret = "test_client_secret";
    let client_secret_hash = hash_password(&Password::new(client_secret.to_string())).unwrap();

    let client = Client::new(
        client_id.to_string(),
        client_secret_hash.into_string(),
        "Test App".to_string(),
        ClientType::Service,
        100,
        vec![],
    );
    db.clients().insert_one(&client, None).await.unwrap();

    // 4. Build Router
    let app = build_router(state).await.expect("Failed to build router");

    // 5. Perform Token Request with wrong secret
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/app/token")
                .header("Content-Type", "application/json")
                .body(Body::from(format!(
                    r#"{{"client_id": "{}", "client_secret": "wrong_secret", "grant_type": "client_credentials"}}"#,
                    client_id
                )))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // 7. Cleanup
    teardown_test_db(&config.mongodb.uri, &db_name).await;
}

#[tokio::test]
async fn test_app_token_invalid_grant_type() {
    // 1. Setup
    let (config, db_name) = setup_test_config().await;

    // 2. Setup Services
    let db = MongoDb::connect(&config.mongodb.uri, &config.mongodb.database)
        .await
        .expect("Failed to connect to DB");

    let state = AppState {
        config: config.clone(),
        db: db.clone(),
        email: EmailService::new(&config.gmail).unwrap(),
        jwt: JwtService::new(&config.jwt).unwrap(),
        redis: Arc::new(MockBlacklist::new()),
        login_rate_limiter: create_login_rate_limiter(5, 60),
        password_reset_rate_limiter: create_password_reset_rate_limiter(3, 3600),
        app_token_rate_limiter: create_ip_rate_limiter(10, 60),
        client_rate_limiter: create_client_rate_limiter(),
        ip_rate_limiter: create_ip_rate_limiter(100, 60),
    };

    // 4. Build Router
    let app = build_router(state).await.expect("Failed to build router");

    // 5. Perform Token Request with wrong grant_type
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/app/token")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    r#"{"client_id": "any", "client_secret": "any", "grant_type": "password"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    // 7. Cleanup
    teardown_test_db(&config.mongodb.uri, &db_name).await;
}
