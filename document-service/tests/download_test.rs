use axum::http::StatusCode;
use document_service::config::DocumentConfig;
use document_service::dtos::DocumentResponse;
use document_service::startup::Application;
use reqwest::multipart;
use uuid::Uuid;

// Test constants for tenant context
const TEST_APP_ID: &str = "test-app-id";
const TEST_ORG_ID: &str = "test-org-id";
const TEST_USER_ID: &str = "test_user";

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

#[tokio::test]
async fn download_original_file_works() {
    // 1. Setup
    std::env::set_var("MONGODB_URI", "mongodb://localhost:27017");
    let mut config = DocumentConfig::load().expect("Failed to load configuration");
    config.common.port = 0;
    config.storage.local_path = format!("target/test-storage-{}", Uuid::new_v4());

    let db_name = format!("test_download_original_{}", Uuid::new_v4());
    config.mongodb.database = db_name.clone();

    let app = Application::build(config.clone())
        .await
        .expect("Failed to build application");
    let port = app.port();
    let db = app.db().clone();

    tokio::spawn(app.run_until_stopped());

    // 2. Upload a document
    let client = reqwest::Client::new();
    let test_data = b"Hello, World!".to_vec();
    let doc = upload_document(
        &client,
        port,
        TEST_USER_ID,
        "test.txt",
        "text/plain",
        test_data.clone(),
    )
    .await;

    // 3. Download the document
    let response = client
        .get(format!(
            "http://127.0.0.1:{}/documents/{}/content",
            port, doc.id
        ))
        .header("X-App-ID", TEST_APP_ID)
        .header("X-Org-ID", TEST_ORG_ID)
        .header("X-User-ID", TEST_USER_ID)
        .send()
        .await
        .expect("Failed to execute request");

    // 4. Assert
    assert_eq!(StatusCode::OK, response.status());
    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "text/plain"
    );

    let downloaded_data = response.bytes().await.expect("Failed to read response");
    assert_eq!(downloaded_data.to_vec(), test_data);

    // Cleanup
    let _ = db.client().database(&db_name).drop(None).await;
    let _ = tokio::fs::remove_dir_all(&config.storage.local_path).await;
}

#[tokio::test]
async fn download_nonexistent_document_returns_404() {
    // 1. Setup
    std::env::set_var("MONGODB_URI", "mongodb://localhost:27017");
    let mut config = DocumentConfig::load().expect("Failed to load configuration");
    config.common.port = 0;
    config.storage.local_path = format!("target/test-storage-{}", Uuid::new_v4());

    let db_name = format!("test_download_404_{}", Uuid::new_v4());
    config.mongodb.database = db_name.clone();

    let app = Application::build(config.clone())
        .await
        .expect("Failed to build application");
    let port = app.port();
    let db = app.db().clone();

    tokio::spawn(app.run_until_stopped());

    // 2. Try to download a non-existent document
    let client = reqwest::Client::new();
    let fake_id = Uuid::new_v4().to_string();

    let response = client
        .get(format!(
            "http://127.0.0.1:{}/documents/{}/content",
            port, fake_id
        ))
        .header("X-App-ID", TEST_APP_ID)
        .header("X-Org-ID", TEST_ORG_ID)
        .header("X-User-ID", TEST_USER_ID)
        .send()
        .await
        .expect("Failed to execute request");

    // 3. Assert
    assert_eq!(StatusCode::NOT_FOUND, response.status());

    // Cleanup
    let _ = db.client().database(&db_name).drop(None).await;
    let _ = tokio::fs::remove_dir_all(&config.storage.local_path).await;
}

