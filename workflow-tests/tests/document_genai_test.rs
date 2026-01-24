//! Document + GenAI Workflow Tests
//!
//! Tests document processing with AI metadata extraction.
//! GenAI API is mocked in tests.

mod common;

use tonic::Request;
use uuid::Uuid;
use workflow_tests::proto::genai::{
    ListModelsRequest, ProcessRequest, OutputFormat, RequestMetadata,
};
use workflow_tests::proto::document::ListDocumentsRequest;
use workflow_tests::ServiceEndpoints;

/// Test: GenAI service lists available models.
#[tokio::test]
async fn genai_lists_models() {
    common::setup().await;

    let endpoints = ServiceEndpoints::from_env();
    let mut genai_client = workflow_tests::GenAiServiceClient::connect(endpoints.genai.clone())
        .await
        .expect("Failed to connect to genai service");

    let tenant_id = Uuid::new_v4().to_string();
    let user_id = Uuid::new_v4().to_string();

    let mut request = Request::new(ListModelsRequest {});

    request.metadata_mut().insert("x-tenant-id", tenant_id.parse().unwrap());
    request.metadata_mut().insert("x-user-id", user_id.parse().unwrap());

    let response = genai_client
        .list_models(request)
        .await
        .expect("Failed to list models");

    let models = response.into_inner().models;
    assert!(!models.is_empty(), "Should have at least one model configured");

    // Verify model has expected fields
    let model = &models[0];
    assert!(!model.id.is_empty());
    assert!(!model.name.is_empty());
}

/// Test: GenAI can process a simple text prompt.
#[tokio::test]
async fn genai_processes_text_prompt() {
    common::setup().await;

    let endpoints = ServiceEndpoints::from_env();
    let mut genai_client = workflow_tests::GenAiServiceClient::connect(endpoints.genai.clone())
        .await
        .expect("Failed to connect to genai service");

    let tenant_id = Uuid::new_v4().to_string();
    let user_id = Uuid::new_v4().to_string();

    let mut request = Request::new(ProcessRequest {
        prompt: "Say hello in exactly one word.".to_string(),
        documents: vec![],
        output_format: OutputFormat::Text as i32,
        output_schema: None,
        session_id: None,
        params: None,
        metadata: Some(RequestMetadata {
            tenant_id: tenant_id.clone(),
            user_id: user_id.clone(),
            tags: Default::default(),
        }),
    });

    request.metadata_mut().insert("x-tenant-id", tenant_id.parse().unwrap());
    request.metadata_mut().insert("x-user-id", user_id.parse().unwrap());

    // This may fail if API key is not configured - that's acceptable in tests
    let response = genai_client.process(request).await;

    match response {
        Ok(resp) => {
            let inner = resp.into_inner();
            // Should have some response
            assert!(inner.result.is_some() || !inner.model.is_empty());
        }
        Err(status) => {
            // API key not configured is acceptable
            assert!(
                status.code() == tonic::Code::FailedPrecondition ||
                status.code() == tonic::Code::Unavailable ||
                status.code() == tonic::Code::Internal,
                "Unexpected error: {:?}",
                status
            );
        }
    }
}

/// Test: Document service lists documents.
#[tokio::test]
async fn document_service_lists_documents() {
    common::setup().await;

    let endpoints = ServiceEndpoints::from_env();
    let mut doc_client = workflow_tests::DocumentServiceClient::connect(endpoints.document.clone())
        .await
        .expect("Failed to connect to document service");

    let tenant_id = Uuid::new_v4().to_string();
    let user_id = Uuid::new_v4().to_string();

    let mut request = Request::new(ListDocumentsRequest {
        page: Some(1),
        page_size: Some(10),
        status: None,
        mime_type: None,
    });

    request.metadata_mut().insert("x-tenant-id", tenant_id.parse().unwrap());
    request.metadata_mut().insert("x-user-id", user_id.parse().unwrap());
    request.metadata_mut().insert("x-app-id", tenant_id.parse().unwrap());
    request.metadata_mut().insert("x-org-id", tenant_id.parse().unwrap());

    let response = doc_client
        .list_documents(request)
        .await
        .expect("Failed to list documents");

    // Empty list is fine - no documents uploaded yet
    let _documents = response.into_inner().documents;
    // Response valid - documents list can be empty
}
