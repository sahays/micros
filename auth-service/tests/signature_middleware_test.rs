use service_core::middleware::rate_limit::{create_client_rate_limiter, create_ip_rate_limiter};
use service_core::middleware::signature::signature_validation_middleware;
use auth_service::{
    config::AuthConfig,
    models::{Client, ClientType},
    services::{EmailService, JwtService, MockBlacklist, MongoDb},
    utils::{hash_password, signature::generate_signature, Password},
    AppState,
};
use axum::{
    body::Body,
    http::{Request, StatusCode},
    middleware::from_fn_with_state,
    routing::post,
    Router,
};
use std::sync::Arc;
use tower::util::ServiceExt;
use uuid::Uuid;

async fn setup_test_config() -> (AuthConfig, String) {
    dotenvy::dotenv().ok();
    let mut config = AuthConfig::from_env().expect("Failed to load environment variables for test");
    let db_name = format!("test_signature_middleware_{}", Uuid::new_v4());
    config.mongodb.database = db_name.clone();
    config.log_level = "error".to_string();
    config.security.require_signatures = true; // Enable signature requirement
    (config, db_name)
}

async fn teardown_test_db(uri: &str, db_name: &str) {
    let client = mongodb::Client::with_uri_str(uri).await.unwrap();
    client.database(db_name).drop(None).await.unwrap();
}

#[tokio::test]
async fn test_signature_middleware() {
    // 1. Setup
    let (config, db_name) = setup_test_config().await;

    let db = MongoDb::connect(&config.mongodb.uri, &config.mongodb.database)
        .await
        .expect("Failed to connect to DB");
    db.initialize_indexes().await.unwrap();

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

    // 2. Build App
    let app = Router::new()
        .route("/protected", post(|| async { "ok" }))
        .layer(from_fn_with_state(
            state.clone(),
            signature_validation_middleware::<AppState>,
        ))
        .with_state(state);

    // 3. Create Test Client
    let client_id = "sig_test_client";
    let signing_secret = "my_signing_secret";
    let client = Client::new(
        client_id.to_string(),
        hash_password(&Password::new("any".to_string()))
            .unwrap()
            .into_string(),
        signing_secret.to_string(),
        "Test App".to_string(),
        ClientType::Service,
        100,
        vec![],
    );
    db.clients().insert_one(&client, None).await.unwrap();

    // 4. Test Case 1: Valid Signature
    let now = chrono::Utc::now().timestamp();
    let nonce = "nonce1";
    let body_str = r#"{"foo":"bar"}"#;
    let signature =
        generate_signature(signing_secret, "POST", "/protected", now, nonce, body_str).unwrap();

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/protected")
                .header("X-Client-ID", client_id)
                .header("X-Timestamp", now.to_string())
                .header("X-Nonce", nonce)
                .header("X-Signature", signature)
                .body(Body::from(body_str))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    // 5. Test Case 2: Replay Attack (Same Nonce)
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/protected")
                .header("X-Client-ID", client_id)
                .header("X-Timestamp", now.to_string())
                .header("X-Nonce", nonce)
                .header(
                    "X-Signature",
                    "any_sig_valid_structurally_but_should_fail_nonce_check",
                )
                .body(Body::from(body_str))
                .unwrap(),
        )
        .await
        .unwrap();

    // It should be 401 Unauthorized due to Replay
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // 6. Test Case 3: Invalid Signature
    let nonce2 = "nonce2";
    let invalid_signature = "aabbcc";
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/protected")
                .header("X-Client-ID", client_id)
                .header("X-Timestamp", now.to_string())
                .header("X-Nonce", nonce2)
                .header("X-Signature", invalid_signature)
                .body(Body::from(body_str))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // 7. Test Case 4: Expired Timestamp
    let old_ts = now - 61;
    let nonce3 = "nonce3";
    let signature_old = generate_signature(
        signing_secret,
        "POST",
        "/protected",
        old_ts,
        nonce3,
        body_str,
    )
    .unwrap();

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/protected")
                .header("X-Client-ID", client_id)
                .header("X-Timestamp", old_ts.to_string())
                .header("X-Nonce", nonce3)
                .header("X-Signature", signature_old)
                .body(Body::from(body_str))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    teardown_test_db(&config.mongodb.uri, &db_name).await;
}