#[tokio::test]
async fn download_without_user_id_or_signature_fails() {
    // 1. Setup
    std::env::set_var("MONGODB_URI", "mongodb://localhost:27017");
    let mut config = DocumentConfig::load().expect("Failed to load configuration");
    config.common.port = 0;
    config.storage.local_path = format!("target/test-storage-{}", Uuid::new_v4());

    let db_name = format!("test_download_unauth_{}", Uuid::new_v4());
    config.mongodb.database = db_name.clone();

    let app = Application::build(config.clone())
        .await
        .expect("Failed to build application");
    let port = app.port();
    let db = app.db().clone();

    tokio::spawn(app.run_until_stopped());

    // 2. Upload a document
    let client = reqwest::Client::new();
    let test_data = b"Hello, World!".to_vec();
    let doc = upload_document(
        &client,
        port,
        TEST_USER_ID,
        "test.txt",
        "text/plain",
        test_data.clone(),
    )
    .await;

    // 3. Try to download without tenant headers - should be unauthorized
    let response = client
        .get(format!(
            "http://127.0.0.1:{}/documents/{}/content",
            port, doc.id
        ))
        .send()
        .await
        .expect("Failed to execute request");

    // 4. Assert - should be unauthorized
    assert_eq!(StatusCode::UNAUTHORIZED, response.status());

    // Cleanup
    let _ = db.client().database(&db_name).drop(None).await;
    let _ = tokio::fs::remove_dir_all(&config.storage.local_path).await;
}

#[tokio::test]
async fn signed_url_works() {
    // 1. Setup
    std::env::set_var("MONGODB_URI", "mongodb://localhost:27017");
    let mut config = DocumentConfig::load().expect("Failed to load configuration");
    config.common.port = 0;
    config.storage.local_path = format!("target/test-storage-{}", Uuid::new_v4());

    let db_name = format!("test_signed_url_{}", Uuid::new_v4());
    config.mongodb.database = db_name.clone();

    let app = Application::build(config.clone())
        .await
        .expect("Failed to build application");
    let port = app.port();
    let db = app.db().clone();

    tokio::spawn(app.run_until_stopped());

    // 2. Upload a document
    let client = reqwest::Client::new();
    let test_data = b"Hello, World!".to_vec();
    let doc = upload_document(
        &client,
        port,
        TEST_USER_ID,
        "test.txt",
        "text/plain",
        test_data.clone(),
    )
    .await;

    // 3. Generate a signed URL
    let expires = chrono::Utc::now().timestamp() + 300; // 5 minutes from now
    let signature = service_core::utils::signature::generate_document_signature(
        &doc.id,
        expires,
        &config.signature.signing_secret,
    )
    .expect("Failed to generate signature");

    // 4. Download using signed URL (no X-User-ID header)
    let response = client
        .get(format!(
            "http://127.0.0.1:{}/documents/{}/content?signature={}&expires={}",
            port, doc.id, signature, expires
        ))
        .send()
        .await
        .expect("Failed to execute request");

    // 5. Assert
    assert_eq!(StatusCode::OK, response.status());
    let downloaded_data = response.bytes().await.expect("Failed to read response");
    assert_eq!(downloaded_data.to_vec(), test_data);

    // Cleanup
    let _ = db.client().database(&db_name).drop(None).await;
    let _ = tokio::fs::remove_dir_all(&config.storage.local_path).await;
}

#[tokio::test]
async fn expired_signed_url_fails() {
    // 1. Setup
    std::env::set_var("MONGODB_URI", "mongodb://localhost:27017");
    let mut config = DocumentConfig::load().expect("Failed to load configuration");
    config.common.port = 0;
    config.storage.local_path = format!("target/test-storage-{}", Uuid::new_v4());

    let db_name = format!("test_expired_url_{}", Uuid::new_v4());
    config.mongodb.database = db_name.clone();

    let app = Application::build(config.clone())
        .await
        .expect("Failed to build application");
    let port = app.port();
    let db = app.db().clone();

    tokio::spawn(app.run_until_stopped());

    // 2. Upload a document
    let client = reqwest::Client::new();
    let test_data = b"Hello, World!".to_vec();
    let doc = upload_document(
        &client,
        port,
        TEST_USER_ID,
        "test.txt",
        "text/plain",
        test_data.clone(),
    )
    .await;

    // 3. Generate a signed URL with past expiration
    let expires = chrono::Utc::now().timestamp() - 300; // 5 minutes ago
    let signature = service_core::utils::signature::generate_document_signature(
        &doc.id,
        expires,
        &config.signature.signing_secret,
    )
    .expect("Failed to generate signature");

    // 4. Try to download using expired signed URL
    let response = client
        .get(format!(
            "http://127.0.0.1:{}/documents/{}/content?signature={}&expires={}",
            port, doc.id, signature, expires
        ))
        .send()
        .await
        .expect("Failed to execute request");

    // 5. Assert - should be unauthorized
    assert_eq!(StatusCode::UNAUTHORIZED, response.status());

    // Cleanup
    let _ = db.client().database(&db_name).drop(None).await;
    let _ = tokio::fs::remove_dir_all(&config.storage.local_path).await;
}

