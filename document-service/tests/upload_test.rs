mod common;

use common::{TestApp, TEST_APP_ID, TEST_ORG_ID, TEST_USER_ID};
use mongodb::bson::doc;
use service_core::grpc::DocumentStatusProto;

#[tokio::test]
async fn upload_document_works() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    // Upload a document via gRPC
    let test_data = b"Hello, World! This is test data.".to_vec();
    let response = client
        .upload_document(
            TEST_APP_ID,
            TEST_ORG_ID,
            TEST_USER_ID,
            "test.txt".to_string(),
            "text/plain".to_string(),
            test_data.clone(),
        )
        .await
        .expect("Failed to upload document");

    let document = response.document.expect("Missing document in response");

    // Verify response
    assert!(!document.id.is_empty());
    assert_eq!(document.original_name, "test.txt");
    assert_eq!(document.mime_type, "text/plain");
    assert_eq!(document.size, test_data.len() as i64);
    assert_eq!(document.status, DocumentStatusProto::Ready as i32);
    assert_eq!(document.owner_id, TEST_USER_ID);
    assert_eq!(document.app_id, TEST_APP_ID);
    assert_eq!(document.org_id, TEST_ORG_ID);

    // Verify document exists in database
    let stored_doc = app
        .db
        .documents()
        .find_one(doc! { "_id": &document.id }, None)
        .await
        .expect("DB query failed")
        .expect("Document not found in DB");

    assert_eq!(stored_doc.owner_id, TEST_USER_ID);
    assert_eq!(stored_doc.original_name, "test.txt");
    assert_eq!(stored_doc.size, test_data.len() as i64);

    app.cleanup().await;
}

#[tokio::test]
async fn upload_document_with_different_mime_types() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    // Test various mime types
    let test_cases = vec![
        ("image.png", "image/png"),
        ("document.pdf", "application/pdf"),
        ("data.json", "application/json"),
        ("video.mp4", "video/mp4"),
    ];

    for (filename, mime_type) in test_cases {
        let response = client
            .upload_document(
                TEST_APP_ID,
                TEST_ORG_ID,
                TEST_USER_ID,
                filename.to_string(),
                mime_type.to_string(),
                vec![0; 100],
            )
            .await
            .expect("Failed to upload document");

        let document = response.document.expect("Missing document");
        assert_eq!(document.original_name, filename);
        assert_eq!(document.mime_type, mime_type);
    }

    app.cleanup().await;
}

#[tokio::test]
async fn upload_document_requires_tenant_context() {
    let app = TestApp::spawn().await;

    // Try to connect and upload without proper tenant headers
    // The gRPC client requires tenant context in metadata
    // This test verifies the server rejects requests without proper metadata

    use service_core::grpc::proto::document::{
        document_service_client::DocumentServiceClient, upload_document_request::Data,
        UploadDocumentRequest, UploadMetadata,
    };
    use std::collections::HashMap;
    use tonic::transport::Channel;

    let channel = Channel::from_shared(app.grpc_address.clone())
        .unwrap()
        .connect()
        .await
        .expect("Failed to connect");

    let mut raw_client = DocumentServiceClient::new(channel);

    // Create upload request WITHOUT tenant metadata
    let metadata_msg = UploadDocumentRequest {
        data: Some(Data::Metadata(UploadMetadata {
            filename: "test.txt".to_string(),
            mime_type: "text/plain".to_string(),
            metadata: HashMap::new(),
        })),
    };

    let chunk_msg = UploadDocumentRequest {
        data: Some(Data::Chunk(vec![0; 100])),
    };

    let request = tonic::Request::new(futures::stream::iter(vec![metadata_msg, chunk_msg]));
    // Note: No tenant metadata added

    let result = raw_client.upload_document(request).await;

    // Should fail with unauthenticated
    assert!(result.is_err());
    let status = result.unwrap_err();
    assert_eq!(status.code(), tonic::Code::Unauthenticated);

    app.cleanup().await;
}

#[tokio::test]
async fn upload_large_document() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    // Upload a 5MB document
    let large_data = vec![0u8; 5 * 1024 * 1024];

    let response = client
        .upload_document(
            TEST_APP_ID,
            TEST_ORG_ID,
            TEST_USER_ID,
            "large_file.bin".to_string(),
            "application/octet-stream".to_string(),
            large_data.clone(),
        )
        .await
        .expect("Failed to upload large document");

    let document = response.document.expect("Missing document");
    assert_eq!(document.size, large_data.len() as i64);

    app.cleanup().await;
}
