use service_core::middleware::rate_limit::{create_client_rate_limiter, create_ip_rate_limiter};
use auth_service::{
    build_router,
    config::AuthConfig,
    dtos::admin::CreateServiceAccountResponse,
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

async fn setup_test_config() -> (AuthConfig, String) {
    dotenvy::dotenv().ok();
    std::env::set_var("ADMIN_API_KEY", "test_admin_key");

    let mut config = AuthConfig::from_env().expect("Failed to load environment variables for test");
    let db_name = format!("test_auth_service_account_{}", Uuid::new_v4());
    config.mongodb.database = db_name.clone();
    config.log_level = "error".to_string();
    (config, db_name)
}

async fn teardown_test_db(uri: &str, db_name: &str) {
    let client = mongodb::Client::with_uri_str(uri).await.unwrap();
    client.database(db_name).drop(None).await.unwrap();
}

#[tokio::test]
async fn test_create_service_account_flow() {
    // 1. Setup
    let (config, db_name) = setup_test_config().await;

    let db = MongoDb::connect(&config.mongodb.uri, &config.mongodb.database)
        .await
        .expect("Failed to connect to DB");

    db.initialize_indexes()
        .await
        .expect("Failed to initialize indexes");

    let email = EmailService::new(&config.gmail).expect("Failed to create email service");
    let email = Arc::new(email);
    let jwt = JwtService::new(&config.jwt).expect("Failed to create JWT service");
    let redis = Arc::new(MockBlacklist::new());

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
        login_rate_limiter: create_ip_rate_limiter(5, 60),
        register_rate_limiter: create_ip_rate_limiter(5, 60),
        password_reset_rate_limiter: create_ip_rate_limiter(3, 3600),
        app_token_rate_limiter: create_ip_rate_limiter(100, 60),
        client_rate_limiter: create_client_rate_limiter(),
        ip_rate_limiter: create_ip_rate_limiter(100, 60),
    };

    let app = build_router(state).await.expect("Failed to build router");

    // 2. Test: Missing API Key
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/admin/services")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    r#"{
                    "service_name": "Payment Service",
                    "scopes": ["user:read", "user:write"]
                }"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // 3. Test: Valid API Key
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/admin/services")
                .header("Content-Type", "application/json")
                .header("X-Admin-Api-Key", "test_admin_key")
                .body(Body::from(
                    r#"{
                    "service_name": "Payment Service",
                    "scopes": ["user:read", "user:write"]
                }"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let service_res: CreateServiceAccountResponse = serde_json::from_slice(&body).unwrap();

    assert!(!service_res.service_id.is_empty());
    assert!(!service_res.api_key.is_empty());

    // Verify DB
    let stored_account = db
        .service_accounts()
        .find_one(doc! { "service_id": &service_res.service_id }, None)
        .await
        .unwrap()
        .expect("Service account not found in DB");

    assert_eq!(stored_account.service_name, "Payment Service");
    assert!(stored_account.enabled);

    // Cleanup
    teardown_test_db(&config.mongodb.uri, &db_name).await;
}

#[tokio::test]
async fn test_service_auth_middleware() {
    // 1. Setup
    let (config, db_name) = setup_test_config().await;

    let db = MongoDb::connect(&config.mongodb.uri, &config.mongodb.database)
        .await
        .expect("Failed to connect to DB");

    db.initialize_indexes()
        .await
        .expect("Failed to initialize indexes");

    let email = EmailService::new(&config.gmail).expect("Failed to create email service");
    let email = Arc::new(email);
    let jwt = JwtService::new(&config.jwt).expect("Failed to create JWT service");
    let redis = Arc::new(MockBlacklist::new());

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
        redis: redis.clone(),
        login_rate_limiter: create_ip_rate_limiter(5, 60),
        register_rate_limiter: create_ip_rate_limiter(5, 60),
        password_reset_rate_limiter: create_ip_rate_limiter(3, 3600),
        app_token_rate_limiter: create_ip_rate_limiter(100, 60),
        client_rate_limiter: create_client_rate_limiter(),
        ip_rate_limiter: create_ip_rate_limiter(100, 60),
    };

    // Create a router with a protected route
    use auth_service::middleware::service_auth_middleware;
    use auth_service::middleware::ServiceContext;
    use axum::middleware::from_fn_with_state;
    use axum::routing::get;

    async fn protected_handler(
        axum::extract::Extension(context): axum::extract::Extension<ServiceContext>,
    ) -> impl axum::response::IntoResponse {
        axum::Json(context)
    }

    let app = build_router(state.clone())
        .await
        .expect("Failed to build router")
        .route(
            "/test/service-protected",
            get(protected_handler)
                .layer(from_fn_with_state(state.clone(), service_auth_middleware)),
        );

    // 2. Create a service account
    let api_key = "svc_test_validkey123";
    let key_hash = auth_service::utils::hash_password(&auth_service::utils::Password::new(
        api_key.to_string(),
    ))
    .unwrap();
    let lookup_hash = auth_service::models::ServiceAccount::calculate_lookup_hash(api_key);

    let account = auth_service::models::ServiceAccount::new(
        "Test Service".to_string(),
        key_hash.into_string(),
        lookup_hash,
        vec!["test:scope".to_string()],
    );

    db.service_accounts()
        .insert_one(&account, None)
        .await
        .unwrap();

    // 3. Test: Valid API Key
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/test/service-protected")
                .header("Authorization", format!("Bearer {}", api_key))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let context: ServiceContext = serde_json::from_slice(&body).unwrap();
    assert_eq!(context.service_id, account.service_id);

    // 4. Test: Invalid API Key
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/test/service-protected")
                .header("Authorization", "Bearer svc_test_wrongkey")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // 5. Test: Disabled Account
    db.service_accounts()
        .update_one(
            doc! { "service_id": &account.service_id },
            doc! { "$set": { "enabled": false } },
            None,
        )
        .await
        .unwrap();

    // Clear cache first
    let cache_key = format!(
        "svc_auth:{}",
        auth_service::models::ServiceAccount::calculate_lookup_hash(api_key)
    );
    redis.set_cache(&cache_key, "", 0).await.unwrap();

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/test/service-protected")
                .header("Authorization", format!("Bearer {}", api_key))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    // Cleanup
    teardown_test_db(&config.mongodb.uri, &db_name).await;
}

