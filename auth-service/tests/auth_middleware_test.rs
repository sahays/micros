use auth_service::{
    config::Config,
    middleware::{
        auth_middleware, create_ip_rate_limiter, create_login_rate_limiter,
        create_password_reset_rate_limiter,
    },
    services::{EmailService, JwtService, MockBlacklist, MongoDb, TokenBlacklist},
    AppState,
};
use axum::{
    body::Body,
    http::{Request, StatusCode},
    middleware::from_fn_with_state,
    routing::get,
    Router,
};
use std::sync::Arc;
use tower::util::ServiceExt;
use uuid::Uuid;

async fn setup_test_config() -> (Config, String) {
    dotenvy::dotenv().ok();
    let mut config = Config::from_env().expect("Failed to load environment variables for test");
    let db_name = format!("test_auth_middleware_{}", Uuid::new_v4());
    config.mongodb.database = db_name.clone();
    config.log_level = "error".to_string();
    (config, db_name)
}

async fn teardown_test_db(uri: &str, db_name: &str) {
    let client = mongodb::Client::with_uri_str(uri).await.unwrap();
    client.database(db_name).drop(None).await.unwrap();
}

#[tokio::test]
async fn test_auth_middleware() {
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
        jwt: jwt.clone(), // Clone for use in test
        redis: redis.clone(),
        login_rate_limiter: login_limiter,
        password_reset_rate_limiter: reset_limiter,
        app_token_rate_limiter: ip_limiter.clone(),
        ip_rate_limiter: ip_limiter,
    };

    // 2. Build App with Middleware
    let app = Router::new()
        .route("/protected", get(|| async { "protected" }))
        .layer(from_fn_with_state(state.clone(), auth_middleware))
        .with_state(state);

    // 3. Test: Missing Authorization Header
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/protected")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // 4. Test: Invalid Token
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/protected")
                .header("Authorization", "Bearer invalid_token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // 5. Test: Valid Token
    let user_id = "user_123";
    let email = "test@example.com";
    let token = jwt.generate_access_token(user_id, email).unwrap();

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/protected")
                .header("Authorization", format!("Bearer {}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    // 6. Test: Blacklisted Token
    // Generate a new token
    let blacklisted_token = jwt.generate_access_token(user_id, email).unwrap();
    // Parse it to get JTI (we need to peek inside or just use the generated jti if accessible)
    // Since we can't easily peek inside without jwt service methods exposed for it or manually decoding,
    // let's rely on validation parsing it.
    // Actually we need the JTI to blacklist it.
    let claims = jwt.validate_access_token(&blacklisted_token).unwrap();

    // Blacklist the token
    redis
        .blacklist_token(&claims.jti, 3600)
        .await
        .expect("Failed to blacklist");

    let response = app
        .oneshot(
            Request::builder()
                .uri("/protected")
                .header("Authorization", format!("Bearer {}", blacklisted_token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // Cleanup
    teardown_test_db(&config.mongodb.uri, &db_name).await;
}
