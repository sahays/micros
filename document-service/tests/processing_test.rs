use axum::http::StatusCode;
use document_service::config::DocumentConfig;
use document_service::dtos::{DocumentResponse, ProcessingOptions, ProcessingStatusResponse};
use document_service::models::DocumentStatus;
use document_service::startup::Application;
use reqwest::multipart;
use std::time::Duration;
use uuid::Uuid;

// Test constants for tenant context
const TEST_APP_ID: &str = "test-app-id";
const TEST_ORG_ID: &str = "test-org-id";

/// Helper function to upload a document with tenant headers
async fn upload_document(
    client: &reqwest::Client,
    port: u16,
    user_id: &str,
    filename: &str,
    mime_type: &str,
    data: Vec<u8>,
) -> DocumentResponse {
    let form = multipart::Form::new().part(
        "file",
        multipart::Part::bytes(data)
            .file_name(filename.to_string())
            .mime_str(mime_type)
            .unwrap(),
    );

    let response = client
        .post(format!("http://127.0.0.1:{}/documents", port))
        .header("X-App-ID", TEST_APP_ID)
        .header("X-Org-ID", TEST_ORG_ID)
        .header("X-User-ID", user_id)
        .multipart(form)
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(StatusCode::CREATED, response.status());
    response.json().await.expect("Failed to parse JSON")
}

/// Helper function to trigger document processing with tenant headers
async fn trigger_processing(
    client: &reqwest::Client,
    port: u16,
    user_id: &str,
    document_id: &str,
    options: ProcessingOptions,
) -> StatusCode {
    let response = client
        .post(format!(
            "http://127.0.0.1:{}/documents/{}/process",
            port, document_id
        ))
        .header("X-App-ID", TEST_APP_ID)
        .header("X-Org-ID", TEST_ORG_ID)
        .header("X-User-ID", user_id)
        .json(&options)
        .send()
        .await
        .expect("Failed to execute request");

    response.status()
}

/// Helper function to get document status with tenant headers
async fn get_document_status(
    client: &reqwest::Client,
    port: u16,
    user_id: &str,
    document_id: &str,
) -> (StatusCode, Option<ProcessingStatusResponse>) {
    let response = client
        .get(format!(
            "http://127.0.0.1:{}/documents/{}/status",
            port, document_id
        ))
        .header("X-App-ID", TEST_APP_ID)
        .header("X-Org-ID", TEST_ORG_ID)
        .header("X-User-ID", user_id)
        .send()
        .await
        .expect("Failed to execute request");

    let status = response.status();
    let body = if status.is_success() {
        Some(response.json().await.expect("Failed to parse JSON"))
    } else {
        None
    };

    (status, body)
}

/// Helper to wait for processing to complete with tenant headers
async fn wait_for_processing(
    client: &reqwest::Client,
    port: u16,
    user_id: &str,
    document_id: &str,
    timeout: Duration,
) -> ProcessingStatusResponse {
    let start = std::time::Instant::now();

    loop {
        let (status, response) = get_document_status(client, port, user_id, document_id).await;
        assert_eq!(StatusCode::OK, status);

        let status_response = response.unwrap();

        // Check if processing is complete (either ready or failed)
        if !matches!(status_response.status, DocumentStatus::Processing) {
            return status_response;
        }

        if start.elapsed() > timeout {
            panic!("Processing timed out after {:?}", timeout);
        }

        tokio::time::sleep(Duration::from_millis(500)).await;
    }
}

#[tokio::test]
async fn manual_processing_trigger_works() {
    // 1. Setup
    std::env::set_var("MONGODB_URI", "mongodb://localhost:27017");
    let mut config = DocumentConfig::load().expect("Failed to load configuration");
    config.common.port = 0; // Random port
    config.storage.local_path = format!("target/test-storage-{}", Uuid::new_v4());

    let db_name = format!("test_manual_processing_{}", Uuid::new_v4());
    config.mongodb.database = db_name.clone();

    let app = Application::build(config.clone())
        .await
        .expect("Failed to build application");
    let port = app.port();
    let db = app.db().clone();

    tokio::spawn(app.run_until_stopped());

    let client = reqwest::Client::new();
    let user_id = "test_user_123";

    // 2. Upload a document
    let doc = upload_document(
        &client,
        port,
        user_id,
        "test.txt",
        "text/plain",
        vec![0; 100],
    )
    .await;

    // Document should be in Ready state (no automatic processing)
    assert_eq!(doc.status, DocumentStatus::Ready);

    // 3. Trigger processing with default options
    let options = ProcessingOptions::default();
    let status = trigger_processing(&client, port, user_id, &doc.id, options).await;
    assert_eq!(StatusCode::ACCEPTED, status);

    // 4. Check status immediately - should be Processing
    let (status, response) = get_document_status(&client, port, user_id, &doc.id).await;
    assert_eq!(StatusCode::OK, status);
    let status_response = response.unwrap();
    assert_eq!(status_response.status, DocumentStatus::Processing);

    // Cleanup
    let _ = db.client().database(&db_name).drop(None).await;
    let _ = tokio::fs::remove_dir_all(&config.storage.local_path).await;
}