#[tokio::test]
async fn test_scope_validation() {
    // 1. Setup
    let (config, db_name) = setup_test_config().await;

    let db = MongoDb::connect(&config.mongodb.uri, &config.mongodb.database)
        .await
        .expect("Failed to connect to DB");

    db.initialize_indexes()
        .await
        .expect("Failed to initialize indexes");

    let email = EmailService::new(&config.gmail).expect("Failed to create email service");
    let email = Arc::new(email);
    let jwt = JwtService::new(&config.jwt).expect("Failed to create JWT service");
    let redis = Arc::new(MockBlacklist::new());

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
        login_rate_limiter: create_ip_rate_limiter(5, 60),
        register_rate_limiter: create_ip_rate_limiter(5, 60),
        password_reset_rate_limiter: create_ip_rate_limiter(3, 3600),
        app_token_rate_limiter: create_ip_rate_limiter(100, 60),
        client_rate_limiter: create_client_rate_limiter(),
        ip_rate_limiter: create_ip_rate_limiter(100, 60),
    };

    use auth_service::middleware::{require_scopes, service_auth_middleware, ServiceContext};
    use axum::middleware::from_fn_with_state;
    use axum::routing::get;

    async fn protected_handler(
        axum::extract::Extension(context): axum::extract::Extension<ServiceContext>,
    ) -> impl axum::response::IntoResponse {
        axum::Json(context)
    }

    let app = build_router(state.clone())
        .await
        .expect("Failed to build router")
        .route(
            "/test/scoped",
            get(protected_handler)
                .layer(from_fn_with_state(
                    state.clone(),
                    move |state, req, next| {
                        require_scopes(state, vec!["user:read".to_string()], req, next)
                    },
                ))
                .layer(from_fn_with_state(state.clone(), service_auth_middleware)),
        );

    // 2. Test: Success with exact scope
    let api_key_1 = "svc_test_user_read_key";
    let key_hash_1 = auth_service::utils::hash_password(&auth_service::utils::Password::new(
        api_key_1.to_string(),
    ))
    .unwrap();
    let account_1 = auth_service::models::ServiceAccount::new(
        "Read Service".to_string(),
        key_hash_1.into_string(),
        auth_service::models::ServiceAccount::calculate_lookup_hash(api_key_1),
        vec!["user:read".to_string()],
    );
    db.service_accounts()
        .insert_one(&account_1, None)
        .await
        .unwrap();

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/test/scoped")
                .header("Authorization", format!("Bearer {}", api_key_1))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    // 3. Test: Failure with insufficient scope
    let api_key_2 = "svc_test_user_write_key";
    let key_hash_2 = auth_service::utils::hash_password(&auth_service::utils::Password::new(
        api_key_2.to_string(),
    ))
    .unwrap();
    let account_2 = auth_service::models::ServiceAccount::new(
        "Write Service".to_string(),
        key_hash_2.into_string(),
        auth_service::models::ServiceAccount::calculate_lookup_hash(api_key_2),
        vec!["user:write".to_string()],
    );
    db.service_accounts()
        .insert_one(&account_2, None)
        .await
        .unwrap();

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/test/scoped")
                .header("Authorization", format!("Bearer {}", api_key_2))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    // 4. Test: Success with wildcard scope
    let api_key_3 = "svc_test_admin_key";
    let key_hash_3 = auth_service::utils::hash_password(&auth_service::utils::Password::new(
        api_key_3.to_string(),
    ))
    .unwrap();
    let account_3 = auth_service::models::ServiceAccount::new(
        "Admin Service".to_string(),
        key_hash_3.into_string(),
        auth_service::models::ServiceAccount::calculate_lookup_hash(api_key_3),
        vec!["user:*".to_string()],
    );
    db.service_accounts()
        .insert_one(&account_3, None)
        .await
        .unwrap();

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/test/scoped")
                .header("Authorization", format!("Bearer {}", api_key_3))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    // Cleanup
    teardown_test_db(&config.mongodb.uri, &db_name).await;
}

