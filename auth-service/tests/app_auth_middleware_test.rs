use auth_service::{
    config::Config,
    middleware::{
        app_auth_middleware, create_client_rate_limiter, create_ip_rate_limiter, create_login_rate_limiter,
        create_password_reset_rate_limiter,
    },
    models::{Client, ClientType},
    services::{EmailService, JwtService, MockBlacklist, MongoDb, TokenBlacklist},
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
use std::sync::Arc;
use tower::util::ServiceExt;
use uuid::Uuid;

async fn setup_test_config() -> (Config, String) {
    dotenvy::dotenv().ok();
    let mut config = Config::from_env().expect("Failed to load environment variables for test");
    let db_name = format!("test_app_auth_middleware_{}", Uuid::new_v4());
    config.mongodb.database = db_name.clone();
    config.log_level = "error".to_string();
    (config, db_name)
}

async fn teardown_test_db(uri: &str, db_name: &str) {
    let client = mongodb::Client::with_uri_str(uri).await.unwrap();
    client.database(db_name).drop(None).await.unwrap();
}

#[tokio::test]
async fn test_app_auth_middleware() {
    // 1. Setup
    let (config, db_name) = setup_test_config().await;

    let db = MongoDb::connect(&config.mongodb.uri, &config.mongodb.database)
        .await
        .expect("Failed to connect to DB");
    db.initialize_indexes().await.unwrap();

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
        app_token_rate_limiter: ip_limiter.clone(),
        client_rate_limiter: create_client_rate_limiter(),
        ip_rate_limiter: ip_limiter,
    };

    // 2. Build App with Middleware
    let app = Router::new()
        .route("/app-protected", get(|| async { "app-protected" }))
        .layer(from_fn_with_state(state.clone(), app_auth_middleware))
        .with_state(state);

    // 3. Create Test Client
    let client_id = "test_client_id";
    let client_secret_hash = hash_password(&Password::new("any".to_string())).unwrap();
    let client = Client::new(
        client_id.to_string(),
        client_secret_hash.into_string(),
        "Test App".to_string(),
        ClientType::Service,
        100,
        vec![],
    );
    db.clients().insert_one(&client, None).await.unwrap();

    // 4. Test: Missing Header
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/app-protected")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // 5. Test: Valid Token via X-App-Token
    let token = jwt.generate_app_token(client_id, "Test App", vec![], 100).unwrap();
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/app-protected")
                .header("X-App-Token", &token)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    // 6. Test: Valid Token via Authorization Header (Bearer)
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/app-protected")
                .header("Authorization", format!("Bearer {}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    // 7. Test: Disabled Client
    db.clients()
        .update_one(
            mongodb::bson::doc! { "client_id": client_id },
            mongodb::bson::doc! { "$set": { "enabled": false } },
            None,
        )
        .await
        .unwrap();

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/app-protected")
                .header("X-App-Token", &token)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // 8. Test: Revoked Client (via Redis blacklist)
    // Re-enable first
    db.clients()
        .update_one(
            mongodb::bson::doc! { "client_id": client_id },
            mongodb::bson::doc! { "$set": { "enabled": true } },
            None,
        )
        .await
        .unwrap();

    redis
        .blacklist_token(&format!("client:{}", client_id), 3600)
        .await
        .unwrap();

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/app-protected")
                .header("X-App-Token", &token)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // Cleanup
    teardown_test_db(&config.mongodb.uri, &db_name).await;
}
