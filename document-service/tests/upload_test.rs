use axum::http::StatusCode;
use document_service::config::DocumentConfig;
use document_service::startup::Application;
use mongodb::bson::doc;
use reqwest::multipart;
use uuid::Uuid;

#[tokio::test]
async fn upload_document_works() {
    // 1. Setup
    std::env::set_var("MONGODB_URI", "mongodb://localhost:27017");
    let mut config = DocumentConfig::load().expect("Failed to load configuration");
    config.common.port = 0; // Random port
    config.storage.local_path = format!("target/test-storage-{}", Uuid::new_v4());

    // Unique DB for test
    let db_name = format!("test_document_upload_{}", Uuid::new_v4());
    config.mongodb.database = db_name.clone();

    let app = Application::build(config.clone())
        .await
        .expect("Failed to build application");
    let port = app.port();
    let db = app.db().clone();

    tokio::spawn(app.run_until_stopped());

    // 2. Request
    let client = reqwest::Client::new();
    let form = multipart::Form::new().part(
        "file",
        multipart::Part::bytes(vec![0; 100])
            .file_name("test.txt")
            .mime_str("text/plain")
            .unwrap(),
    );

    let response = client
        .post(format!("http://127.0.0.1:{}/documents", port))
        .header("X-User-ID", "test_user_123") // User context from BFF
        .multipart(form)
        .send()
        .await
        .expect("Failed to execute request.");

    // 3. Assert Response
    assert_eq!(StatusCode::CREATED, response.status());

    let body: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body["original_name"], "test.txt");
    assert_eq!(body["mime_type"], "text/plain");
    assert_eq!(body["size"], 100);
    assert_eq!(body["status"], "ready"); // No automatic processing - must be triggered manually

    let doc_id = body["id"].as_str().unwrap();

    // 4. Verify DB
    let stored_doc = db
        .documents()
        .find_one(doc! { "_id": doc_id }, None)
        .await
        .unwrap()
        .expect("Document not found in DB");

    assert_eq!(stored_doc.owner_id, "test_user_123");
    assert_eq!(stored_doc.original_name, "test.txt");
    assert_eq!(stored_doc.size, 100);

    // 5. Verify Storage
    let storage_path =
        std::path::Path::new(&config.storage.local_path).join(&stored_doc.storage_key);
    assert!(storage_path.exists());

    // Cleanup
    let _ = db.client().database(&db_name).drop(None).await;
    let _ = tokio::fs::remove_dir_all(&config.storage.local_path).await;
}