#[tokio::test]
async fn invalid_signature_fails() {
    // 1. Setup
    std::env::set_var("MONGODB_URI", "mongodb://localhost:27017");
    let mut config = DocumentConfig::load().expect("Failed to load configuration");
    config.common.port = 0;
    config.storage.local_path = format!("target/test-storage-{}", Uuid::new_v4());

    let db_name = format!("test_invalid_sig_{}", Uuid::new_v4());
    config.mongodb.database = db_name.clone();

    let app = Application::build(config.clone())
        .await
        .expect("Failed to build application");
    let port = app.port();
    let db = app.db().clone();

    tokio::spawn(app.run_until_stopped());

    // 2. Upload a document
    let client = reqwest::Client::new();
    let test_data = b"Hello, World!".to_vec();
    let doc = upload_document(
        &client,
        port,
        TEST_USER_ID,
        "test.txt",
        "text/plain",
        test_data.clone(),
    )
    .await;

    // 3. Use an invalid signature
    let expires = chrono::Utc::now().timestamp() + 300;
    let invalid_signature = "invalid_signature_12345";

    // 4. Try to download using invalid signature
    let response = client
        .get(format!(
            "http://127.0.0.1:{}/documents/{}/content?signature={}&expires={}",
            port, doc.id, invalid_signature, expires
        ))
        .send()
        .await
        .expect("Failed to execute request");

    // 5. Assert - should be unauthorized
    assert_eq!(StatusCode::UNAUTHORIZED, response.status());

    // Cleanup
    let _ = db.client().database(&db_name).drop(None).await;
    let _ = tokio::fs::remove_dir_all(&config.storage.local_path).await;
}

// Note: The following tests for processed files would require actual ffmpeg/imagemagick
// installations and are more suitable for integration testing in a full environment.
// They are commented out but provided as templates for future testing.

/*
#[tokio::test]
async fn download_processed_image_works() {
    // This test requires imagemagick to be installed
    // 1. Setup test environment
    // 2. Upload an image file
    // 3. Trigger processing with webp conversion
    // 4. Wait for processing to complete
    // 5. Download the processed file
    // 6. Verify content-type is image/webp
    // 7. Verify file size is different (optimized)
}

#[tokio::test]
async fn download_compressed_video_works() {
    // This test requires ffmpeg to be installed
    // 1. Setup test environment
    // 2. Upload a video file
    // 3. Trigger processing with 720p resolution
    // 4. Wait for processing to complete (may take time)
    // 5. Download the compressed video
    // 6. Verify content-type is video/mp4
    // 7. Verify resolution metadata in status
}

#[tokio::test]
async fn download_chunked_video_returns_json() {
    // This test requires ffmpeg and a large video file >1GB after compression
    // 1. Setup test environment
    // 2. Upload a large video file
    // 3. Trigger processing with 720p resolution
    // 4. Wait for processing to complete
    // 5. Download request should return JSON with chunk metadata
    // 6. Verify JSON structure: type, chunks array, chunk_count
}

#[tokio::test]
async fn download_video_chunk_works() {
    // This test requires ffmpeg and a large video file
    // 1. Setup and process a large video (chunked)
    // 2. Get chunk metadata from download endpoint
    // 3. Download individual chunks by index
    // 4. Verify each chunk returns video/mp4 content
    // 5. Verify chunk sizes match metadata
}
*/
