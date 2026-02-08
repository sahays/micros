use workflow_tests::helpers::genai;
use workflow_tests::proto::genai::{
    CreateSessionRequest, DeleteSessionRequest, GetSessionRequest, ListModelsRequest,
    OutputFormat, ProcessRequest, RequestMetadata,
};
use workflow_tests::GenAiServiceClient;

async fn connect() -> GenAiServiceClient<tonic::transport::Channel> {
    let endpoint = genai::grpc_endpoint();
    GenAiServiceClient::connect(endpoint)
        .await
        .expect("Failed to connect to genai gRPC server")
}

#[tokio::test]
async fn list_models_returns_configured_models() {
    let mut client = connect().await;

    let response = client
        .list_models(ListModelsRequest {})
        .await
        .expect("Failed to list models");

    let models = response.into_inner().models;
    assert_eq!(models.len(), 3);

    let model_ids: Vec<&str> = models.iter().map(|m| m.id.as_str()).collect();
    assert!(model_ids.contains(&"gemini-2.0-flash")); // Text model
    assert!(model_ids.contains(&"veo-2")); // Video model
}

#[tokio::test]
async fn process_returns_text_response() {
    if genai::skip_api_tests() {
        eprintln!("Skipping test: SKIP_API_TESTS is set or GOOGLE_API_KEY is not valid");
        return;
    }

    let mut client = connect().await;

    let response = client
        .process(ProcessRequest {
            prompt: "Say 'Hello' in exactly one word.".to_string(),
            documents: vec![],
            output_format: OutputFormat::Text as i32,
            output_schema: None,
            session_id: None,
            params: None,
            metadata: Some(RequestMetadata {
                tenant_id: "test-tenant".to_string(),
                user_id: "test-user".to_string(),
                tags: Default::default(),
            }),
        })
        .await
        .expect("Failed to process request");

    let response = response.into_inner();

    assert!(!response.request_id.is_empty());
    assert_eq!(response.output_format, OutputFormat::Text as i32);
    assert!(!response.model.is_empty());

    assert!(response.usage.is_some());
    let usage = response.usage.unwrap();
    assert!(usage.input_tokens > 0);
    assert!(usage.output_tokens > 0);

    match response.result {
        Some(workflow_tests::proto::genai::process_response::Result::Text(text)) => {
            assert!(!text.is_empty());
        }
        _ => panic!("Expected text response"),
    }
}

#[tokio::test]
async fn process_returns_error_for_invalid_api_key() {
    // Only run this test when we're using a test API key (invalid)
    if !genai::skip_api_tests() {
        eprintln!("Skipping test: valid GOOGLE_API_KEY is set");
        return;
    }

    let mut client = connect().await;

    let result = client
        .process(ProcessRequest {
            prompt: "Hello, world!".to_string(),
            documents: vec![],
            output_format: OutputFormat::Text as i32,
            output_schema: None,
            session_id: None,
            params: None,
            metadata: Some(RequestMetadata {
                tenant_id: "test-tenant".to_string(),
                user_id: "test-user".to_string(),
                tags: Default::default(),
            }),
        })
        .await;

    assert!(result.is_err());
    let status = result.unwrap_err();
    assert_eq!(status.code(), tonic::Code::Internal);
    assert!(
        status.message().contains("API key") || status.message().contains("Provider API error")
    );
}

#[tokio::test]
async fn process_rejects_empty_prompt() {
    let mut client = connect().await;

    let result = client
        .process(ProcessRequest {
            prompt: "".to_string(),
            documents: vec![],
            output_format: OutputFormat::Text as i32,
            output_schema: None,
            session_id: None,
            params: None,
            metadata: Some(RequestMetadata {
                tenant_id: "test-tenant".to_string(),
                user_id: "test-user".to_string(),
                tags: Default::default(),
            }),
        })
        .await;

    assert!(result.is_err());
    let status = result.unwrap_err();
    assert_eq!(status.code(), tonic::Code::InvalidArgument);
}