#[tokio::test]
async fn processing_with_custom_options_works() {
    // 1. Setup
    std::env::set_var("MONGODB_URI", "mongodb://localhost:27017");
    let mut config = DocumentConfig::load().expect("Failed to load configuration");
    config.common.port = 0;
    config.storage.local_path = format!("target/test-storage-{}", Uuid::new_v4());

    let db_name = format!("test_custom_options_{}", Uuid::new_v4());
    config.mongodb.database = db_name.clone();

    let app = Application::build(config.clone())
        .await
        .expect("Failed to build application");
    let port = app.port();
    let db = app.db().clone();

    tokio::spawn(app.run_until_stopped());

    let client = reqwest::Client::new();
    let user_id = "test_user_456";

    // 2. Upload document
    let doc = upload_document(
        &client,
        port,
        user_id,
        "image.jpg",
        "image/jpeg",
        vec![0; 200],
    )
    .await;

    // 3. Trigger processing with custom image options
    let options = ProcessingOptions {
        processors: None,
        pdf_options: None,
        image_options: Some(document_service::dtos::ImageOptions {
            format: "webp".to_string(),
            quality: 90,
        }),
        video_options: None,
    };

    let status = trigger_processing(&client, port, user_id, &doc.id, options).await;
    assert_eq!(StatusCode::ACCEPTED, status);

    // Cleanup
    let _ = db.client().database(&db_name).drop(None).await;
    let _ = tokio::fs::remove_dir_all(&config.storage.local_path).await;
}

#[tokio::test]
async fn any_authenticated_caller_can_process_document() {
    // Test that document-service trusts the BFF to handle authorization
    // The service should process any valid document ID without ownership checks

    // 1. Setup
    std::env::set_var("MONGODB_URI", "mongodb://localhost:27017");
    let mut config = DocumentConfig::load().expect("Failed to load configuration");
    config.common.port = 0;
    config.storage.local_path = format!("target/test-storage-{}", Uuid::new_v4());

    let db_name = format!("test_bff_trust_{}", Uuid::new_v4());
    config.mongodb.database = db_name.clone();

    let app = Application::build(config.clone())
        .await
        .expect("Failed to build application");
    let port = app.port();
    let db = app.db().clone();

    tokio::spawn(app.run_until_stopped());

    let client = reqwest::Client::new();
    let user_id_1 = "user_owner";
    let user_id_2 = "service_caller"; // Could be BFF or another service

    // 2. Upload document as user_1
    let doc = upload_document(
        &client,
        port,
        user_id_1,
        "shared.txt",
        "text/plain",
        vec![0; 50],
    )
    .await;

    // 3. Any authenticated caller can trigger processing (BFF's job to check ownership)
    let options = ProcessingOptions::default();
    let status = trigger_processing(&client, port, user_id_2, &doc.id, options).await;
    assert_eq!(StatusCode::ACCEPTED, status); // Should succeed - trusts caller

    // 4. Any authenticated caller can get status
    let (status, _) = get_document_status(&client, port, user_id_2, &doc.id).await;
    assert_eq!(StatusCode::OK, status); // Should succeed - trusts caller

    // Cleanup
    let _ = db.client().database(&db_name).drop(None).await;
    let _ = tokio::fs::remove_dir_all(&config.storage.local_path).await;
}

