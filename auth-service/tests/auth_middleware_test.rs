use auth_service::{
    build_router,
    config::Config,
    middleware::{create_login_rate_limiter, create_password_reset_rate_limiter, auth_middleware},
    services::{EmailService, JwtService, MongoDb},
    AppState,
};
use axum::{
    body::Body,
    http::{Request, StatusCode},
    middleware::from_fn_with_state,
    routing::get,
    Router,
};
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
    
    let login_limiter = create_login_rate_limiter(5, 60);
    let reset_limiter = create_password_reset_rate_limiter(3, 3600);

    let state = AppState {
        config: config.clone(),
        db,
        email,
        jwt: jwt.clone(), // Clone for use in test
        login_rate_limiter: login_limiter,
        password_reset_rate_limiter: reset_limiter,
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

    // Cleanup
    teardown_test_db(&config.mongodb.uri, &db_name).await;
}
