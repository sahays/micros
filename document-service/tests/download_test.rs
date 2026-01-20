mod common;

use common::{TestApp, TEST_APP_ID, TEST_ORG_ID, TEST_USER_ID};
use uuid::Uuid;

/// Helper to upload a document for testing
async fn upload_test_document(
    client: &mut service_core::grpc::DocumentClient,
    filename: &str,
    mime_type: &str,
    data: Vec<u8>,
) -> String {
    let response = client
        .upload_document(
            TEST_APP_ID,
            TEST_ORG_ID,
            TEST_USER_ID,
            filename.to_string(),
            mime_type.to_string(),
            data,
        )
        .await
        .expect("Failed to upload document");

    response.document.expect("Missing document").id
}

#[tokio::test]
async fn download_document_works() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    // Upload a document
    let test_data = b"Hello, World! Download test.".to_vec();
    let doc_id =
        upload_test_document(&mut client, "test.txt", "text/plain", test_data.clone()).await;

    // Download the document
    let (filename, content_type, downloaded_data) = client
        .download_document(TEST_APP_ID, TEST_ORG_ID, TEST_USER_ID, doc_id)
        .await
        .expect("Failed to download document");

    // Verify
    assert_eq!(filename, "test.txt");
    assert_eq!(content_type, "text/plain");
    assert_eq!(downloaded_data, test_data);

    app.cleanup().await;
}

#[tokio::test]
async fn download_nonexistent_document_returns_not_found() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    let fake_id = Uuid::new_v4().to_string();

    let result = client
        .download_document(TEST_APP_ID, TEST_ORG_ID, TEST_USER_ID, fake_id)
        .await;

    assert!(result.is_err());
    let status = result.unwrap_err();
    assert_eq!(status.code(), tonic::Code::NotFound);

    app.cleanup().await;
}

#[tokio::test]
async fn get_document_metadata_works() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    // Upload a document
    let test_data = vec![0u8; 1024];
    let doc_id =
        upload_test_document(&mut client, "metadata.txt", "text/plain", test_data.clone()).await;

    // Get metadata
    let response = client
        .get_document(TEST_APP_ID, TEST_ORG_ID, TEST_USER_ID, doc_id.clone())
        .await
        .expect("Failed to get document");

    let document = response.document.expect("Missing document");
    assert_eq!(document.id, doc_id);
    assert_eq!(document.original_name, "metadata.txt");
    assert_eq!(document.mime_type, "text/plain");
    assert_eq!(document.size, test_data.len() as i64);

    app.cleanup().await;
}

#[tokio::test]
async fn list_documents_works() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    // Upload multiple documents
    for i in 0..5 {
        upload_test_document(
            &mut client,
            &format!("doc{}.txt", i),
            "text/plain",
            vec![0; 100],
        )
        .await;
    }

    // List documents
    let response = client
        .list_documents(
            TEST_APP_ID,
            TEST_ORG_ID,
            TEST_USER_ID,
            None,
            None,
            Some(1),
            Some(10),
        )
        .await
        .expect("Failed to list documents");

    assert_eq!(response.documents.len(), 5);
    assert_eq!(response.total, 5);

    app.cleanup().await;
}

#[tokio::test]
async fn list_documents_with_pagination() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    // Upload 10 documents
    for i in 0..10 {
        upload_test_document(
            &mut client,
            &format!("page{}.txt", i),
            "text/plain",
            vec![0; 50],
        )
        .await;
    }

    // Get first page
    let page1 = client
        .list_documents(
            TEST_APP_ID,
            TEST_ORG_ID,
            TEST_USER_ID,
            None,
            None,
            Some(1),
            Some(5),
        )
        .await
        .expect("Failed to list page 1");

    assert_eq!(page1.documents.len(), 5);
    assert_eq!(page1.page, 1);
    assert_eq!(page1.page_size, 5);
    assert_eq!(page1.total, 10);
    assert_eq!(page1.total_pages, 2);

    // Get second page
    let page2 = client
        .list_documents(
            TEST_APP_ID,
            TEST_ORG_ID,
            TEST_USER_ID,
            None,
            None,
            Some(2),
            Some(5),
        )
        .await
        .expect("Failed to list page 2");

    assert_eq!(page2.documents.len(), 5);
    assert_eq!(page2.page, 2);

    app.cleanup().await;
}

#[tokio::test]
async fn delete_document_works() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    // Upload a document
    let doc_id =
        upload_test_document(&mut client, "to_delete.txt", "text/plain", vec![0; 100]).await;

    // Verify it exists
    let _ = client
        .get_document(TEST_APP_ID, TEST_ORG_ID, TEST_USER_ID, doc_id.clone())
        .await
        .expect("Document should exist");

    // Delete the document
    let response = client
        .delete_document(TEST_APP_ID, TEST_ORG_ID, TEST_USER_ID, doc_id.clone())
        .await
        .expect("Failed to delete document");

    assert!(response.success);

    // Verify it's gone
    let result = client
        .get_document(TEST_APP_ID, TEST_ORG_ID, TEST_USER_ID, doc_id)
        .await;

    assert!(result.is_err());
    assert_eq!(result.unwrap_err().code(), tonic::Code::NotFound);

    app.cleanup().await;
}

#[tokio::test]
async fn signed_url_generation_works() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    // Upload a document
    let doc_id = upload_test_document(&mut client, "signed.txt", "text/plain", vec![0; 100]).await;

    // Generate signed URL
    let response = client
        .generate_signed_url(TEST_APP_ID, TEST_ORG_ID, TEST_USER_ID, doc_id.clone(), 3600)
        .await
        .expect("Failed to generate signed URL");

    // Verify URL contains document ID and signature
    assert!(response.url.contains(&doc_id));
    assert!(response.url.contains("signature="));
    assert!(response.url.contains("expires="));
    assert!(response.expires_at.is_some());

    app.cleanup().await;
}
