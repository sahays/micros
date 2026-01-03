use auth_service::{
    build_router,
    config::Config,
    middleware::{
        create_ip_rate_limiter, create_login_rate_limiter, create_password_reset_rate_limiter,
    },
    services::{EmailService, JwtService, MockBlacklist, MongoDb},
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
    let db_name = format!("test_auth_google_{}", Uuid::new_v4());
    config.mongodb.database = db_name.clone();
    config.log_level = "error".to_string();
    (config, db_name)
}

async fn teardown_test_db(uri: &str, db_name: &str) {
    let client = mongodb::Client::with_uri_str(uri).await.unwrap();
    client.database(db_name).drop(None).await.unwrap();
}

#[tokio::test]
async fn test_google_login_redirect() {
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
        jwt: jwt.clone(),
        redis,
        login_rate_limiter: login_limiter,
        password_reset_rate_limiter: reset_limiter,
        ip_rate_limiter: ip_limiter,
    };

    // 2. Build Router
    let app = build_router(state).await.expect("Failed to build router");

    // 3. Test GET /auth/google
    let response = app
        .oneshot(
            Request::builder()
                .uri("/auth/google")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // 4. Verify Redirect
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    let location = response.headers()["location"].to_str().unwrap();
    assert!(location.starts_with("https://accounts.google.com/o/oauth2/v2/auth"));
    assert!(location.contains("client_id="));
    assert!(location.contains("scope=openid%20email%20profile"));
    assert!(location.contains("code_challenge="));
    assert!(location.contains("state="));

    // 5. Verify Cookies
    let set_cookie = response.headers().get_all("set-cookie");
    let cookies: Vec<_> = set_cookie.iter().map(|c| c.to_str().unwrap()).collect();
    assert!(cookies.iter().any(|c| c.contains("oauth_state=")));
    assert!(cookies.iter().any(|c| c.contains("code_verifier=")));

    // Cleanup
    teardown_test_db(&config.mongodb.uri, &db_name).await;
}