#[tokio::test]
async fn process_requires_schema_for_structured_json() {
    let mut client = connect().await;

    let result = client
        .process(ProcessRequest {
            prompt: "Extract data".to_string(),
            documents: vec![],
            output_format: OutputFormat::StructuredJson as i32,
            output_schema: None,
            session_id: None,
            params: None,
            metadata: Some(RequestMetadata {
                tenant_id: "test-tenant".to_string(),
                user_id: "test-user".to_string(),
                tags: Default::default(),
            }),
        })
        .await;

    assert!(result.is_err());
    let status = result.unwrap_err();
    assert_eq!(status.code(), tonic::Code::InvalidArgument);
    assert!(status.message().contains("output_schema"));
}

#[tokio::test]
async fn process_rejects_invalid_json_schema() {
    let mut client = connect().await;

    let result = client
        .process(ProcessRequest {
            prompt: "Extract data".to_string(),
            documents: vec![],
            output_format: OutputFormat::StructuredJson as i32,
            output_schema: Some("not valid json".to_string()),
            session_id: None,
            params: None,
            metadata: Some(RequestMetadata {
                tenant_id: "test-tenant".to_string(),
                user_id: "test-user".to_string(),
                tags: Default::default(),
            }),
        })
        .await;

    assert!(result.is_err());
    let status = result.unwrap_err();
    assert_eq!(status.code(), tonic::Code::InvalidArgument);
    assert!(status.message().contains("valid JSON"));
}

#[tokio::test]
async fn session_lifecycle() {
    let mut client = connect().await;

    // Create a session
    let create_response = client
        .create_session(CreateSessionRequest {
            title: Some("Test Session".to_string()),
            system_prompt: Some("You are a helpful assistant.".to_string()),
            documents: vec![],
            metadata: Some(RequestMetadata {
                tenant_id: "test-tenant".to_string(),
                user_id: "test-user".to_string(),
                tags: Default::default(),
            }),
        })
        .await
        .expect("Failed to create session");

    let session = create_response
        .into_inner()
        .session
        .expect("Session should be returned");
    assert!(!session.id.is_empty());
    assert_eq!(session.title, Some("Test Session".to_string()));
    assert_eq!(session.message_count, 0);

    let session_id = session.id.clone();

    // Get the session
    let get_response = client
        .get_session(GetSessionRequest {
            session_id: session_id.clone(),
            include_messages: true,
        })
        .await
        .expect("Failed to get session");

    let retrieved_session = get_response
        .into_inner()
        .session
        .expect("Session should be returned");
    assert_eq!(retrieved_session.id, session_id);
    assert_eq!(retrieved_session.title, Some("Test Session".to_string()));

    // Delete the session
    let delete_response = client
        .delete_session(DeleteSessionRequest {
            session_id: session_id.clone(),
        })
        .await
        .expect("Failed to delete session");

    assert!(delete_response.into_inner().success);

    // Verify session is deleted
    let get_result = client
        .get_session(GetSessionRequest {
            session_id: session_id.clone(),
            include_messages: false,
        })
        .await;

    assert!(get_result.is_err());
    assert_eq!(get_result.unwrap_err().code(), tonic::Code::NotFound);
}

#[tokio::test]
async fn create_session_requires_metadata() {
    let mut client = connect().await;

    let result = client
        .create_session(CreateSessionRequest {
            title: Some("Test Session".to_string()),
            system_prompt: None,
            documents: vec![],
            metadata: None,
        })
        .await;

    assert!(result.is_err());
    let status = result.unwrap_err();
    assert_eq!(status.code(), tonic::Code::InvalidArgument);
    assert!(status.message().contains("metadata"));
}

#[tokio::test]
async fn get_session_requires_session_id() {
    let mut client = connect().await;

    let result = client
        .get_session(GetSessionRequest {
            session_id: "".to_string(),
            include_messages: false,
        })
        .await;

    assert!(result.is_err());
    let status = result.unwrap_err();
    assert_eq!(status.code(), tonic::Code::InvalidArgument);
}
