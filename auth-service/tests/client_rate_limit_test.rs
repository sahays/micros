use auth_service::{
    config::AuthConfig,
    middleware::app_auth_middleware,
    models::{Client, ClientType},
    services::{
        AppTokenClaims, EmailService, JwtService, MockBlacklist, MongoDb, SecurityAuditService,
    },
    utils::{hash_password, Password},
    AppState,
};
use axum::{
    body::Body,
    http::{Request, StatusCode},
    middleware::from_fn_with_state,
    routing::get,
    Router,
};
use service_core::middleware::rate_limit::{
    client_rate_limit_middleware, create_client_rate_limiter, create_ip_rate_limiter,
};
use std::sync::Arc;
use tower::util::ServiceExt;
use uuid::Uuid;

async fn setup_test_config() -> (AuthConfig, String) {
    dotenvy::dotenv().ok();
    let mut config = AuthConfig::from_env().expect("Failed to load environment variables for test");
    let db_name = format!("test_client_rate_limit_{}", Uuid::new_v4());
    config.mongodb.database = db_name.clone();
    config.log_level = "error".to_string();
    (config, db_name)
}

async fn teardown_test_db(uri: &str, db_name: &str) {
    let client = mongodb::Client::with_uri_str(uri).await.unwrap();
    client.database(db_name).drop(None).await.unwrap();
}

#[tokio::test]
async fn test_client_rate_limiting() {
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
    let security_audit = SecurityAuditService::new(db.clone());

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
        security_audit,
    };

    // 2. Build App with Middlewares
    let app = Router::new()
        .route("/protected", get(|| async { "ok" }))
        .layer(from_fn_with_state(
            state.client_rate_limiter.clone(),
            client_rate_limit_middleware::<AppTokenClaims>,
        ))
        .layer(from_fn_with_state(state.clone(), app_auth_middleware))
        .with_state(state);

    // 3. Create Test Client with low rate limit (2 per min)
    let client_id = "limited_client";
    let client_secret_hash = hash_password(&Password::new("any".to_string())).unwrap();
    let client = Client::new(
        client_id.to_string(),
        client_secret_hash.into_string(),
        "dummy_signing_secret".to_string(),
        "Test App".to_string(),
        ClientType::Service,
        2, // 2 requests per minute
        vec![],
    );
    db.clients().insert_one(&client, None).await.unwrap();

    let token = jwt
        .generate_app_token(client_id, "Test App", vec![], 2)
        .unwrap();

    // 4. First request - OK
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/protected")
                .header("X-App-Token", &token)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // 5. Second request - OK
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/protected")
                .header("X-App-Token", &token)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // 6. Third request - 429
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/protected")
                .header("X-App-Token", &token)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);

    // Check headers
    // assert_eq!(response.headers().get("x-ratelimit-limit").unwrap(), "2");
    // assert_eq!(
    //    response.headers().get("x-ratelimit-remaining").unwrap(),
    //    "0"
    // );
    assert!(response.headers().get("retry-after").is_some());

    // 7. Cleanup
    teardown_test_db(&config.mongodb.uri, &db_name).await;
}