#[tokio::test]
async fn cannot_process_already_processing_document() {
    // 1. Setup
    std::env::set_var("MONGODB_URI", "mongodb://localhost:27017");
    let mut config = DocumentConfig::load().expect("Failed to load configuration");
    config.common.port = 0;
    config.storage.local_path = format!("target/test-storage-{}", Uuid::new_v4());

    let db_name = format!("test_duplicate_processing_{}", Uuid::new_v4());
    config.mongodb.database = db_name.clone();

    let app = Application::build(config.clone())
        .await
        .expect("Failed to build application");
    let port = app.port();
    let db = app.db().clone();

    tokio::spawn(app.run_until_stopped());

    let client = reqwest::Client::new();
    let user_id = "test_user_789";

    // 2. Upload document
    let doc = upload_document(
        &client,
        port,
        user_id,
        "test.txt",
        "text/plain",
        vec![0; 100],
    )
    .await;

    // 3. Trigger processing (first time - should succeed)
    let options = ProcessingOptions::default();
    let status = trigger_processing(&client, port, user_id, &doc.id, options.clone()).await;
    assert_eq!(StatusCode::ACCEPTED, status);

    // 4. Try to trigger processing again immediately (should fail)
    let status = trigger_processing(&client, port, user_id, &doc.id, options).await;
    assert_eq!(StatusCode::BAD_REQUEST, status);

    // Cleanup
    let _ = db.client().database(&db_name).drop(None).await;
    let _ = tokio::fs::remove_dir_all(&config.storage.local_path).await;
}

#[tokio::test]
async fn status_endpoint_returns_correct_information() {
    // 1. Setup
    std::env::set_var("MONGODB_URI", "mongodb://localhost:27017");
    let mut config = DocumentConfig::load().expect("Failed to load configuration");
    config.common.port = 0;
    config.storage.local_path = format!("target/test-storage-{}", Uuid::new_v4());

    let db_name = format!("test_status_endpoint_{}", Uuid::new_v4());
    config.mongodb.database = db_name.clone();

    let app = Application::build(config.clone())
        .await
        .expect("Failed to build application");
    let port = app.port();
    let db = app.db().clone();

    tokio::spawn(app.run_until_stopped());

    let client = reqwest::Client::new();
    let user_id = "test_user_status";

    // 2. Upload document
    let doc = upload_document(
        &client,
        port,
        user_id,
        "status.txt",
        "text/plain",
        vec![0; 75],
    )
    .await;

    // 3. Get status (should be Ready)
    let (status, response) = get_document_status(&client, port, user_id, &doc.id).await;
    assert_eq!(StatusCode::OK, status);

    let status_response = response.unwrap();
    assert_eq!(status_response.document_id, doc.id);
    assert_eq!(status_response.status, DocumentStatus::Ready);
    assert_eq!(status_response.processing_attempts, 0);
    assert!(status_response.processing_metadata.is_none());
    assert!(status_response.error_message.is_none());

    // Cleanup
    let _ = db.client().database(&db_name).drop(None).await;
    let _ = tokio::fs::remove_dir_all(&config.storage.local_path).await;
}

#[tokio::test]
async fn document_not_found_returns_404() {
    // 1. Setup
    std::env::set_var("MONGODB_URI", "mongodb://localhost:27017");
    let mut config = DocumentConfig::load().expect("Failed to load configuration");
    config.common.port = 0;
    config.storage.local_path = format!("target/test-storage-{}", Uuid::new_v4());

    let db_name = format!("test_not_found_{}", Uuid::new_v4());
    config.mongodb.database = db_name.clone();

    let app = Application::build(config.clone())
        .await
        .expect("Failed to build application");
    let port = app.port();
    let db = app.db().clone();

    tokio::spawn(app.run_until_stopped());

    let client = reqwest::Client::new();
    let user_id = "test_user_404";

    // 2. Try to get status of non-existent document
    let fake_id = Uuid::new_v4().to_string();
    let (status, _) = get_document_status(&client, port, user_id, &fake_id).await;
    assert_eq!(StatusCode::NOT_FOUND, status);

    // 3. Try to trigger processing on non-existent document
    let options = ProcessingOptions::default();
    let status = trigger_processing(&client, port, user_id, &fake_id, options).await;
    assert_eq!(StatusCode::NOT_FOUND, status);

    // Cleanup
    let _ = db.client().database(&db_name).drop(None).await;
    let _ = tokio::fs::remove_dir_all(&config.storage.local_path).await;
}