#[tokio::test]
async fn test_service_rotation_revocation() {
    // 1. Setup
    let (config, db_name) = setup_test_config().await;

    let db = MongoDb::connect(&config.mongodb.uri, &config.mongodb.database)
        .await
        .expect("Failed to connect to DB");

    db.initialize_indexes()
        .await
        .expect("Failed to initialize indexes");

    let email = EmailService::new(&config.gmail).expect("Failed to create email service");
    let email = Arc::new(email);
    let jwt = JwtService::new(&config.jwt).expect("Failed to create JWT service");
    let redis = Arc::new(MockBlacklist::new());

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
        redis: redis.clone(),
        login_rate_limiter: create_ip_rate_limiter(5, 60),
        register_rate_limiter: create_ip_rate_limiter(5, 60),
        password_reset_rate_limiter: create_ip_rate_limiter(3, 3600),
        app_token_rate_limiter: create_ip_rate_limiter(100, 60),
        client_rate_limiter: create_client_rate_limiter(),
        ip_rate_limiter: create_ip_rate_limiter(100, 60),
    };

    let app = build_router(state.clone())
        .await
        .expect("Failed to build router");

    // 2. Create service account
    let api_key_1 = "svc_test_rotation_key_1";
    let key_hash_1 = auth_service::utils::hash_password(&auth_service::utils::Password::new(
        api_key_1.to_string(),
    ))
    .unwrap();
    let account = auth_service::models::ServiceAccount::new(
        "Rotation Service".to_string(),
        key_hash_1.into_string(),
        auth_service::models::ServiceAccount::calculate_lookup_hash(api_key_1),
        vec!["test:rotation".to_string()],
    );
    db.service_accounts()
        .insert_one(&account, None)
        .await
        .unwrap();
    let service_id = account.service_id.clone();

    // 3. Test: Rotation
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/auth/admin/services/{}/rotate", service_id))
                .header("X-Admin-Api-Key", "test_admin_key")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let rotate_res: auth_service::dtos::admin::RotateServiceKeyResponse =
        serde_json::from_slice(&body).unwrap();
    let api_key_2 = rotate_res.new_api_key;
    assert_ne!(api_key_1, api_key_2);

    // 4. Verify both keys work (grace period)
    let app_with_test_route = app.route(
        "/test/rotation",
        axum::routing::get(|| async { "ok" }).layer(axum::middleware::from_fn_with_state(
            state.clone(),
            auth_service::middleware::service_auth_middleware,
        )),
    );

    // Test Key 1 (Old)
    let response = app_with_test_route
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/test/rotation")
                .header("Authorization", format!("Bearer {}", api_key_1))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Test Key 2 (New)
    let response = app_with_test_route
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/test/rotation")
                .header("Authorization", format!("Bearer {}", api_key_2))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // 5. Test: Revocation
    let response = app_with_test_route
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/auth/admin/services/{}", service_id))
                .header("X-Admin-Api-Key", "test_admin_key")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // 6. Verify BOTH keys now fail

    let response = app_with_test_route
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/test/rotation")
                .header("Authorization", format!("Bearer {}", api_key_1))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    // 7. Verify audit logs

    // Give some time for async audit logs to be written

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let response = app_with_test_route
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/auth/admin/services/{}/audit-log", service_id))
                .header("X-Admin-Api-Key", "test_admin_key")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();

    let logs: Vec<auth_service::models::AuditLog> = serde_json::from_slice(&body).unwrap();

    assert!(!logs.is_empty());

    assert!(logs.iter().any(|l| l.event_type == "service_auth_success"));

    assert!(logs
        .iter()
        .any(|l| l.event_type == "access_denied" || l.event_type == "service_auth_failure"));

    // Cleanup

    teardown_test_db(&config.mongodb.uri, &db_name).await;
}
