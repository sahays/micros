mod common;

use common::{TestApp, TEST_APP_ID, TEST_ORG_ID, TEST_USER_ID};
use service_core::grpc::DocumentStatusProto;
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
async fn manual_processing_trigger_works() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    // Upload a document
    let doc_id = upload_test_document(&mut client, "test.txt", "text/plain", vec![0; 100]).await;

    // Verify it starts in Ready state (no automatic processing)
    let get_response = client
        .get_document(TEST_APP_ID, TEST_ORG_ID, TEST_USER_ID, doc_id.clone())
        .await
        .expect("Failed to get document");

    let document = get_response.document.expect("Missing document");
    assert_eq!(document.status, DocumentStatusProto::Ready as i32);

    // Trigger processing with default options
    let process_response = client
        .process_document(TEST_APP_ID, TEST_ORG_ID, TEST_USER_ID, doc_id.clone(), None)
        .await
        .expect("Failed to trigger processing");

    assert!(process_response.queued);
    assert_eq!(
        process_response.status,
        DocumentStatusProto::Processing as i32
    );

    // Check status - should be Processing
    let status_response = client
        .get_processing_status(TEST_APP_ID, TEST_ORG_ID, TEST_USER_ID, doc_id.clone())
        .await
        .expect("Failed to get processing status");

    assert_eq!(status_response.document_id, doc_id);
    assert_eq!(
        status_response.status,
        DocumentStatusProto::Processing as i32
    );

    app.cleanup().await;
}

#[tokio::test]
async fn processing_with_custom_options_works() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    // Upload an image document
    let doc_id = upload_test_document(&mut client, "image.jpg", "image/jpeg", vec![0; 200]).await;

    // Trigger processing with custom image options
    use service_core::grpc::proto::document::{ImageOptions, ProcessingOptions};

    let options = ProcessingOptions {
        processors: vec![],
        pdf_options: None,
        image_options: Some(ImageOptions {
            format: "webp".to_string(),
            quality: 90,
        }),
        video_options: None,
    };

    let process_response = client
        .process_document(
            TEST_APP_ID,
            TEST_ORG_ID,
            TEST_USER_ID,
            doc_id.clone(),
            Some(options),
        )
        .await
        .expect("Failed to trigger processing");

    assert!(process_response.queued);

    app.cleanup().await;
}

#[tokio::test]
async fn cannot_process_already_processing_document() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    // Upload a document
    let doc_id = upload_test_document(&mut client, "test.txt", "text/plain", vec![0; 100]).await;

    // Trigger processing (first time - should succeed)
    let first_result = client
        .process_document(TEST_APP_ID, TEST_ORG_ID, TEST_USER_ID, doc_id.clone(), None)
        .await;

    assert!(first_result.is_ok());

    // Try to trigger processing again immediately (should fail)
    let second_result = client
        .process_document(TEST_APP_ID, TEST_ORG_ID, TEST_USER_ID, doc_id.clone(), None)
        .await;

    assert!(second_result.is_err());
    let status = second_result.unwrap_err();
    assert_eq!(status.code(), tonic::Code::FailedPrecondition);

    app.cleanup().await;
}

#[tokio::test]
async fn status_endpoint_returns_correct_information() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    // Upload a document
    let doc_id = upload_test_document(&mut client, "status.txt", "text/plain", vec![0; 75]).await;

    // Get processing status (should be Ready, 0 attempts)
    let status_response = client
        .get_processing_status(TEST_APP_ID, TEST_ORG_ID, TEST_USER_ID, doc_id.clone())
        .await
        .expect("Failed to get processing status");

    assert_eq!(status_response.document_id, doc_id);
    assert_eq!(status_response.status, DocumentStatusProto::Ready as i32);
    assert_eq!(status_response.processing_attempts, 0);
    assert!(status_response.metadata.is_none());
    assert!(status_response.error_message.is_none());

    app.cleanup().await;
}

#[tokio::test]
async fn processing_nonexistent_document_returns_not_found() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    let fake_id = Uuid::new_v4().to_string();

    // Try to get status of non-existent document
    let status_result = client
        .get_processing_status(TEST_APP_ID, TEST_ORG_ID, TEST_USER_ID, fake_id.clone())
        .await;

    assert!(status_result.is_err());
    assert_eq!(status_result.unwrap_err().code(), tonic::Code::NotFound);

    // Try to trigger processing on non-existent document
    let process_result = client
        .process_document(TEST_APP_ID, TEST_ORG_ID, TEST_USER_ID, fake_id, None)
        .await;

    assert!(process_result.is_err());
    assert_eq!(process_result.unwrap_err().code(), tonic::Code::NotFound);

    app.cleanup().await;
}

#[tokio::test]
async fn any_authenticated_caller_can_process_document() {
    // Test that document-service trusts the BFF to handle authorization
    // The service should process any valid document ID without ownership checks

    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    let user_id_owner = "user_owner";
    let user_id_service = "service_caller";

    // Upload document as user_owner
    let response = client
        .upload_document(
            TEST_APP_ID,
            TEST_ORG_ID,
            user_id_owner,
            "shared.txt".to_string(),
            "text/plain".to_string(),
            vec![0; 50],
        )
        .await
        .expect("Failed to upload");

    let doc_id = response.document.expect("Missing document").id;

    // Any authenticated caller can trigger processing
    let process_result = client
        .process_document(
            TEST_APP_ID,
            TEST_ORG_ID,
            user_id_service,
            doc_id.clone(),
            None,
        )
        .await;

    assert!(process_result.is_ok()); // Should succeed - trusts caller

    // Any authenticated caller can get status
    let status_result = client
        .get_processing_status(TEST_APP_ID, TEST_ORG_ID, user_id_service, doc_id)
        .await;

    assert!(status_result.is_ok()); // Should succeed - trusts caller

    app.cleanup().await;
}
